//! Journal de diagnostic et surveillance du thread de l'interface.
//!
//! Le journal (`screencapture.log`, 2 Mo maximum, l'ancien est gardé en `.old.log`) note le
//! démarrage, les écrans, les actions et les erreurs. Un thread de surveillance vérifie toutes
//! les 20 s que l'interface répond encore ; sinon il écrit la dernière étape atteinte, ce qui
//! désigne l'appel bloquant.

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const MAX_SIZE: u64 = 2 * 1024 * 1024;

static FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);
static START: OnceLock<Instant> = OnceLock::new();
/// Dernier passage dans `update`, en ms depuis le démarrage.
static HEARTBEAT: AtomicU64 = AtomicU64::new(0);
/// Dernière étape franchie par le thread de l'interface.
static STEP: Mutex<(&str, u64)> = Mutex::new(("démarrage", 0));

fn now_ms() -> u64 {
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// Dossier des journaux : %LOCALAPPDATA%\ScreenCapture\logs ou ~/Library/Logs/ScreenCapture.
pub fn log_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join("Library").join("Logs").join("ScreenCapture");
    #[cfg(not(target_os = "macos"))]
    if let Some(data) = crate::update::portable_data_dir() {
        return data.join("logs");
    }
    #[cfg(not(target_os = "macos"))]
    let base = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir).join("ScreenCapture").join("logs");
    base
}

pub fn log_path() -> PathBuf {
    log_dir().join("screencapture.log")
}

/// Démarre l'horloge du diagnostic et enregistre les plantages (panic) dans le journal quand
/// celui-ci est activé, y compris ceux des threads de travail.
pub fn init() {
    let _ = START.set(Instant::now());
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        log(&format!(
            "PANIC dans le thread « {} » : {info}\n{}",
            thread.name().unwrap_or("?"),
            std::backtrace::Backtrace::force_capture()
        ));
        default_hook(info);
    }));
}

/// Active ou désactive l'écriture du journal (réglage « Journal de diagnostic », désactivé par
/// défaut). Sans journal, la surveillance continue de débloquer l'interface, sans rien écrire.
pub fn set_enabled(enabled: bool) {
    let mut file = FILE.lock().unwrap_or_else(|e| e.into_inner());
    if enabled == file.is_some() {
        return;
    }
    if !enabled {
        *file = None;
        return;
    }
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = log_path();
    if std::fs::metadata(&path).map(|m| m.len() > MAX_SIZE).unwrap_or(false) {
        let _ = std::fs::rename(&path, dir.join("screencapture.old.log"));
    }
    *file = std::fs::OpenOptions::new().create(true).append(true).open(&path).ok();
    drop(file);
    log(&format!(
        "===== ScreenCapture {} : journal activé ({} {}) =====",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
}

/// Ajoute une ligne horodatée au journal (sans effet si le journal est désactivé).
pub fn log(msg: &str) {
    let line = format!("{} [{}] {msg}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), std::thread::current().name().unwrap_or("?"));
    if let Ok(mut guard) = FILE.lock() {
        if let Some(f) = guard.as_mut() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }
}

/// Note l'étape en cours du thread de l'interface (sans écrire dans le journal).
pub fn step(name: &'static str) {
    if let Ok(mut s) = STEP.lock() {
        *s = (name, now_ms());
    }
}

/// À appeler au début de chaque image : l'interface répond.
pub fn heartbeat() {
    HEARTBEAT.store(now_ms(), Ordering::SeqCst);
    step("update");
}

fn last_step() -> (&'static str, u64) {
    STEP.lock().map(|s| *s).unwrap_or(("?", 0))
}

/// Demande une image et attend qu'elle commence ; `false` si l'interface ne répond pas à temps.
/// Le réveil passe par un thread jetable pour ne jamais bloquer l'appelant.
pub fn ui_responds(ctx: &eframe::egui::Context, timeout: Duration) -> bool {
    static PROBING: AtomicBool = AtomicBool::new(false);
    let t0 = now_ms();
    if !PROBING.swap(true, Ordering::SeqCst) {
        let ctx = ctx.clone();
        let _ = std::thread::Builder::new().name("diag-reveil".into()).spawn(move || {
            ctx.request_repaint();
            PROBING.store(false, Ordering::SeqCst);
        });
    }
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if HEARTBEAT.load(Ordering::SeqCst) >= t0 {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Une boîte de dialogue fichier (modale) est ouverte : l'interface attend l'utilisateur.
static IN_DIALOG: AtomicBool = AtomicBool::new(false);

/// Exécute `f`, qui ouvre une boîte de dialogue modale : la surveillance ne la prend pas pour un blocage.
pub fn in_dialog<T>(f: impl FnOnce() -> T) -> T {
    step("boîte de dialogue fichier");
    IN_DIALOG.store(true, Ordering::SeqCst);
    let r = f();
    IN_DIALOG.store(false, Ordering::SeqCst);
    r
}

/// Surveillance : changements d'écrans et blocages de l'interface. `unblock` tente de relancer
/// une interface figée (fenêtre réduite, menu resté ouvert).
pub fn start_watchdog(ctx: &eframe::egui::Context, unblock: impl Fn() + Send + 'static) {
    let ctx = ctx.clone();
    let _ = std::thread::Builder::new().name("diag-surveillance".into()).spawn(move || {
        let mut screens = crate::capture::monitors();
        log(&format!("écrans : {screens:?}"));
        let mut blocked_since: Option<u64> = None;
        loop {
            std::thread::sleep(Duration::from_secs(20));
            let now = crate::capture::monitors();
            if now != screens {
                log(&format!("écrans modifiés : {screens:?} -> {now:?}"));
                screens = now;
            }
            let mut responds = ui_responds(&ctx, Duration::from_secs(10));
            if !responds && IN_DIALOG.load(Ordering::SeqCst) {
                continue;
            }
            if !responds {
                let (name, _) = last_step();
                unblock();
                // Fenêtre restaurée ou menu fermé : l'interface repart-elle ?
                responds = ui_responds(&ctx, Duration::from_secs(5));
                if responds && blocked_since.is_none() {
                    log(&format!("interface débloquée automatiquement (étape « {name} »)"));
                    continue;
                }
            }
            if responds {
                if let Some(since) = blocked_since.take() {
                    log(&format!("interface débloquée après {} s", (now_ms() - since) / 1000));
                }
            } else if blocked_since.is_none() {
                let (name, at) = last_step();
                blocked_since = Some(at);
                log(&format!(
                    "INTERFACE BLOQUÉE : aucune image depuis {} s ; dernière étape « {name} » commencée il y a {} s",
                    (now_ms() - HEARTBEAT.load(Ordering::SeqCst)) / 1000,
                    (now_ms() - at) / 1000
                ));
            }
        }
    });
}

/// Résumé pour le journal quand une autre instance demande l'affichage.
pub fn describe_block() -> String {
    let (name, at) = last_step();
    format!("dernière étape « {name} » commencée il y a {} s", (now_ms() - at) / 1000)
}
