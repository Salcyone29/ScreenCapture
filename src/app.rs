//! Application : machine à états (espace de travail / éditeur / surcouche / enregistrement),
//! icône de notification, raccourcis globaux et page des paramètres.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, vec2, Align2, Color32, CornerRadius, FontId, Margin, Vec2};
use egui_material_icons::icons::*;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use image::RgbaImage;
use raw_window_handle::HasWindowHandle;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::capture::{self, Rect};
use crate::editor::{Editor, EditorAction};
use crate::overlay::{Overlay, OverlayResult, Purpose};
use crate::platform::{self, Window};
use crate::recorder::{self, Recorder};
use crate::settings::{Accent, AfterCapture, ImageFormat, Settings, ThemeChoice, VideoFormat};
use crate::theme::{self, palette, ButtonKind};
use crate::workspace::{pretty_hotkey, Workspace, WsAction};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CaptureKind {
    Region,
    Fullscreen,
    Window,
    LastRegion,
    Delayed,
    Video,
}

pub enum AppEvent {
    Hotkey(u32),
    Shortcut(CaptureKind),
    Menu(String),
    TrayOpen,
    Captured { kind: CaptureKind, image: RgbaImage, origin: Rect, target: Option<Rect> },
    CaptureFailed(String),
    Ocr(Result<String, String>),
    VideoDone(Result<PathBuf, String>),
    AudioDevices(Vec<String>),
    /// Commande reçue d'une autre instance (`ScreenCapture.exe --cmd …`, tests de validation).
    Command(String),
    /// Échap pendant une capture.
    CancelCapture,
    /// Nettoyage de conservation terminé (nombre, octets, erreurs, lancé à la main).
    RetentionDone { n: usize, bytes: u64, errors: Vec<String>, manual: bool },
}

/// Contenu de la fenêtre principale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Workspace,
    Editor,
}

/// Miniature affichée en bas à droite après une capture (option « Espace de travail »).
struct CapturePopup {
    image: RgbaImage,
    path: Option<PathBuf>,
    texture: Option<egui::TextureHandle>,
    since: Instant,
    /// Rectangle de la fenêtre (pixels physiques), calculé à l'ouverture.
    rect: Rect,
    /// Message bref affiché à la place du titre (« Copié »).
    notice: Option<(String, Instant)>,
}

/// Fenêtre flottante egui (fenêtre native distincte de la fenêtre principale) : sélection,
/// miniature ou barre d'enregistrement. Elle est créée invisible, placée au pixel près, puis
/// rendue visible une fois dessinée à sa taille : la fenêtre principale n'est jamais transformée.
#[derive(Default)]
struct Floating {
    win: Option<Window>,
    frames: u32,
}

impl Floating {
    /// À appeler à chaque image, depuis la fenêtre flottante.
    fn sync(&mut self, title: &str, r: Rect, activate: bool, exclude_from_capture: bool, round: bool) {
        self.frames += 1;
        let Some(w) = self.win else {
            if let Some(w) = Window::find(title) {
                w.cloak(true);
                w.round_corners(round);
                w.place_floating(r, activate, exclude_from_capture);
                crate::diag::log(&format!("fenêtre « {title} » créée en {r:?}"));
                self.win = Some(w);
                self.frames = 0;
            }
            return;
        };
        // Un changement d'échelle (écran en 125 %…) peut faire redimensionner la fenêtre par le système.
        if w.rect() != Some(r) {
            w.place_floating(r, false, exclude_from_capture);
        }
        if self.frames == 2 {
            w.cloak(false);
        }
    }
}

/// Taille minimale de l'espace de travail, en points (indépendante de l'échelle de l'écran).
pub const MIN_SIZE: (f32, f32) = (900.0, 600.0);

/// Taille de l'espace de travail en points, protégée contre un défaut de winit : en glissant la
/// fenêtre vers un écran d'une autre échelle (100 % → 125 %), winit la rétrécit à chaque passage
/// de la limite entre les écrans (×0,8, sans tenir compte de la taille minimale). On mémorise la
/// taille voulue et on la rétablit après chaque changement d'échelle.
#[derive(Default)]
struct SizeKeeper {
    /// Taille voulue (points), relevée quand l'échelle est stable.
    wanted: Option<egui::Vec2>,
    scale: Option<f32>,
    /// Dernier changement d'échelle : la taille n'est plus relevée pendant un moment.
    changed_at: Option<Instant>,
}

impl SizeKeeper {
    fn update(&mut self, ctx: &egui::Context, win: Window) {
        let (scale, inner, minimized, maximized, fullscreen) = ctx.input(|i| {
            let v = i.viewport();
            (v.native_pixels_per_point, v.inner_rect.map(|r| r.size()), v.minimized, v.maximized, v.fullscreen)
        });
        let (Some(scale), Some(inner)) = (scale, inner) else { return };
        if minimized == Some(true) || maximized == Some(true) || fullscreen == Some(true) {
            return;
        }
        let min = egui::vec2(MIN_SIZE.0, MIN_SIZE.1);
        if self.scale.is_some_and(|s| (s - scale).abs() > 0.01) {
            self.changed_at = Some(Instant::now());
            crate::diag::log(&format!("échelle de la fenêtre {:.2} → {scale:.2}, taille {inner:?}", self.scale.unwrap_or(0.0)));
        }
        self.scale = Some(scale);
        // Pendant que l'utilisateur tient la fenêtre, Windows la déplace : la redimensionner à ce
        // moment la ferait décrocher du pointeur. La taille est rétablie au relâchement.
        if win.is_moving() {
            if self.changed_at.is_some() {
                self.changed_at = Some(Instant::now());
            }
            ctx.request_repaint_after(Duration::from_millis(100));
            return;
        }
        let settling = self.changed_at.is_some_and(|t| t.elapsed() < Duration::from_millis(1500));
        let target = match self.wanted {
            Some(w) if settling => w,
            _ => inner.max(min),
        };
        if (inner - target).length() > 2.0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(target));
            ctx.request_repaint_after(Duration::from_millis(50));
            return;
        }
        if settling {
            ctx.request_repaint_after(Duration::from_millis(100));
        } else if self.wanted != Some(inner) {
            self.wanted = Some(inner);
            platform::set_wanted_size((inner.x, inner.y));
        }
    }
}

const TITLE_OVERLAY: &str = "ScreenCapture — Sélection";
const TITLE_POPUP: &str = "ScreenCapture — Miniature";
const TITLE_RECBAR: &str = "ScreenCapture — Enregistrement";

/// Fenêtre flottante sans bordure ni bouton dans la barre des tâches, créée invisible.
fn floating_builder(title: &str, r: Rect) -> egui::ViewportBuilder {
    let s = platform::scale_at(r.x + r.w / 2, r.y + r.h / 2);
    egui::ViewportBuilder::default()
        .with_title(title)
        .with_decorations(false)
        .with_taskbar(false)
        .with_always_on_top()
        .with_resizable(false)
        .with_visible(false)
        .with_position([r.x as f32 / s, r.y as f32 / s])
        .with_inner_size([r.w as f32 / s, r.h as f32 / s])
}

const POPUP_DELAY: Duration = Duration::from_secs(6);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Settings,
    About,
}

struct Hotkeys {
    manager: Option<GlobalHotKeyManager>,
    registered: Vec<(HotKey, CaptureKind)>,
    status: Vec<(CaptureKind, bool)>,
    /// Raccourcis utilisant Impr. écran, servis par le hook clavier (masque de modificateurs).
    printscreen: Arc<Mutex<Vec<(u8, CaptureKind)>>>,
    override_snip: Arc<std::sync::atomic::AtomicBool>,
    hook_ok: bool,
}

impl Hotkeys {
    fn apply(&mut self, s: &crate::settings::Hotkeys) {
        let Some(m) = &self.manager else { return };
        for (h, _) in self.registered.drain(..) {
            let _ = m.unregister(h);
        }
        self.status.clear();
        let mut prt = Vec::new();
        for (kind, spec) in [
            (CaptureKind::Region, &s.region),
            (CaptureKind::Fullscreen, &s.fullscreen),
            (CaptureKind::Window, &s.window),
            (CaptureKind::LastRegion, &s.last_region),
            (CaptureKind::Video, &s.video),
        ] {
            if spec.trim().is_empty() {
                self.status.push((kind, false));
                continue;
            }
            let ok = match spec.trim().parse::<HotKey>() {
                Ok(h) if h.key == Code::PrintScreen && self.hook_ok => {
                    // Le hook capte la touche quand une autre application a le focus ;
                    // RegisterHotKey prend le relais quand c'est notre fenêtre.
                    let mut mask = 0;
                    for (m, bit) in [
                        (Modifiers::CONTROL, platform::MOD_CTRL),
                        (Modifiers::SHIFT, platform::MOD_SHIFT),
                        (Modifiers::ALT, platform::MOD_ALT),
                        (Modifiers::SUPER, platform::MOD_WIN),
                    ] {
                        if h.mods.contains(m) {
                            mask |= bit;
                        }
                    }
                    if prt.iter().any(|(m, _)| *m == mask) {
                        false
                    } else {
                        prt.push((mask, kind));
                        if m.register(h).is_ok() {
                            self.registered.push((h, kind));
                        }
                        true
                    }
                }
                Ok(h) => {
                    let ok = m.register(h).is_ok();
                    if ok {
                        self.registered.push((h, kind));
                    }
                    ok
                }
                Err(_) => false,
            };
            self.status.push((kind, ok));
        }
        if let Ok(mut p) = self.printscreen.lock() {
            *p = prt;
        }
        self.override_snip.store(s.override_snip && self.hook_ok, std::sync::atomic::Ordering::SeqCst);
    }

    fn kind_of(&self, id: u32) -> Option<CaptureKind> {
        self.registered.iter().find(|(h, _)| h.id() == id).map(|(_, k)| *k)
    }
}

pub struct App {
    /// Fenêtre principale : espace de travail ou éditeur. Jamais déplacée ni redimensionnée par
    /// l'application, seulement masquée ou affichée.
    win: Window,
    mode: Mode,
    /// Fenêtre principale affichée (éventuellement réduite dans la barre des tâches).
    shown: bool,
    /// La fenêtre principale était à l'écran au début de la capture : la réafficher à la fin.
    restore_after_capture: bool,
    /// Nombre d'images dessinées depuis le lancement.
    frame: u64,
    size_keeper: SizeKeeper,
    overlay_win: Floating,
    popup_win: Floating,
    recbar_win: Floating,
    /// Rectangle de la barre d'enregistrement (pixels physiques).
    recbar_rect: Rect,
    page: Page,
    settings: Settings,
    workspace: Workspace,
    editor: Option<Editor>,
    overlay: Option<Overlay>,
    recorder: Option<Recorder>,
    popup: Option<CapturePopup>,
    finishing_video: bool,
    tx: Sender<AppEvent>,
    rx: Receiver<AppEvent>,
    hotkeys: Hotkeys,
    _tray: Option<TrayIcon>,
    menu_ids: HashMap<String, &'static str>,
    snackbar: Option<(String, Instant)>,
    ocr_text: Option<String>,
    busy: bool,
    /// Début de la capture en cours, pour ne jamais rester bloqué en « occupé ».
    busy_since: Instant,
    /// Capture annulée (Échap) avant d'avoir été prise : ignorer l'image qui arrivera.
    capture_cancelled: bool,
    /// Prochain nettoyage automatique (au démarrage, puis toutes les heures).
    retention_due: Instant,
    retention_running: bool,
    /// Aperçu de la carte « Conservation » : (clé des réglages, calculé à, plan).
    retention_preview: Option<(String, Instant, crate::retention::Plan)>,
    /// Réglages de conservation avant une modification en attente de confirmation.
    retention_confirm: Option<(u32, u32, u64)>,
    quit: bool,
    applied_theme: Option<(ThemeChoice, Accent, bool)>,
    hotkey_edit: crate::settings::Hotkeys,
    audio_devices: Vec<String>,
    mic_refresh: Option<Instant>,
    /// Pastille du nuancier en cours de modification dans les Paramètres.
    palette_sel: Option<usize>,
    palette_hex: (usize, String),
    /// Ctrl+V déjà traité tant que la touche reste enfoncée.
    paste_down: bool,
    screen_check: Instant,
    park_on_start: bool,
    title: String,
    last_shortcut: Option<(CaptureKind, Instant)>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, settings: Settings, start_hidden: bool) -> Self {
        let ctx = cc.egui_ctx.clone();
        theme::install_fonts(&ctx);
        let hwnd = cc.window_handle().map(|h| platform::native_handle(h.as_raw())).unwrap_or(0);
        let (tx, rx) = channel();

        // Raccourcis globaux.
        let manager = GlobalHotKeyManager::new().ok();
        {
            let tx = tx.clone();
            let ctx = ctx.clone();
            GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
                crate::debug_log(&format!("hotkey {} {:?}", e.id, e.state));
                if e.state == HotKeyState::Pressed {
                    let _ = tx.send(AppEvent::Hotkey(e.id));
                    ctx.request_repaint();
                }
            }));
        }
        let mut hotkeys = Hotkeys {
            manager,
            registered: Vec::new(),
            status: Vec::new(),
            printscreen: Arc::new(Mutex::new(Vec::new())),
            override_snip: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            hook_ok: false,
        };
        {
            let prt = hotkeys.printscreen.clone();
            let snip = hotkeys.override_snip.clone();
            // Le hook clavier doit répondre en quelques millisecondes, sinon Windows le retire :
            // il ne fait que poster l'évènement, un thread relais réveille l'interface.
            let tx = relay(tx.clone(), ctx.clone());
            hotkeys.hook_ok = platform::install_printscreen_hook(Box::new(move |m| {
                if m == platform::KEY_ESCAPE {
                    let _ = tx.send(AppEvent::CancelCapture);
                    return true;
                }
                let kind = if m & platform::KEY_SNIP != 0 {
                    snip.load(std::sync::atomic::Ordering::SeqCst).then_some(CaptureKind::Region)
                } else {
                    prt.lock().ok().and_then(|v| v.iter().find(|(mask, _)| *mask == m & 0x0F).map(|(_, k)| *k))
                };
                match kind {
                    Some(k) => {
                        if m & platform::NO_FIRE == 0 {
                            crate::debug_log(&format!("raccourci (hook) -> {k:?}"));
                            let _ = tx.send(AppEvent::Shortcut(k));
                        }
                        true
                    }
                    None => false,
                }
            }));
        }
        hotkeys.apply(&settings.hotkeys);
        crate::diag::log(&format!("hook Impr. écran : {}", if hotkeys.hook_ok { "installé" } else { "indisponible" }));
        // Interface figée par un menu de notification resté ouvert : le fermer.
        crate::diag::start_watchdog(&ctx, || {
            platform::cancel_modal_loops();
        });
        {
            // Nouvelle instance lancée (menu Démarrer…) : afficher la fenêtre. Si l'interface
            // ne répond plus, quitter pour que la nouvelle instance prenne le relais.
            let tx = tx.clone();
            let ctx = ctx.clone();
            platform::activation_listener(move |cmd: Option<String>| {
                crate::diag::log(&format!("demande d'une nouvelle instance : {}", cmd.as_deref().unwrap_or("afficher")));
                let _ = tx.send(match cmd {
                    Some(c) => AppEvent::Command(c),
                    None => AppEvent::TrayOpen,
                });
                if !crate::diag::ui_responds(&ctx, Duration::from_secs(4)) {
                    crate::diag::log(&format!("interface bloquée ({}) : arrêt pour relance", crate::diag::describe_block()));
                    std::process::exit(3);
                }
            });
        }

        // Icône de la zone de notification.
        let mut menu_ids = HashMap::new();
        let tray = build_tray(&mut menu_ids, &settings).ok();
        {
            let tx = tx.clone();
            let ctx = ctx.clone();
            // Ces gestionnaires tournent hors de la boucle egui : si la fenêtre est restée sur un
            // écran débranché, eframe peut ne plus rafraîchir l'interface ; on la ramène d'abord.
            let win = Window(hwnd);
            let quit_id = menu_ids.iter().find(|(_, v)| **v == "quit").map(|(k, _)| k.clone());
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                crate::diag::step("menu de notification");
                crate::diag::log(&format!("menu de notification : élément {}", e.id.0));
                win.rescue();
                if Some(&e.id.0) == quit_id.as_ref() {
                    // Filet de sécurité : quitter même si l'interface ne traite plus les évènements.
                    std::thread::spawn(|| {
                        std::thread::sleep(Duration::from_secs(4));
                        std::process::exit(0);
                    });
                }
                let _ = tx.send(AppEvent::Menu(e.id.0.clone()));
                ctx.request_repaint();
                crate::diag::step("menu de notification traité");
            }));
        }
        {
            let tx = tx.clone();
            let ctx = ctx.clone();
            let win = Window(hwnd);
            TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
                crate::diag::step("clic sur l'icône de notification");
                win.rescue();
                let open = matches!(
                    e,
                    TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }
                        | TrayIconEvent::DoubleClick { button: MouseButton::Left, .. }
                );
                if open {
                    let _ = tx.send(AppEvent::TrayOpen);
                    ctx.request_repaint();
                }
            }));
        }

        let hotkey_edit = settings.hotkeys.clone();
        platform::fix_dpi_changes(Window(hwnd));
        // Démarrage dans la zone de notification : masquer la fenêtre dès sa création (pas de flash) ;
        // le retrait du bouton de la barre des tâches attend qu'eframe ait fini de l'afficher.
        if start_hidden {
            Window(hwnd).cloak(true);
        }
        Self {
            win: Window(hwnd),
            mode: Mode::Workspace,
            shown: !start_hidden,
            restore_after_capture: false,
            frame: 0,
            size_keeper: SizeKeeper::default(),
            overlay_win: Floating::default(),
            popup_win: Floating::default(),
            recbar_win: Floating::default(),
            recbar_rect: Rect::new(0, 0, 1, 1),
            page: Page::Home,
            workspace: Workspace::new(&ctx),
            settings,
            editor: None,
            overlay: None,
            recorder: None,
            popup: None,
            finishing_video: false,
            tx,
            rx,
            hotkeys,
            _tray: tray,
            menu_ids,
            snackbar: None,
            ocr_text: None,
            busy: false,
            busy_since: Instant::now(),
            capture_cancelled: false,
            retention_due: Instant::now() + Duration::from_secs(15),
            retention_running: false,
            retention_preview: None,
            retention_confirm: None,
            quit: false,
            applied_theme: None,
            hotkey_edit,
            audio_devices: Vec::new(),
            mic_refresh: None,
            palette_sel: None,
            palette_hex: (usize::MAX, String::new()),
            paste_down: false,
            screen_check: Instant::now(),
            park_on_start: start_hidden,
            title: String::new(),
            last_shortcut: None,
        }
    }

    fn toast(&mut self, msg: impl Into<String>) {
        self.snackbar = Some((msg.into(), Instant::now()));
    }

    // ------------------------------------------------------------------ fenêtres

    /// Affiche la fenêtre principale (espace de travail ou éditeur) telle qu'elle était.
    fn show_root(&mut self) {
        crate::diag::step("affichage de la fenêtre principale");
        self.win.show();
        self.shown = true;
        crate::diag::log(&format!("fenêtre principale affichée : {}", self.win.describe()));
    }

    /// Masque la fenêtre principale (zone de notification) sans la déplacer ni la redimensionner.
    fn hide_root(&mut self) {
        crate::diag::step("masquage de la fenêtre principale");
        self.win.hide();
        self.shown = false;
        crate::diag::log(&format!("fenêtre principale masquée : {}", self.win.describe()));
    }

    /// Fin d'une capture : la fenêtre principale retrouve l'état qu'elle avait avant.
    fn end_capture(&mut self) {
        if std::mem::take(&mut self.restore_after_capture) {
            self.show_root();
        }
    }

    /// État complet, pour le journal (commande `--cmd state` des tests de validation).
    fn describe_state(&self) -> String {
        format!(
            "état : mode={:?} sélectionnées={} affichée={} restaurer={} occupé={} sélection={} miniature={} vidéo={} | principale {} | sélection {:?} | miniature {:?} | barre {:?}",
            self.mode,
            self.workspace.selected_count(),
            self.shown,
            self.restore_after_capture,
            self.busy,
            self.overlay.is_some(),
            self.popup.is_some(),
            self.recorder.is_some(),
            self.win.describe(),
            self.overlay_win.win.map(|w| format!("{} bandeau={} zone={:?}", w.describe(), self.overlay.as_ref().map(|o| o.help_position).unwrap_or("-"), self.overlay.as_ref().and_then(|o| o.current))),
            self.popup_win.win.map(|w| w.describe()),
            self.recbar_win.win.map(|w| w.describe()),
        )
    }

    // ------------------------------------------------------------------ capture

    fn start_capture(&mut self, kind: CaptureKind, ctx: &egui::Context, from_hotkey: bool) {
        if kind == CaptureKind::Video && self.recorder.is_some() {
            self.stop_recording(ctx);
            return;
        }
        // Une capture qui ne revient pas (pilote, écran en veille…) ne doit pas tout bloquer.
        if self.busy && self.busy_since.elapsed() > Duration::from_secs(20) {
            crate::diag::log("capture précédente sans réponse depuis 20 s : abandonnée");
            self.busy = false;
        }
        if self.busy || self.overlay.is_some() || self.recorder.is_some() {
            crate::diag::log(&format!(
                "capture {kind:?} ignorée (occupé {}, sélection {}, vidéo {})",
                self.busy,
                self.overlay.is_some(),
                self.recorder.is_some()
            ));
            return;
        }
        crate::diag::log(&format!("capture {kind:?} (raccourci {from_hotkey}, mode {:?}, affichée {})", self.mode, self.shown));
        // Une nouvelle capture remplace la miniature éventuellement affichée.
        self.popup = None;
        // Capture de fenêtre par raccourci : fenêtre au premier plan (si ce n'est pas la nôtre).
        let target = if kind == CaptureKind::Window && from_hotkey && !(self.shown && platform::is_foreground(self.win)) {
            capture::foreground_window_rect()
        } else {
            None
        };
        if kind == CaptureKind::LastRegion && self.settings.last_region.is_none() {
            self.toast(crate::i18n::tr("Aucune zone précédente : sélectionnez une zone."));
        }
        // La fenêtre principale ne doit pas figurer sur la capture : elle est masquée si elle est à
        // l'écran, et réaffichée à la fin. Réduite ou dans la zone de notification, on n'y touche pas.
        let on_screen = self.shown && !self.win.is_minimized();
        self.restore_after_capture = on_screen;
        if on_screen {
            self.hide_root();
        }
        self.busy = true;
        self.busy_since = Instant::now();
        let delay = match kind {
            CaptureKind::Delayed => Duration::from_secs(self.settings.delay_seconds as u64),
            // Laisser au compositeur le temps de retirer la fenêtre de l'écran.
            _ if on_screen => Duration::from_millis(150),
            _ => Duration::from_millis(40),
        };
        let cursor = self.settings.capture_cursor;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            let origin = capture::virtual_screen();
            let started = Instant::now();
            let result = std::panic::catch_unwind(|| capture::capture_rect(origin, cursor));
            let ev = match result {
                Ok(Ok(image)) => AppEvent::Captured { kind, image, origin, target },
                Ok(Err(e)) => AppEvent::CaptureFailed(e.to_string()),
                Err(_) => AppEvent::CaptureFailed(crate::i18n::tr("erreur interne pendant la capture").into()),
            };
            crate::diag::log(&format!(
                "écran capturé {origin:?} en {} ms : {}",
                started.elapsed().as_millis(),
                match &ev {
                    AppEvent::CaptureFailed(e) => e.as_str(),
                    _ => "ok",
                }
            ));
            let _ = tx.send(ev);
            ctx.request_repaint();
        });
    }

    fn on_captured(&mut self, ctx: &egui::Context, kind: CaptureKind, image: RgbaImage, origin: Rect, target: Option<Rect>) {
        self.busy = false;
        let direct = match kind {
            CaptureKind::Fullscreen => Some(origin),
            CaptureKind::Window => target,
            CaptureKind::LastRegion => self.settings.last_region,
            _ => None,
        };
        if let Some(r) = direct {
            match capture::crop_virtual(&image, origin, r) {
                Some(img) => self.finish_capture(ctx, img),
                None => self.end_capture(),
            }
            return;
        }
        let purpose = if kind == CaptureKind::Video { Purpose::Video } else { Purpose::Image };
        let mut overlay = Overlay::new(purpose, origin, image);
        overlay.magnifier = self.settings.show_magnifier;
        if purpose == Purpose::Video && self.settings.audio_device.is_some() && self.settings.video_format != VideoFormat::Gif {
            overlay.audio = Some(self.settings.record_audio);
        }
        self.overlay = Some(overlay);
        self.overlay_win = Floating::default();
        crate::diag::log(&format!("sélection {origin:?}"));
        ctx.request_repaint();
    }

    fn show_capture_popup(&mut self, ctx: &egui::Context, image: RgbaImage, path: Option<PathBuf>) {
        // En bas à droite de l'écran où se trouve le pointeur, à l'échelle de cet écran.
        let (cx, cy) = capture::cursor_pos();
        let wa = capture::work_area_at(cx, cy);
        let s = platform::scale_at(cx, cy);
        let (w, h) = ((336.0 * s) as i32, (282.0 * s) as i32);
        let margin = (16.0 * s) as i32;
        let rect = Rect::new(wa.x + wa.w - w - margin, wa.y + wa.h - h - margin, w, h);
        crate::diag::log(&format!("miniature {rect:?}"));
        self.popup = Some(CapturePopup { image, path, texture: None, since: Instant::now(), rect, notice: None });
        self.popup_win = Floating::default();
        ctx.request_repaint();
    }

    /// Miniature : fenêtre flottante distincte, sans effet sur la fenêtre principale.
    fn capture_popup_viewport(&mut self, ctx: &egui::Context) {
        let Some(rect) = self.popup.as_ref().map(|p| p.rect) else { return };
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("sc_popup"), floating_builder(TITLE_POPUP, rect), |vctx, _| {
            self.popup_win.sync(TITLE_POPUP, rect, false, false, true);
            self.capture_popup_ui(vctx);
        });
    }

    fn capture_popup_ui(&mut self, ctx: &egui::Context) {
        let p = palette(ctx);
        let Some(pop) = self.popup.as_mut() else { return };
        if pop.texture.is_none() {
            let t = crate::workspace::fit_thumbnail(&pop.image, 640, 360);
            let ci = egui::ColorImage::from_rgba_unmultiplied([t.width() as usize, t.height() as usize], t.as_raw());
            pop.texture = Some(ctx.load_texture("capture-popup", ci, egui::TextureOptions::LINEAR));
        }
        // Le survol suspend la disparition automatique.
        if ctx.input(|i| i.pointer.has_pointer()) {
            pop.since = Instant::now();
        }
        let remaining = 1.0 - pop.since.elapsed().as_secs_f32() / POPUP_DELAY.as_secs_f32();
        let name = pop.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Capture".into());
        let size = (pop.image.width(), pop.image.height());
        let tex = pop.texture.clone();
        let title = match &pop.notice {
            Some((msg, at)) if at.elapsed() < Duration::from_secs(2) => msg.clone(),
            _ if self.settings.copy_to_clipboard => crate::i18n::tr("Capture enregistrée et copiée").to_string(),
            _ => crate::i18n::tr("Capture enregistrée").to_string(),
        };
        let (mut edit, mut close, mut copy, mut reveal) = (false, false, false, false);

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.surface_container).inner_margin(Margin::same(12)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(ICON_CHECK_CIRCLE).size(20.0).color(p.primary));
                    ui.label(egui::RichText::new(title).font(theme::medium(14.0)).color(p.on_surface));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = theme::icon_button(ui, ICON_CLOSE, false, crate::i18n::tr("Fermer")).clicked();
                    });
                });
                // Miniature cliquable : ouvre l'éditeur.
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 160.0), egui::Sense::click());
                let painter = ui.painter();
                painter.rect_filled(rect, 12.0, p.surface_highest);
                if let Some(t) = &tex {
                    let ts = t.size_vec2();
                    let k = (rect.width() / ts.x).min(rect.height() / ts.y);
                    let img_rect = egui::Rect::from_center_size(rect.center(), ts * k);
                    painter.image(t.id(), img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                }
                if resp.hovered() {
                    painter.rect_filled(rect, 12.0, Color32::from_black_alpha(110));
                    let chip = egui::Rect::from_center_size(rect.center(), vec2(120.0, 40.0));
                    painter.rect_filled(chip, 20.0, p.primary);
                    painter.text(chip.left_center() + vec2(28.0, 0.0), Align2::CENTER_CENTER, ICON_EDIT, FontId::proportional(18.0), p.on_primary);
                    painter.text(chip.left_center() + vec2(46.0, 0.0), Align2::LEFT_CENTER, crate::i18n::tr("Éditer"), theme::medium(14.0), p.on_primary);
                }
                edit = resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(crate::i18n::tr("Ouvrir dans l'éditeur")).clicked();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.add_space(2.0);
                        let short = if name.chars().count() > 30 { format!("{}…", name.chars().take(29).collect::<String>()) } else { name.clone() };
                        ui.label(egui::RichText::new(short).size(12.5).color(p.on_surface));
                        ui.label(egui::RichText::new(crate::i18n::trf("{0} × {1} px", &[&size.0, &size.1])).size(11.5).color(p.on_surface_variant));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        reveal = theme::icon_button(ui, ICON_FOLDER_OPEN, false, platform::reveal_label()).clicked();
                        copy = theme::icon_button(ui, ICON_CONTENT_COPY, false, crate::i18n::tr("Copier")).clicked();
                    });
                });
            });
        // Barre de temps restant avant disparition, au bas de la fenêtre.
        let screen = ctx.screen_rect();
        let bar = egui::Rect::from_min_size(
            egui::pos2(screen.left(), screen.bottom() - 4.0),
            vec2(screen.width() * remaining.clamp(0.0, 1.0), 4.0),
        );
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("popup_timer"))).rect_filled(bar, 0.0, p.primary);
        ctx.request_repaint_after(Duration::from_millis(50));

        if copy {
            if let Some(pop) = self.popup.as_mut() {
                let msg = match copy_image(&pop.image) {
                    Ok(()) => crate::i18n::tr("Copiée dans le presse-papiers").to_string(),
                    Err(e) => crate::i18n::trf("Erreur : {0}", &[&e]),
                };
                pop.notice = Some((msg, Instant::now()));
            }
        }
        if reveal {
            if let Some(path) = self.popup.as_ref().and_then(|pop| pop.path.clone()) {
                platform::reveal_in_explorer(&path);
            }
        }
        let closed_by_system = ctx.input(|i| i.viewport().close_requested());
        if edit {
            if let Some(pop) = self.popup.take() {
                self.open_editor(pop.image, pop.path);
                self.show_root();
            }
        } else if close || closed_by_system || remaining <= 0.0 {
            self.popup = None;
        }
    }

    fn finish_capture(&mut self, ctx: &egui::Context, img: RgbaImage) {
        if self.settings.copy_to_clipboard {
            if let Err(e) = copy_image(&img) {
                self.toast(crate::i18n::trf("Presse-papiers indisponible : {0}", &[&e]));
            }
        }
        let mut path = None;
        if self.settings.auto_save {
            match self.save_new(&img) {
                Ok(p) => path = Some(p),
                Err(e) => self.toast(crate::i18n::trf("Enregistrement impossible : {0}", &[&e])),
            }
        }
        match self.settings.after_capture {
            AfterCapture::OpenEditor => {
                self.close_editor(true);
                self.editor = Some(Editor::new(img, path));
                self.mode = Mode::Editor;
                self.restore_after_capture = false;
                self.show_root();
            }
            AfterCapture::ShowWorkspace => {
                self.page = Page::Home;
                self.show_capture_popup(ctx, img, path);
                self.end_capture();
            }
            AfterCapture::Nothing => self.end_capture(),
        }
    }

    fn new_capture_path(&self, ext: &str) -> PathBuf {
        self.unique_path("Capture", ext)
    }

    /// `<préfixe>_<date>.<ext>` dans le dossier des captures, sans écraser un fichier existant.
    fn unique_path(&self, prefix: &str, ext: &str) -> PathBuf {
        let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let mut p = self.settings.save_dir.join(format!("{prefix}_{stamp}.{ext}"));
        let mut n = 2;
        while p.exists() {
            p = self.settings.save_dir.join(format!("{prefix}_{stamp}_{n}.{ext}"));
            n += 1;
        }
        p
    }

    fn save_new(&mut self, img: &RgbaImage) -> anyhow::Result<PathBuf> {
        std::fs::create_dir_all(&self.settings.save_dir)?;
        let path = self.new_capture_path(self.settings.image_format.ext());
        save_image(img, &path, self.settings.jpeg_quality)?;
        self.workspace.refresh();
        Ok(path)
    }

    // ------------------------------------------------------------------ éditeur

    fn open_editor(&mut self, img: RgbaImage, path: Option<PathBuf>) {
        self.close_editor(true);
        self.editor = Some(Editor::new(img, path));
        self.mode = Mode::Editor;
    }

    /// Ferme l'éditeur ; les modifications sont enregistrées automatiquement.
    fn close_editor(&mut self, autosave: bool) {
        let Some(mut ed) = self.editor.take() else { return };
        if autosave && ed.modified {
            let _ = self.save_editor(&mut ed, None);
        }
        if self.mode == Mode::Editor {
            self.mode = Mode::Workspace;
        }
    }

    fn save_editor(&mut self, ed: &mut Editor, target: Option<PathBuf>) -> anyhow::Result<PathBuf> {
        let img = ed.export_image(&self.settings);
        let path = match target.or_else(|| ed.path.clone()) {
            Some(p) => p,
            None => {
                std::fs::create_dir_all(&self.settings.save_dir)?;
                self.new_capture_path(self.settings.image_format.ext())
            }
        };
        save_image(&img, &path, self.settings.jpeg_quality)?;
        ed.path = Some(path.clone());
        ed.modified = false;
        self.workspace.forget(&path);
        Ok(path)
    }

    fn editor_actions(&mut self, ctx: &egui::Context, actions: Vec<EditorAction>) {
        for a in actions {
            let Some(mut ed) = self.editor.take() else { return };
            match a {
                EditorAction::Close => {
                    self.editor = Some(ed);
                    let modified = self.editor.as_ref().map(|e| e.modified).unwrap_or(false);
                    self.close_editor(true);
                    if modified {
                        self.toast(crate::i18n::tr("Modifications enregistrées"));
                    }
                    continue;
                }
                EditorAction::Save => match self.save_editor(&mut ed, None) {
                    Ok(p) => self.toast(crate::i18n::trf("Enregistré : {0}", &[&p.file_name().unwrap_or_default().to_string_lossy()])),
                    Err(e) => self.toast(crate::i18n::trf("Erreur : {0}", &[&e])),
                },
                EditorAction::SaveAs => {
                    let name = ed.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Capture".into());
                    let file = crate::diag::in_dialog(|| {
                        rfd::FileDialog::new()
                            .set_directory(&self.settings.save_dir)
                            .set_file_name(format!("{name}.png"))
                            .add_filter(crate::i18n::tr("Image PNG"), &["png"])
                            .add_filter(crate::i18n::tr("Image JPEG"), &["jpg", "jpeg"])
                            .add_filter(crate::i18n::tr("Image BMP"), &["bmp"])
                            .save_file()
                    });
                    if let Some(f) = file {
                        match self.save_editor(&mut ed, Some(f)) {
                            Ok(p) => self.toast(crate::i18n::trf("Enregistré : {0}", &[&p.display()])),
                            Err(e) => self.toast(crate::i18n::trf("Erreur : {0}", &[&e])),
                        }
                    }
                }
                EditorAction::Copy => {
                    let img = ed.export_image(&self.settings);
                    match copy_image(&img) {
                        Ok(()) => self.toast(crate::i18n::tr("Image copiée dans le presse-papiers")),
                        Err(e) => self.toast(crate::i18n::trf("Erreur : {0}", &[&e])),
                    }
                }
                EditorAction::Ocr => {
                    let img = ed.export_image(&Settings { effect_border: false, effect_shadow: false, ..self.settings.clone() });
                    let tx = self.tx.clone();
                    let ctx = ctx.clone();
                    self.toast(crate::i18n::tr("Reconnaissance du texte…"));
                    std::thread::spawn(move || {
                        let r = platform::ocr(&img).map_err(|e| e.to_string());
                        let _ = tx.send(AppEvent::Ocr(r));
                        ctx.request_repaint();
                    });
                }
            }
            self.editor = Some(ed);
        }
    }

    // ------------------------------------------------------------------ vidéo

    fn start_recording(&mut self, ctx: &egui::Context, region: Rect) {
        let out = self.new_capture_path(self.settings.video_format.ext()).with_file_name(format!(
            "Video_{}.{}",
            chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"),
            self.settings.video_format.ext()
        ));
        match Recorder::start(&self.settings, region, out) {
            Ok(rec) => {
                if let Some(w) = &rec.warning {
                    self.toast(w.clone());
                }
                let s = platform::scale_at(region.x + region.w / 2, region.y + region.h / 2);
                let (w, h) = ((360.0 * s) as i32, (60.0 * s) as i32);
                let mon = capture::monitor_at(region.x + region.w / 2, region.y + region.h / 2);
                let x = (region.x + region.w / 2 - w / 2).clamp(mon.x, mon.x + mon.w - w);
                let y = if region.y - h - 12 >= mon.y {
                    region.y - h - 12
                } else if region.y + region.h + 12 + h <= mon.y + mon.h {
                    region.y + region.h + 12
                } else {
                    mon.y + 12
                };
                self.recorder = Some(rec);
                self.recbar_rect = Rect::new(x, y, w, h);
                self.recbar_win = Floating::default();
                ctx.request_repaint();
            }
            Err(e) => {
                self.end_capture();
                self.toast(e.to_string());
            }
        }
    }

    fn stop_recording(&mut self, ctx: &egui::Context) {
        let Some(rec) = self.recorder.take() else { return };
        self.finishing_video = true;
        let tx = self.tx.clone();
        let c = ctx.clone();
        std::thread::spawn(move || {
            let r = rec.finish().map_err(|e| e.to_string());
            let _ = tx.send(AppEvent::VideoDone(r));
            c.request_repaint();
        });
        self.page = Page::Home;
        self.restore_after_capture = false;
        self.show_root();
        self.toast(if self.settings.video_format == VideoFormat::Gif { crate::i18n::tr("Conversion en GIF…") } else { crate::i18n::tr("Finalisation de la vidéo…") });
    }

    fn cancel_recording(&mut self, ctx: &egui::Context) {
        let _ = ctx;
        if let Some(rec) = self.recorder.take() {
            rec.abort();
        }
        self.end_capture();
        self.toast(crate::i18n::tr("Enregistrement annulé"));
    }

    // ------------------------------------------------------------------ événements

    fn handle_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.rx.try_recv() {
            crate::diag::step("traitement d'un évènement");
            match &ev {
                AppEvent::Hotkey(id) => crate::diag::log(&format!("raccourci global {id}")),
                AppEvent::Shortcut(k) => crate::diag::log(&format!("raccourci Impr. écran {k:?}")),
                AppEvent::Menu(id) => crate::diag::log(&format!("menu {:?}", self.menu_ids.get(id))),
                AppEvent::TrayOpen => crate::diag::log(&format!("ouverture demandée (mode {:?}, sélection {})", self.mode, self.overlay.is_some())),
                AppEvent::Command(c) => crate::diag::log(&format!("commande « {c} »")),
                AppEvent::CancelCapture => crate::diag::log("Échap : annulation de la capture"),
                _ => {}
            }
            match ev {
                AppEvent::Hotkey(id) => {
                    if let Some(k) = self.hotkeys.kind_of(id) {
                        self.shortcut(k, ctx);
                    }
                }
                AppEvent::Shortcut(k) => self.shortcut(k, ctx),
                AppEvent::Menu(id) => match self.menu_ids.get(&id).copied() {
                    Some("region") => self.start_capture(CaptureKind::Region, ctx, false),
                    Some("full") => self.start_capture(CaptureKind::Fullscreen, ctx, false),
                    Some("window") => self.start_capture(CaptureKind::Window, ctx, false),
                    Some("last") => self.start_capture(CaptureKind::LastRegion, ctx, false),
                    Some("delayed") => self.start_capture(CaptureKind::Delayed, ctx, false),
                    Some("video") => self.start_capture(CaptureKind::Video, ctx, false),
                    Some("open") => {
                        if self.mode != Mode::Editor {
                            self.page = Page::Home;
                        }
                        self.show_root();
                    }
                    Some("folder") => platform::open_folder(&self.settings.save_dir),
                    Some("settings") => {
                        self.close_editor(true);
                        self.mode = Mode::Workspace;
                        self.page = Page::Settings;
                        self.show_root();
                    }
                    Some("quit") => self.request_quit(ctx),
                    _ => {}
                },
                AppEvent::TrayOpen => self.open_from_outside(),
                AppEvent::Command(c) => self.command(&c, ctx),
                AppEvent::CancelCapture => self.cancel_capture(),
                // Capture annulée par Échap pendant le délai : l'image arrivée est ignorée.
                AppEvent::Captured { .. } | AppEvent::CaptureFailed(_) if std::mem::take(&mut self.capture_cancelled) => {}
                AppEvent::Captured { kind, image, origin, target } => self.on_captured(ctx, kind, image, origin, target),
                AppEvent::CaptureFailed(e) => {
                    self.busy = false;
                    self.end_capture();
                    self.toast(crate::i18n::trf("Échec de la capture : {0}", &[&e]));
                }
                AppEvent::Ocr(r) => match r {
                    Ok(t) if t.trim().is_empty() => self.toast(crate::i18n::tr("Aucun texte détecté")),
                    Ok(t) => {
                        let _ = arboard::Clipboard::new().and_then(|mut c| c.set_text(t.clone()));
                        self.ocr_text = Some(t);
                    }
                    Err(e) => self.toast(crate::i18n::trf("OCR indisponible : {0}", &[&e])),
                },
                AppEvent::VideoDone(r) => {
                    self.finishing_video = false;
                    match r {
                        Ok(p) => {
                            self.workspace.refresh();
                            self.toast(crate::i18n::trf("Vidéo enregistrée : {0}", &[&p.file_name().unwrap_or_default().to_string_lossy()]));
                        }
                        Err(e) => self.toast(crate::i18n::trf("Erreur vidéo : {0}", &[&e])),
                    }
                }
                AppEvent::AudioDevices(d) => self.audio_devices = d,
                AppEvent::RetentionDone { n, bytes, errors, manual } => {
                    self.retention_running = false;
                    self.retention_preview = None;
                    crate::diag::log(&format!("conservation : {} supprimée(s), {} erreur(s)", crate::retention::describe(n, bytes), errors.len()));
                    for e in &errors {
                        crate::diag::log(&format!("conservation : {e}"));
                    }
                    if n > 0 {
                        self.workspace.refresh();
                        let how = if self.settings.retention_to_trash { crate::i18n::tr("envoyée(s) dans la Corbeille") } else { crate::i18n::tr("supprimée(s)") };
                        self.toast(crate::i18n::trf("Conservation : {0} {1}", &[&crate::retention::describe(n, bytes), &how]));
                    } else if manual {
                        self.toast(crate::i18n::tr("Conservation : rien à supprimer"));
                    }
                    if let Some(e) = errors.first() {
                        self.toast(crate::i18n::trf("Conservation : {0}", &[&e]));
                    }
                }
            }
        }
    }

    /// Échap pendant une capture : délai en cours, écran en cours de prise ou sélection affichée.
    fn cancel_capture(&mut self) {
        if self.overlay.take().is_some() {
            crate::diag::log("sélection annulée (Échap)");
            self.end_capture();
        } else if self.busy {
            crate::diag::log("capture annulée avant la prise (Échap)");
            self.busy = false;
            self.capture_cancelled = true;
            self.end_capture();
        }
    }

    /// Icône de notification, relance depuis le menu Démarrer : abandonner une sélection éventuelle
    /// et afficher la fenêtre principale.
    fn open_from_outside(&mut self) {
        if self.overlay.take().is_some() {
            self.restore_after_capture = false;
        }
        self.show_root();
    }

    /// Commandes de pilotage (`ScreenCapture.exe --cmd …`), utilisées par les tests de validation.
    fn command(&mut self, c: &str, ctx: &egui::Context) {
        match c {
            "show" => self.open_from_outside(),
            "hide" => self.hide_root(),
            "region" => self.start_capture(CaptureKind::Region, ctx, false),
            "full" => self.start_capture(CaptureKind::Fullscreen, ctx, false),
            "window" => self.start_capture(CaptureKind::Window, ctx, true),
            "last" => self.start_capture(CaptureKind::LastRegion, ctx, false),
            "video" => self.start_capture(CaptureKind::Video, ctx, false),
            "delayed" => self.start_capture(CaptureKind::Delayed, ctx, false),
            "mic" => self.toggle_recording_mic(),
            "retention" => self.run_retention(ctx, true),
            c if c.starts_with("pin ") => {
                let path = self.settings.save_dir.join(&c[4..]);
                self.toggle_pins(&[path]);
            }
            "cancel" => {
                if self.overlay.take().is_some() {
                    self.end_capture();
                }
                self.popup = None;
            }
            "editor" => {
                // Ouvre la dernière capture dans l'éditeur (scénarios de test avec l'éditeur).
                let last = std::fs::read_dir(&self.settings.save_dir)
                    .ok()
                    .and_then(|rd| {
                        rd.flatten()
                            .filter(|e| e.path().extension().is_some_and(|x| x == "png" || x == "jpg"))
                            .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
                    })
                    .map(|e| e.path());
                if let Some(path) = last {
                    if let Ok(img) = image::open(&path) {
                        self.open_editor(img.to_rgba8(), Some(path));
                        self.show_root();
                    }
                }
            }
            "editor-copy" => {
                if let Some(ed) = self.editor.as_mut() {
                    let img = ed.export_image(&self.settings);
                    let r = copy_image(&img);
                    crate::diag::log(&format!("copie depuis l'éditeur : {r:?}"));
                }
            }
            "cards" => {
                // Position des vignettes en pixels d'écran (tests de validation).
                let (origin, ppp) = ctx.input(|i| (i.viewport().inner_rect.map(|r| r.min).unwrap_or_default(), i.pixels_per_point()));
                for (i, (path, r)) in self.workspace.card_rects.iter().enumerate() {
                    crate::diag::log(&format!(
                        "carte {i} {} {} {} {} {}",
                        ((origin.x + r.min.x) * ppp).round(),
                        ((origin.y + r.min.y) * ppp).round(),
                        (r.width() * ppp).round(),
                        (r.height() * ppp).round(),
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
            }
            "home" | "settings" | "about" => {
                self.close_editor(true);
                self.mode = Mode::Workspace;
                self.page = match c {
                    "settings" => Page::Settings,
                    "about" => Page::About,
                    _ => Page::Home,
                };
                self.open_from_outside();
            }
            "state" => {}
            "quit" => self.request_quit(ctx),
            other => crate::diag::log(&format!("commande inconnue « {other} »")),
        }
        crate::diag::log(&self.describe_state());
    }

    /// Un même appui peut arriver par le hook et par RegisterHotKey : on l'ignore s'il est trop rapproché.
    fn shortcut(&mut self, k: CaptureKind, ctx: &egui::Context) {
        if let Some((last, at)) = self.last_shortcut {
            if last == k && at.elapsed() < Duration::from_millis(400) {
                return;
            }
        }
        self.last_shortcut = Some((k, Instant::now()));
        self.start_capture(k, ctx, true);
    }

    fn request_quit(&mut self, ctx: &egui::Context) {
        crate::diag::log("arrêt demandé");
        self.close_editor(true);
        if let Some(rec) = self.recorder.take() {
            let _ = rec.finish();
        }
        self.settings.save();
        self.quit = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn set_title(&mut self, ctx: &egui::Context, title: String) {
        if self.title != title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn apply_theme_if_needed(&mut self, ctx: &egui::Context) {
        let sys_dark = ctx.system_theme().map(|t| t == egui::Theme::Dark).unwrap_or(false);
        let key = (self.settings.theme, self.settings.accent, sys_dark);
        if self.applied_theme != Some(key) {
            theme::apply(ctx, self.settings.theme, self.settings.accent);
            self.applied_theme = Some(key);
        }
    }

    // ------------------------------------------------------------------ pages

    fn nav_rail(&mut self, ctx: &egui::Context) {
        let p = palette(ctx);
        egui::SidePanel::left("rail")
            .exact_width(96.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(p.surface_container).inner_margin(Margin::symmetric(8, 16)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // FAB
                    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(56.0), egui::Sense::click());
                    let fill = p.state_layer(p.primary_container, p.on_primary_container, &resp);
                    ui.painter().rect_filled(rect.translate(vec2(0.0, 2.0)), 16.0, Color32::from_black_alpha(40));
                    ui.painter().rect_filled(rect, 16.0, fill);
                    ui.painter().text(rect.center(), Align2::CENTER_CENTER, ICON_SCREENSHOT_REGION, FontId::proportional(26.0), p.on_primary_container);
                    if resp.on_hover_text(crate::i18n::tr("Capturer une zone")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.start_capture(CaptureKind::Region, ctx, false);
                    }
                    ui.add_space(20.0);
                    for (page, icon, label) in [(Page::Home, ICON_HOME, crate::i18n::tr("Accueil")), (Page::Settings, ICON_SETTINGS, crate::i18n::tr("Paramètres")), (Page::About, ICON_INFO, crate::i18n::tr("À propos"))] {
                        if theme::nav_item(ui, icon, label, self.page == page).clicked() {
                            self.page = page;
                        }
                        ui.add_space(4.0);
                    }
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        if theme::icon_button(ui, ICON_POWER_SETTINGS_NEW, false, crate::i18n::tr("Quitter ScreenCapture")).clicked() {
                            self.request_quit(ctx);
                        }
                        let dark = p.dark;
                        if theme::icon_button(ui, if dark { ICON_LIGHT_MODE } else { ICON_DARK_MODE }, false, crate::i18n::tr("Basculer le thème")).clicked() {
                            self.settings.theme = if dark { ThemeChoice::Light } else { ThemeChoice::Dark };
                            self.settings.save();
                        }
                    });
                });
            });
    }

    fn settings_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = palette(ctx);
        let mut changed = false;
        let mut refresh_tray = false;
        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            ui.set_max_width(820.0);
            ui.add_space(8.0);
            ui.label(egui::RichText::new(crate::i18n::tr("Paramètres")).font(theme::medium(28.0)).color(p.on_surface));
            ui.add_space(16.0);

            theme::card(ui, crate::i18n::tr("Enregistrement"), ICON_FOLDER, |ui| {
                theme::setting_row(ui, crate::i18n::tr("Dossier des captures"), &self.settings.save_dir.display().to_string(), |ui| {
                    if theme::button(ui, ButtonKind::Tonal, Some(ICON_FOLDER_OPEN), crate::i18n::tr("Parcourir")).clicked() {
                        if let Some(d) = crate::diag::in_dialog(|| rfd::FileDialog::new().set_directory(&self.settings.save_dir).pick_folder()) {
                            self.settings.save_dir = d;
                            self.workspace.refresh();
                            changed = true;
                        }
                    }
                });
                theme::setting_row(ui, crate::i18n::tr("Format d'image"), crate::i18n::tr("PNG sans perte ou JPEG compressé"), |ui| {
                    if theme::filter_chip(ui, "JPEG", self.settings.image_format == ImageFormat::Jpg).clicked() {
                        self.settings.image_format = ImageFormat::Jpg;
                        changed = true;
                    }
                    if theme::filter_chip(ui, "PNG", self.settings.image_format == ImageFormat::Png).clicked() {
                        self.settings.image_format = ImageFormat::Png;
                        changed = true;
                    }
                });
                if self.settings.image_format == ImageFormat::Jpg {
                    theme::setting_row(ui, crate::i18n::tr("Qualité JPEG"), "", |ui| {
                        changed |= ui.add(egui::Slider::new(&mut self.settings.jpeg_quality, 40..=100)).changed();
                    });
                }
                theme::setting_row(ui, crate::i18n::tr("Enregistrer automatiquement"), crate::i18n::tr("Chaque capture est enregistrée dans le dossier"), |ui| {
                    changed |= theme::switch(ui, &mut self.settings.auto_save).changed();
                });
                theme::setting_row(ui, crate::i18n::tr("Copier dans le presse-papiers"), crate::i18n::tr("Chaque capture est copiée immédiatement"), |ui| {
                    changed |= theme::switch(ui, &mut self.settings.copy_to_clipboard).changed();
                });
            });
            ui.add_space(12.0);

            changed |= self.retention_card(ui, ctx);
            ui.add_space(12.0);

            theme::card(ui, crate::i18n::tr("Capture"), ICON_SCREENSHOT_REGION, |ui| {
                theme::setting_row(ui, crate::i18n::tr("Après la capture"), "", |ui| {
                    for (v, label) in [(AfterCapture::Nothing, crate::i18n::tr("Rien")), (AfterCapture::ShowWorkspace, crate::i18n::tr("Miniature")), (AfterCapture::OpenEditor, crate::i18n::tr("Éditeur"))] {
                        if theme::filter_chip(ui, label, self.settings.after_capture == v).clicked() {
                            self.settings.after_capture = v;
                            changed = true;
                        }
                    }
                });
                theme::setting_row(ui, crate::i18n::tr("Loupe de précision"), crate::i18n::tr("Affiche les pixels agrandis et la couleur sous le curseur pendant la sélection"), |ui| {
                    changed |= theme::switch(ui, &mut self.settings.show_magnifier).changed();
                });
                theme::setting_row(ui, crate::i18n::tr("Inclure le curseur"), crate::i18n::tr("Dessine le pointeur de la souris dans la capture"), |ui| {
                    changed |= theme::switch(ui, &mut self.settings.capture_cursor).changed();
                });
                theme::setting_row(ui, crate::i18n::tr("Délai de la capture différée"), "", |ui| {
                    changed |= ui.add(egui::Slider::new(&mut self.settings.delay_seconds, 1..=30).suffix(" s")).changed();
                });
            });
            ui.add_space(12.0);

            theme::card(ui, crate::i18n::tr("Raccourcis clavier globaux"), ICON_KEYBOARD, |ui| {
                let examples = if cfg!(target_os = "macos") {
                    crate::i18n::tr("Exemples : super+shift+Digit2 (⌘⇧2), ctrl+alt+KeyS, F13. Laisser vide pour désactiver.")
                } else {
                    crate::i18n::tr("Exemples : PrintScreen, ctrl+PrintScreen, ctrl+shift+KeyS, alt+F9. Laisser vide pour désactiver.")
                };
                ui.label(egui::RichText::new(examples).size(12.5).color(p.on_surface_variant));
                ui.add_space(6.0);
                let status = self.hotkeys.status.clone();
                let ok = |k: CaptureKind| status.iter().find(|(c, _)| *c == k).map(|(_, o)| *o).unwrap_or(false);
                let rows: [(&str, CaptureKind); 5] = [
                    (crate::i18n::tr("Capturer une zone"), CaptureKind::Region),
                    (crate::i18n::tr("Plein écran"), CaptureKind::Fullscreen),
                    (crate::i18n::tr("Fenêtre active"), CaptureKind::Window),
                    (crate::i18n::tr("Dernière zone"), CaptureKind::LastRegion),
                    (crate::i18n::tr("Vidéo (démarrer / arrêter)"), CaptureKind::Video),
                ];
                for (label, kind) in rows {
                    let field = match kind {
                        CaptureKind::Region => &mut self.hotkey_edit.region,
                        CaptureKind::Fullscreen => &mut self.hotkey_edit.fullscreen,
                        CaptureKind::Window => &mut self.hotkey_edit.window,
                        CaptureKind::LastRegion => &mut self.hotkey_edit.last_region,
                        _ => &mut self.hotkey_edit.video,
                    };
                    let desc = if field.trim().is_empty() {
                        crate::i18n::tr("Désactivé").to_string()
                    } else if ok(kind) {
                        crate::i18n::trf("Actif : {0}", &[&pretty_hotkey(field)])
                    } else {
                        crate::i18n::tr("Non enregistré (déjà utilisé par une autre application, ou syntaxe invalide)").to_string()
                    };
                    theme::setting_row(ui, label, &desc, |ui| {
                        ui.label(egui::RichText::new(if ok(kind) { ICON_CHECK_CIRCLE } else { ICON_ERROR }).size(20.0).color(if ok(kind) { p.primary } else { p.error }));
                        theme::text_field(ui, field, 190.0, "");
                    });
                }
                #[cfg(windows)]
                theme::setting_row(
                    ui,
                    crate::i18n::tr("Remplacer Win + Maj + S"),
                    crate::i18n::tr("Capture de zone au lieu de l'Outil Capture de Windows (utile si la touche capture du clavier envoie ce raccourci)"),
                    |ui| {
                        if theme::switch(ui, &mut self.hotkey_edit.override_snip).changed() {
                            self.settings.hotkeys.override_snip = self.hotkey_edit.override_snip;
                            self.hotkeys.apply(&self.settings.hotkeys);
                            changed = true;
                        }
                    },
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if theme::button(ui, ButtonKind::Filled, Some(ICON_CHECK), crate::i18n::tr("Appliquer les raccourcis")).clicked() {
                        self.settings.hotkeys = self.hotkey_edit.clone();
                        self.hotkeys.apply(&self.settings.hotkeys);
                        refresh_tray = true;
                        changed = true;
                    }
                    if theme::button(ui, ButtonKind::Text, Some(ICON_RESTART_ALT), crate::i18n::tr("Valeurs par défaut")).clicked() {
                        self.hotkey_edit = crate::settings::Hotkeys::default();
                    }
                });
                let tip = if cfg!(target_os = "macos") {
                    crate::i18n::tr("Astuce : ⌘⇧3, ⌘⇧4 et ⌘⇧5 sont réservés par macOS (Réglages Système > Clavier > Raccourcis clavier > Captures d'écran).")
                } else {
                    crate::i18n::tr("Astuce : si Impr. écran ouvre l'Outil Capture de Windows, désactivez « Utiliser la touche Impr. écran pour ouvrir la capture d'écran » dans Paramètres Windows > Accessibilité > Clavier.")
                };
                ui.label(egui::RichText::new(tip).size(12.0).color(p.on_surface_variant));
            });
            ui.add_space(12.0);

            theme::card(ui, crate::i18n::tr("Vidéo"), ICON_VIDEOCAM, |ui| {
                let found = recorder::find_ffmpeg(&self.settings);
                let desc = match &found {
                    Some(f) => crate::i18n::trf("Utilisé : {0}", &[&f.display()]),
                    None => crate::i18n::trf("Introuvable — installez FFmpeg ({0}) ou indiquez son emplacement", &[&recorder::FFMPEG_HINT]),
                };
                theme::setting_row(ui, "FFmpeg", &desc, |ui| {
                    if theme::button(ui, ButtonKind::Tonal, Some(ICON_FOLDER_OPEN), crate::i18n::tr("Choisir")).clicked() {
                        if let Some(f) = crate::diag::in_dialog(|| {
                            let d = rfd::FileDialog::new();
                            if cfg!(windows) { d.add_filter("ffmpeg", &["exe"]) } else { d }.pick_file()
                        }) {
                            self.settings.ffmpeg_path = f.display().to_string();
                            changed = true;
                        }
                    }
                });
                theme::setting_row(ui, crate::i18n::tr("Format"), "", |ui| {
                    for (v, label) in [(VideoFormat::Gif, "GIF"), (VideoFormat::Webm, "WebM"), (VideoFormat::Mp4, "MP4")] {
                        if theme::filter_chip(ui, label, self.settings.video_format == v).clicked() {
                            self.settings.video_format = v;
                            changed = true;
                        }
                    }
                });
                theme::setting_row(ui, crate::i18n::tr("Images par seconde"), "", |ui| {
                    changed |= ui.add(egui::Slider::new(&mut self.settings.video_fps, 5..=60)).changed();
                });
                theme::setting_row(ui, crate::i18n::tr("Afficher le cadre d'enregistrement"), "", |ui| {
                    changed |= theme::switch(ui, &mut self.settings.video_show_region).changed();
                });
                theme::setting_row(ui, crate::i18n::tr("Enregistrer le curseur"), "", |ui| {
                    changed |= theme::switch(ui, &mut self.settings.video_cursor).changed();
                });
                let mic = self.settings.audio_device.clone().unwrap_or_else(|| crate::i18n::tr("Aucun").into());
                // Liste rafraîchie automatiquement tant que la page est affichée (micro branché à chaud).
                let stale = self.mic_refresh.is_none_or(|t| t.elapsed() > Duration::from_secs(5));
                let desc = if self.mic_refresh.is_some() && self.audio_devices.is_empty() {
                    crate::i18n::tr("Aucun micro branché ou activé dans Windows • non disponible en GIF")
                } else {
                    crate::i18n::tr("Non disponible pour le format GIF")
                };
                theme::setting_row(
                    ui,
                    crate::i18n::tr("Enregistrer le son du micro"),
                    crate::i18n::tr("Choix par défaut ; modifiable au moment de choisir la zone (touche M)"),
                    |ui| {
                        changed |= theme::switch(ui, &mut self.settings.record_audio).changed();
                    },
                );
                theme::setting_row(ui, crate::i18n::tr("Microphone"), desc, |ui| {
                    let force = theme::icon_button(ui, ICON_REFRESH, false, crate::i18n::tr("Actualiser la liste")).clicked();
                    if force || stale {
                        self.mic_refresh = Some(Instant::now());
                        let tx = self.tx.clone();
                        let c = ctx.clone();
                        std::thread::spawn(move || {
                            let _ = tx.send(AppEvent::AudioDevices(recorder::list_microphones()));
                            c.request_repaint();
                        });
                        ctx.request_repaint_after(Duration::from_secs(5));
                    }
                    egui::ComboBox::from_id_salt("mic").selected_text(mic).width(220.0).show_ui(ui, |ui| {
                        if ui.selectable_label(self.settings.audio_device.is_none(), crate::i18n::tr("Aucun")).clicked() {
                            self.settings.audio_device = None;
                            changed = true;
                        }
                        for d in &self.audio_devices {
                            if ui.selectable_label(self.settings.audio_device.as_deref() == Some(d), d).clicked() {
                                self.settings.audio_device = Some(d.clone());
                                changed = true;
                            }
                        }
                    });
                });
            });
            ui.add_space(12.0);

            // Carte cachée tant que le diagnostic n'a pas été activé (triple-clic sur la version).
            if self.settings.diagnostic {
                theme::card(ui, crate::i18n::tr("Diagnostic"), ICON_BUG_REPORT, |ui| {
                    let path = crate::diag::log_path();
                    theme::setting_row(ui, crate::i18n::tr("Journal de diagnostic"), &path.display().to_string(), |ui| {
                        if theme::switch(ui, &mut self.settings.diagnostic).changed() {
                            crate::diag::set_enabled(self.settings.diagnostic);
                            changed = true;
                        }
                        if theme::button(ui, ButtonKind::Tonal, Some(ICON_FOLDER_OPEN), crate::i18n::tr("Ouvrir le dossier")).clicked() {
                            platform::open_folder(&crate::diag::log_dir());
                        }
                    });
                    ui.label(
                        egui::RichText::new(crate::i18n::tr("En cas de blocage, relancer ScreenCapture depuis le menu Démarrer remplace l'instance bloquée ; le journal indique alors l'étape en cause. Désactiver le journal masque cette section (triple-clic sur la version, page « À propos », pour la retrouver)."))
                            .size(12.0)
                            .color(p.on_surface_variant),
                    );
                });
                ui.add_space(12.0);
            }

            theme::card(ui, crate::i18n::tr("Couleurs de l'éditeur"), ICON_COLOR_LENS, |ui| {
                changed |= self.palette_editor(ui);
            });
            ui.add_space(12.0);

            theme::card(ui, crate::i18n::tr("Apparence et démarrage"), ICON_PALETTE, |ui| {
                theme::setting_row(ui, crate::i18n::tr("Langue"), "", |ui| {
                    for l in crate::i18n::Language::ALL {
                        let label = if l == crate::i18n::Language::System { crate::i18n::tr("Système") } else { l.native_name() };
                        if theme::filter_chip(ui, label, self.settings.language == l).clicked() && self.settings.language != l {
                            self.settings.language = l;
                            crate::i18n::set_language(l);
                            refresh_tray = true;
                            changed = true;
                        }
                    }
                });
                theme::setting_row(ui, crate::i18n::tr("Thème"), "", |ui| {
                    for (v, label) in [(ThemeChoice::Dark, crate::i18n::tr("Sombre")), (ThemeChoice::Light, crate::i18n::tr("Clair")), (ThemeChoice::System, crate::i18n::tr("Système"))] {
                        if theme::filter_chip(ui, label, self.settings.theme == v).clicked() {
                            self.settings.theme = v;
                            changed = true;
                        }
                    }
                });
                theme::setting_row(ui, crate::i18n::tr("Couleur d'accent"), "", |ui| {
                    for a in [Accent::Red, Accent::Orange, Accent::Green, Accent::Blue, Accent::Violet] {
                        let c = theme::Palette::new(false, a).primary;
                        if theme::swatch(ui, c, self.settings.accent == a).clicked() {
                            self.settings.accent = a;
                            changed = true;
                        }
                    }
                });
                theme::setting_row(ui, crate::i18n::tr("Fermer dans la zone de notification"), crate::i18n::tr("La croix masque la fenêtre sans quitter"), |ui| {
                    changed |= theme::switch(ui, &mut self.settings.close_to_tray).changed();
                });
                theme::setting_row(ui, crate::i18n::tr("Démarrer réduit"), crate::i18n::tr("Lancer directement dans la zone de notification"), |ui| {
                    changed |= theme::switch(ui, &mut self.settings.start_in_tray).changed();
                });
            });
            ui.add_space(24.0);
        });
        if refresh_tray {
            self.refresh_tray();
        }
        if changed {
            self.settings.save();
        }
    }

    /// Textes du menu de notification (langue ou raccourcis modifiés).
    fn refresh_tray(&mut self) {
        if let Some(tray) = &self._tray {
            if let Ok(menu) = tray_menu(&mut self.menu_ids, &self.settings) {
                tray.set_menu(Some(Box::new(menu)));
            }
            let _ = tray.set_tooltip(Some(tray_tooltip(&self.settings)));
        }
    }

    /// Nuancier des styles : modifier, ajouter, retirer ou réordonner les couleurs.
    fn palette_editor(&mut self, ui: &mut egui::Ui) -> bool {
        const MAX: usize = 32;
        let mut changed = false;
        let pal = &mut self.settings.palette;
        if pal.is_empty() {
            *pal = crate::editor::model::COLORS.to_vec();
        }
        if self.palette_sel.is_some_and(|i| i >= pal.len()) {
            self.palette_sel = None;
        }
        theme::setting_row(
            ui,
            crate::i18n::tr("Nuancier des styles"),
            crate::i18n::tr("Clic : modifier  •  clic droit : retirer  •  + : ajouter"),
            |ui| {
                if theme::button(ui, theme::ButtonKind::Text, Some(ICON_RESTART_ALT), crate::i18n::tr("Couleurs d'origine")).clicked() {
                    *pal = crate::editor::model::COLORS.to_vec();
                    self.palette_sel = None;
                    changed = true;
                }
            },
        );
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            let mut remove = None;
            for (i, c) in pal.iter().enumerate() {
                let c32 = egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
                let hint = if c[3] == 0 { crate::i18n::tr("Transparent").to_string() } else { format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]) };
                let r = theme::swatch(ui, c32, self.palette_sel == Some(i)).on_hover_text(hint);
                if r.clicked() {
                    self.palette_sel = if self.palette_sel == Some(i) { None } else { Some(i) };
                }
                if r.secondary_clicked() && pal.len() > 1 {
                    remove = Some(i);
                }
            }
            if let Some(i) = remove {
                pal.remove(i);
                self.palette_sel = None;
                changed = true;
            }
            if pal.len() < MAX && theme::icon_button(ui, ICON_ADD, false, crate::i18n::tr("Ajouter une couleur")).clicked() {
                let base = self.palette_sel.and_then(|i| pal.get(i).copied()).unwrap_or([128, 128, 128, 255]);
                pal.push(if base[3] == 0 { [128, 128, 128, 255] } else { base });
                self.palette_sel = Some(pal.len() - 1);
                changed = true;
            }
        });

        if let Some(i) = self.palette_sel {
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                let c = pal[i];
                let mut c32 = egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
                if egui::widgets::color_picker::color_picker_color32(ui, &mut c32, egui::widgets::color_picker::Alpha::OnlyBlend) {
                    pal[i] = c32.to_srgba_unmultiplied();
                    changed = true;
                }
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.label(crate::i18n::trf("Couleur {0} sur {1}", &[&(i + 1), &pal.len()]));
                    let hex_of = |c: [u8; 4]| format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]);
                    let editing = ui.memory(|m| m.focused()).is_some_and(|f| f == egui::Id::new("palette_hex"));
                    if self.palette_hex.0 != i || !editing {
                        self.palette_hex = (i, hex_of(c));
                    }
                    let hex = &mut self.palette_hex.1;
                    let r = ui.add(egui::TextEdit::singleline(hex).id(egui::Id::new("palette_hex")).desired_width(90.0).hint_text("#RRVVBB"));
                    if r.changed() {
                        let h = hex.trim().trim_start_matches('#');
                        if h.len() == 6 {
                            if let Ok(v) = u32::from_str_radix(h, 16) {
                                pal[i] = [(v >> 16) as u8, (v >> 8) as u8, v as u8, 255];
                                changed = true;
                            }
                        }
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if i > 0 && theme::icon_button(ui, ICON_CHEVRON_LEFT, false, crate::i18n::tr("Déplacer à gauche")).clicked() {
                            pal.swap(i, i - 1);
                            self.palette_sel = Some(i - 1);
                            changed = true;
                        }
                        if i + 1 < pal.len() && theme::icon_button(ui, ICON_CHEVRON_RIGHT, false, crate::i18n::tr("Déplacer à droite")).clicked() {
                            pal.swap(i, i + 1);
                            self.palette_sel = Some(i + 1);
                            changed = true;
                        }
                        if pal.len() > 1 && theme::icon_button(ui, ICON_DELETE, false, crate::i18n::tr("Retirer cette couleur")).clicked() {
                            pal.remove(i);
                            self.palette_sel = None;
                            changed = true;
                        }
                    });
                });
            });
        }
        changed
    }

    fn about_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = palette(ctx);
        ui.add_space(8.0);
        ui.label(egui::RichText::new("ScreenCapture").font(theme::medium(28.0)).color(p.on_surface));
        // Triple-clic sur la version : active le journal de diagnostic.
        let version = ui.add(
            egui::Label::new(egui::RichText::new(crate::i18n::trf("Version {0}  •  Rust + egui  •  Material Design 3", &[&env!("CARGO_PKG_VERSION")])).color(p.on_surface_variant))
                .sense(egui::Sense::click())
                .selectable(false),
        );
        ui.label(egui::RichText::new(crate::i18n::tr("Conçu par Stéphane Salou")).color(p.on_surface_variant));
        if version.triple_clicked() && !self.settings.diagnostic {
            self.settings.diagnostic = true;
            self.settings.save();
            crate::diag::set_enabled(true);
            self.toast(crate::i18n::tr("Journal de diagnostic activé (Paramètres → Diagnostic)"));
        }
        ui.add_space(16.0);
        theme::card(ui, crate::i18n::tr("Fonctionnalités"), ICON_STARS, |ui| {
            for (icon, text) in [
                (ICON_SCREENSHOT_REGION, crate::i18n::tr("Capture de zone avec loupe, réticule et pipette, détection automatique des fenêtres")),
                (ICON_SCREENSHOT_MONITOR, crate::i18n::tr("Plein écran multi-moniteurs, fenêtre active, dernière zone, capture différée")),
                (ICON_VIDEOCAM, crate::i18n::tr("Enregistrement vidéo MP4 / WebM / GIF avec micro (via FFmpeg)")),
                (ICON_EDIT, crate::i18n::tr("Éditeur : flèches, formes, surligneur, flou, texte, bulles, numéros, loupe, tampons, rognage, rotation")),
                (ICON_DOCUMENT_SCANNER, crate::i18n::tr("Extraction de texte (OCR Windows)")),
                (ICON_PHOTO_LIBRARY, crate::i18n::tr("Espace de travail avec historique, recherche et filtres")),
                (ICON_KEYBOARD, crate::i18n::tr("Raccourcis clavier globaux configurables, icône de notification")),
            ] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(icon).size(20.0).color(p.primary));
                    ui.label(text);
                });
            }
        });
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(
                crate::i18n::tr("Inspiré de l'ergonomie de Screenpresso, de l'Outil Capture d'écran de Windows, de Greenshot et d'autres outils de capture. Aucune partie de leur code n'est réutilisée."),
            )
            .size(12.5)
            .color(p.on_surface_variant),
        );
    }

    fn workspace_actions(&mut self, ctx: &egui::Context, actions: Vec<WsAction>) {
        for a in actions {
            match a {
                WsAction::Capture(k) => self.start_capture(k, ctx, false),
                WsAction::Edit(path) => match image::open(&path) {
                    Ok(img) => self.open_editor(img.to_rgba8(), Some(path)),
                    Err(e) => self.toast(crate::i18n::trf("Impossible d'ouvrir l'image : {0}", &[&e])),
                },
                WsAction::Copy(path) => {
                    let r = image::open(&path).map_err(anyhow::Error::from).and_then(|i| copy_image(&i.to_rgba8()));
                    match r {
                        Ok(()) => self.toast(crate::i18n::tr("Copié dans le presse-papiers")),
                        Err(e) => self.toast(crate::i18n::trf("Erreur : {0}", &[&e])),
                    }
                }
                WsAction::TogglePin(paths) => self.toggle_pins(&paths),
                WsAction::Delete(paths) => {
                    let mut done = 0;
                    let mut error = None;
                    for path in &paths {
                        match platform::move_to_recycle_bin(path) {
                            Ok(()) => {
                                self.workspace.forget(path);
                                done += 1;
                            }
                            Err(e) => error = Some(e),
                        }
                    }
                    crate::diag::log(&format!("suppression : {done} / {} capture(s) dans la Corbeille", paths.len()));
                    // Une capture supprimée n'a plus à rester épinglée.
                    let gone: Vec<String> = paths.iter().filter(|p| !p.exists()).filter_map(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).collect();
                    if self.settings.pinned.iter().any(|p| gone.contains(p)) {
                        self.settings.pinned.retain(|p| !gone.contains(p));
                        self.settings.save();
                    }
                    match (error, done) {
                        (Some(e), _) => self.toast(crate::i18n::trf("Erreur : {0}", &[&e])),
                        (None, 1) => self.toast(crate::i18n::tr("Envoyé dans la Corbeille")),
                        (None, n) => self.toast(crate::i18n::trf("{0} captures envoyées dans la Corbeille", &[&n])),
                    }
                }
                WsAction::Open(path) => platform::open_with_default(&path),
                WsAction::Reveal(path) => platform::reveal_in_explorer(&path),
                WsAction::OpenFolder => platform::open_folder(&self.settings.save_dir),
                WsAction::Paste => self.paste_clipboard(),
                WsAction::OpenFile => {
                    if let Some(f) = crate::diag::in_dialog(|| rfd::FileDialog::new().add_filter(crate::i18n::tr("Images"), &["png", "jpg", "jpeg", "bmp", "gif"]).pick_file()) {
                        match image::open(&f) {
                            Ok(img) => {
                                let inside = f.parent() == Some(self.settings.save_dir.as_path());
                                self.open_editor(img.to_rgba8(), inside.then_some(f));
                            }
                            Err(e) => self.toast(crate::i18n::trf("Impossible d'ouvrir l'image : {0}", &[&e])),
                        }
                    }
                }
            }
        }
    }

    /// Colle l'image du presse-papiers (ou les fichiers image copiés) dans le dossier des captures.
    fn paste_clipboard(&mut self) {
        crate::diag::step("collage du presse-papiers");
        let Ok(mut cb) = arboard::Clipboard::new() else {
            self.toast(crate::i18n::tr("Presse-papiers indisponible"));
            return;
        };
        if let Ok(img) = cb.get_image() {
            let Some(rgba) = RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.into_owned()) else { return };
            let r = std::fs::create_dir_all(&self.settings.save_dir).map_err(anyhow::Error::from).and_then(|_| {
                let path = self.unique_path("Collage", self.settings.image_format.ext());
                save_image(&rgba, &path, self.settings.jpeg_quality).map(|_| path)
            });
            match r {
                Ok(path) => {
                    self.workspace.refresh();
                    self.page = Page::Home;
                    self.toast(crate::i18n::trf("Image collée : {0}", &[&path.file_name().unwrap_or_default().to_string_lossy()]));
                }
                Err(e) => self.toast(crate::i18n::trf("Enregistrement impossible : {0}", &[&e])),
            }
            return;
        }
        let files = cb.get().file_list().unwrap_or_default();
        if files.is_empty() {
            self.toast(crate::i18n::tr("Le presse-papiers ne contient pas d'image"));
        } else {
            self.import_files(&files);
        }
    }

    /// Copie des fichiers image (collés ou déposés) dans le dossier des captures.
    fn import_files(&mut self, files: &[PathBuf]) {
        let images: Vec<&PathBuf> = files
            .iter()
            .filter(|f| {
                let ext = f.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
                f.is_file() && crate::workspace::IMAGE_EXT.contains(&ext.as_str())
            })
            .collect();
        if images.is_empty() {
            self.toast(crate::i18n::tr("Aucune image parmi les fichiers"));
            return;
        }
        let _ = std::fs::create_dir_all(&self.settings.save_dir);
        let mut copied = 0;
        for f in &images {
            if f.parent() == Some(self.settings.save_dir.as_path()) {
                continue;
            }
            let mut dest = self.settings.save_dir.join(f.file_name().unwrap_or_default());
            let stem = f.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let ext = f.extension().unwrap_or_default().to_string_lossy().to_string();
            let mut n = 2;
            while dest.exists() {
                dest = self.settings.save_dir.join(format!("{stem}_{n}.{ext}"));
                n += 1;
            }
            match std::fs::copy(f, &dest) {
                Ok(_) => copied += 1,
                Err(e) => crate::diag::log(&format!("copie impossible de {}: {e}", f.display())),
            }
        }
        self.workspace.refresh();
        self.page = Page::Home;
        self.toast(match copied {
            0 => crate::i18n::tr("Ces images sont déjà dans le dossier des captures").to_string(),
            1 => crate::i18n::tr("1 image ajoutée au dossier des captures").to_string(),
            n => crate::i18n::trf("{0} images ajoutées au dossier des captures", &[&n]),
        });
    }

    /// Sélection plein écran : fenêtre flottante couvrant tous les écrans.
    fn overlay_viewport(&mut self, ctx: &egui::Context) {
        let Some(origin) = self.overlay.as_ref().map(|o| o.origin) else { return };
        let result = ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("sc_overlay"), floating_builder(TITLE_OVERLAY, origin), |vctx, _| {
            self.overlay_win.sync(TITLE_OVERLAY, origin, true, false, false);
            let closed = vctx.input(|i| i.viewport().close_requested());
            match self.overlay.as_mut() {
                Some(o) if !closed => o.ui(vctx),
                _ => OverlayResult::Cancel,
            }
        });
        match result {
            OverlayResult::Pending => {}
            OverlayResult::CopyColor(hex) => {
                let _ = arboard::Clipboard::new().and_then(|mut c| c.set_text(hex.clone()));
                self.overlay = None;
                self.end_capture();
                self.toast(crate::i18n::trf("Couleur {0} copiée", &[&hex]));
            }
            OverlayResult::Cancel => {
                crate::diag::log("sélection annulée");
                self.overlay = None;
                self.end_capture();
            }
            OverlayResult::Selected(r) => {
                crate::diag::log(&format!("zone sélectionnée {r:?}"));
                let ov = self.overlay.take().expect("overlay");
                match ov.purpose {
                    Purpose::Image => {
                        self.settings.last_region = Some(r);
                        self.settings.save();
                        match capture::crop_virtual(&ov.image, ov.origin, r) {
                            Some(img) => self.finish_capture(ctx, img),
                            None => self.end_capture(),
                        }
                    }
                    Purpose::Video => {
                        // Choix « avec / sans le son » fait dans la sélection : gardé pour la suite.
                        if let Some(a) = ov.audio {
                            if a != self.settings.record_audio {
                                self.settings.record_audio = a;
                                self.settings.save();
                            }
                        }
                        self.start_recording(ctx, r)
                    }
                }
            }
        }
    }

    /// Nettoyage de conservation, en tâche de fond. La capture ouverte dans l'éditeur est protégée.
    fn run_retention(&mut self, ctx: &egui::Context, manual: bool) {
        if self.retention_running || (!manual && !crate::retention::enabled(&self.settings)) {
            return;
        }
        self.retention_running = true;
        let settings = self.settings.clone();
        let keep = self.editor.as_ref().and_then(|e| e.path.clone());
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let plan = crate::retention::plan(&settings, keep.as_deref());
            let (n, bytes, errors) = crate::retention::execute(&plan, settings.retention_to_trash);
            let _ = tx.send(AppEvent::RetentionDone { n, bytes, errors, manual });
            ctx.request_repaint();
        });
    }

    /// Épingle les captures, ou les désépingle si elles le sont toutes.
    fn toggle_pins(&mut self, paths: &[PathBuf]) {
        let names: Vec<String> = paths.iter().filter_map(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).collect();
        let all_pinned = names.iter().all(|n| self.settings.pinned.contains(n));
        if all_pinned {
            self.settings.pinned.retain(|p| !names.contains(p));
        } else {
            for n in names.iter() {
                if !self.settings.pinned.contains(n) {
                    self.settings.pinned.push(n.clone());
                }
            }
        }
        self.settings.save();
        self.retention_preview = None;
        crate::diag::log(&format!("{} : {}", if all_pinned { "désépinglées" } else { "épinglées" }, names.join(", ")));
        self.toast(match (all_pinned, names.len()) {
            (false, 1) => crate::i18n::tr("Capture épinglée : elle ne sera jamais supprimée automatiquement").to_string(),
            (false, n) => crate::i18n::trf("{0} captures épinglées", &[&n]),
            (true, 1) => crate::i18n::tr("Capture désépinglée").to_string(),
            (true, n) => crate::i18n::trf("{0} captures désépinglées", &[&n]),
        });
    }

    /// Carte « Conservation des captures » des Paramètres. Renvoie `true` si un réglage a changé.
    fn retention_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> bool {
        let p = palette(ctx);
        let mut changed = false;
        let before = (self.settings.retention_images_days, self.settings.retention_videos_days, self.settings.retention_max_mb);
        let was_enabled = crate::retention::enabled(&self.settings);
        theme::card(ui, crate::i18n::tr("Conservation des captures"), ICON_AUTO_DELETE, |ui| {
            theme::setting_row(ui, crate::i18n::tr("Supprimer les images après"), "", |ui| {
                changed |= days_picker(ui, "ret_img", &mut self.settings.retention_images_days);
            });
            theme::setting_row(ui, crate::i18n::tr("Supprimer les vidéos après"), "", |ui| {
                changed |= days_picker(ui, "ret_vid", &mut self.settings.retention_videos_days);
            });
            theme::setting_row(ui, crate::i18n::tr("Taille maximale du dossier"), crate::i18n::tr("Au-delà, les captures les plus anciennes sont supprimées"), |ui| {
                let mut on = self.settings.retention_max_mb > 0;
                if on {
                    let mut gb = self.settings.retention_max_mb as f64 / 1024.0;
                    if ui.add(egui::DragValue::new(&mut gb).range(0.1..=10_000.0).speed(0.1).max_decimals(1).suffix(" Go")).changed() {
                        self.settings.retention_max_mb = ((gb * 1024.0).round() as u64).max(100);
                        changed = true;
                    }
                }
                if theme::switch(ui, &mut on).changed() {
                    self.settings.retention_max_mb = if on { 5 * 1024 } else { 0 };
                    changed = true;
                }
            });
            theme::setting_row(ui, crate::i18n::tr("Mode de suppression"), "", |ui| {
                for (v, label) in [(false, crate::i18n::tr("Définitive")), (true, crate::i18n::tr("Corbeille (récupérable)"))] {
                    if theme::filter_chip(ui, label, self.settings.retention_to_trash == v).clicked() && self.settings.retention_to_trash != v {
                        self.settings.retention_to_trash = v;
                        changed = true;
                    }
                }
            });
            // Aperçu, recalculé quand les réglages changent (et au plus toutes les 3 s).
            let key = format!(
                "{}|{}|{}|{}|{}",
                self.settings.retention_images_days, self.settings.retention_videos_days, self.settings.retention_max_mb, self.settings.pinned.len(), self.settings.save_dir.display()
            );
            let stale = self.retention_preview.as_ref().is_none_or(|(k, at, _)| *k != key || at.elapsed() > Duration::from_secs(3));
            if stale {
                let keep = self.editor.as_ref().and_then(|e| e.path.clone());
                self.retention_preview = Some((key, Instant::now(), crate::retention::plan(&self.settings, keep.as_deref())));
            }
            let plan = self.retention_preview.as_ref().map(|(_, _, p)| p.clone()).unwrap_or_default();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let text = if !crate::retention::enabled(&self.settings) {
                    crate::i18n::tr("Aucune suppression automatique.").to_string()
                } else if plan.is_empty() {
                    crate::i18n::tr("Rien à supprimer pour l'instant. Vérification au démarrage puis toutes les heures.").to_string()
                } else {
                    let by_size = plan.files.iter().filter(|f| f.reason == crate::retention::Reason::Size).count();
                    let detail = match by_size {
                        0 => String::new(),
                        n if n == plan.files.len() => crate::i18n::tr(" (taille maximale dépassée)").to_string(),
                        n => crate::i18n::trf(" (dont {0} pour la taille maximale)", &[&n]),
                    };
                    crate::i18n::trf("{0} seraient supprimées maintenant{1}.", &[&crate::retention::describe(plan.files.len(), plan.bytes()), &detail])
                };
                ui.label(egui::RichText::new(text).color(if plan.is_empty() { p.on_surface_variant } else { p.primary }));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::button(ui, ButtonKind::Tonal, Some(ICON_AUTO_DELETE), crate::i18n::tr("Nettoyer maintenant")).clicked() && !plan.is_empty() {
                        self.run_retention(ctx, true);
                    }
                });
            });
            ui.label(
                egui::RichText::new(crate::i18n::trf("Seuls les fichiers créés par ScreenCapture (Capture_…, Video_…, Collage_…) sont concernés. Les captures épinglées ({0} sur une vignette) et celle ouverte dans l'éditeur ne sont jamais supprimées. L'âge se compte depuis la création du fichier.", &[&ICON_KEEP]))
                .size(12.0)
                .color(p.on_surface_variant),
            );
            // Un réglage qui supprimerait des captures dès maintenant demande confirmation.
            let after = (self.settings.retention_images_days, self.settings.retention_videos_days, self.settings.retention_max_mb);
            if after != before && self.retention_confirm.is_none() {
                let newly_enabled_or_stricter = !was_enabled || after != before;
                if newly_enabled_or_stricter && !plan.is_empty() {
                    self.retention_confirm = Some(before);
                }
            }
        });
        if let Some(old) = self.retention_confirm {
            let plan = self.retention_preview.as_ref().map(|(_, _, p)| p.clone()).unwrap_or_default();
            let body = crate::i18n::trf("Avec ce réglage, {0} seront {1} dès maintenant.", &[&crate::retention::describe(plan.files.len(), plan.bytes()), &if self.settings.retention_to_trash { crate::i18n::tr("envoyées dans la Corbeille") } else { crate::i18n::tr("supprimées définitivement") }]);
            if let Some(ok) = theme::confirm_dialog(ctx, "retention_confirm", ICON_AUTO_DELETE, crate::i18n::tr("Appliquer la conservation ?"), &body, crate::i18n::tr("Appliquer")) {
                self.retention_confirm = None;
                if ok {
                    self.run_retention(ctx, true);
                } else {
                    (self.settings.retention_images_days, self.settings.retention_videos_days, self.settings.retention_max_mb) = old;
                    self.retention_preview = None;
                }
                changed = true;
            }
        }
        changed
    }

    /// Bouton micro de la barre d'enregistrement (et commande `mic`) : le choix est gardé pour la suite.
    fn toggle_recording_mic(&mut self) {
        let Some(rec) = self.recorder.as_mut() else { return };
        let on = !rec.with_audio;
        rec.set_audio(on);
        if rec.with_audio == on {
            self.settings.record_audio = on;
            self.settings.save();
        }
    }

    /// Barre d'enregistrement : fenêtre flottante exclue de la vidéo.
    fn recording_bar_viewport(&mut self, ctx: &egui::Context) {
        if self.recorder.is_none() {
            return;
        }
        let rect = self.recbar_rect;
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("sc_recbar"), floating_builder(TITLE_RECBAR, rect), |vctx, _| {
            self.recbar_win.sync(TITLE_RECBAR, rect, false, true, true);
            self.recording_bar(vctx);
        });
    }

    fn recording_bar(&mut self, ctx: &egui::Context) {
        let p = palette(ctx);
        let elapsed = self.recorder.as_ref().map(|r| r.started.elapsed()).unwrap_or_default();
        let mut stop = false;
        let mut cancel = false;
        let mut toggle_mic = false;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.surface_high).corner_radius(CornerRadius::same(0)).inner_margin(Margin::symmetric(14, 10)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let t = ctx.input(|i| i.time);
                    let a = (0.55 + 0.45 * (t * 3.0).sin()) as f32;
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 7.0, Color32::from_rgb(229, 57, 53).gamma_multiply(a));
                    let s = elapsed.as_secs();
                    ui.label(egui::RichText::new(format!("{:02}:{:02}", s / 60, s % 60)).font(theme::medium(18.0)).color(p.on_surface));
                    if let Some(r) = &self.recorder {
                        ui.label(egui::RichText::new(format!("{}×{}", r.region.w, r.region.h)).size(12.0).color(p.on_surface_variant));

                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        stop = theme::button(ui, ButtonKind::Danger, Some(ICON_STOP_CIRCLE), crate::i18n::tr("Arrêter")).clicked();
                        cancel = theme::icon_button(ui, ICON_DELETE, false, crate::i18n::tr("Annuler l'enregistrement")).clicked();
                        // Micro : activable ou coupé à tout moment pendant l'enregistrement.
                        if let Some(r) = &self.recorder {
                            if r.audio_available {
                                let (icon, tip) = if r.with_audio { (ICON_MIC, crate::i18n::tr("Son activé : cliquer pour couper le micro")) } else { (ICON_MIC_OFF, crate::i18n::tr("Son coupé : cliquer pour activer le micro")) };
                                toggle_mic = theme::icon_button(ui, icon, r.with_audio, tip).clicked();
                            } else {
                                ui.label(egui::RichText::new(ICON_MIC_OFF).size(18.0).color(p.outline)).on_hover_text(crate::i18n::tr("Sans le son : aucun micro choisi dans les Paramètres (ou GIF)"));
                            }
                        }
                    });
                });
            });
        ctx.request_repaint_after(Duration::from_millis(100));
        if toggle_mic {
            self.toggle_recording_mic();
        }
        if stop {
            self.stop_recording(ctx);
        } else if cancel {
            self.cancel_recording(ctx);
        }
    }

    fn snackbar_ui(&mut self, ctx: &egui::Context) {
        let Some((msg, at)) = self.snackbar.clone() else { return };
        if at.elapsed() > Duration::from_secs(4) {
            self.snackbar = None;
            return;
        }
        let p = palette(ctx);
        egui::Area::new(egui::Id::new("snackbar"))
            .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -24.0))
            .order(egui::Order::Tooltip)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.inverse_surface)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(16, 12))
                    .shadow(egui::epaint::Shadow { offset: [0, 3], blur: 10, spread: 0, color: Color32::from_black_alpha(60) })
                    .show(ui, |ui| {
                        // Sans largeur imposée, la bulle se dimensionne à la première image et
                        // le texte passait à la ligne après chaque mot.
                        ui.add(egui::Label::new(egui::RichText::new(msg).color(p.inverse_on_surface)).extend());
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(250));
    }

    fn ocr_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut text) = self.ocr_text.clone() else { return };
        let mut open = true;
        let mut close = false;
        egui::Window::new(crate::i18n::tr("Texte extrait"))
            .collapsible(false)
            .default_width(520.0)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(crate::i18n::tr("Le texte a été copié dans le presse-papiers.")).color(palette(ctx).on_surface_variant));
                ui.add_space(8.0);
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut text).desired_width(f32::INFINITY).desired_rows(10));
                });
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::button(ui, ButtonKind::Filled, Some(ICON_CONTENT_COPY), crate::i18n::tr("Copier")).clicked() {
                        let _ = arboard::Clipboard::new().and_then(|mut c| c.set_text(text.clone()));
                        close = true;
                    }
                    if theme::button(ui, ButtonKind::Text, None, crate::i18n::tr("Fermer")).clicked() {
                        close = true;
                    }
                });
            });
        self.ocr_text = if !open || close { None } else { Some(text) };
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::diag::heartbeat();
        self.frame += 1;
        platform::CAPTURE_ACTIVE.store(self.busy || self.overlay.is_some(), std::sync::atomic::Ordering::SeqCst);
        self.apply_theme_if_needed(ctx);
        // eframe rend la fenêtre visible après la première image en réappliquant ses styles :
        // le masquage du démarrage se fait donc un peu après.
        if self.park_on_start && self.frame >= 3 {
            self.park_on_start = false;
            self.hide_root();
        } else if self.park_on_start {
            ctx.request_repaint();
        }
        self.handle_events(ctx);
        if self.shown {
            self.size_keeper.update(ctx, self.win);
        }
        // Écran débranché alors que la fenêtre y était affichée : la ramener (vérifié toutes les 2 s).
        if self.shown && self.screen_check.elapsed() > Duration::from_secs(2) {
            self.screen_check = Instant::now();
            crate::diag::step("vérification des écrans");
            self.win.rescue();
            ctx.request_repaint_after(Duration::from_secs(2));
        }

        // Fermeture : masquer dans la zone de notification.
        if ctx.input(|i| i.viewport().close_requested()) && !self.quit {
            if self.settings.close_to_tray {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                if self.mode == Mode::Editor {
                    self.close_editor(true);
                }
                self.hide_root();
            } else {
                self.request_quit(ctx);
            }
        }

        // Conservation : nettoyage 15 s après le démarrage puis toutes les heures.
        if Instant::now() >= self.retention_due {
            self.retention_due = Instant::now() + Duration::from_secs(3600);
            if self.retention_confirm.is_none() && !self.busy && self.overlay.is_none() {
                self.run_retention(ctx, false);
            }
        }
        ctx.request_repaint_after(self.retention_due.saturating_duration_since(Instant::now()));

        // Fenêtres flottantes, indépendantes de la fenêtre principale.
        crate::diag::step("fenêtres flottantes");
        self.overlay_viewport(ctx);
        self.capture_popup_viewport(ctx);
        self.recording_bar_viewport(ctx);

        crate::diag::step(match self.mode {
            Mode::Editor => "interface : éditeur",
            Mode::Workspace => "interface : espace de travail",
        });
        match self.mode {
            Mode::Editor => {
                let mut settings = std::mem::take(&mut self.settings);
                let actions = self.editor.as_mut().map(|e| e.ui(ctx, &mut settings)).unwrap_or_default();
                self.settings = settings;
                if let Some(e) = &self.editor {
                    self.set_title(ctx, crate::i18n::trf("{0} — ScreenCapture", &[&e.title()]));
                }
                self.editor_actions(ctx, actions);
            }
            Mode::Workspace => {
                self.set_title(ctx, "ScreenCapture".into());
                // Ctrl+V : egui le transforme en collage de texte et n'émet rien quand le presse-papiers
                // ne contient qu'une image ; on lit donc l'état de la touche V.
                let (command, focused, text_paste) =
                    ctx.input(|i| (i.modifiers.command, i.focused, i.events.iter().any(|e| matches!(e, egui::Event::Paste(_)))));
                let paste = command && focused && (platform::is_v_key_down() || text_paste);
                if paste && !self.paste_down && !ctx.wants_keyboard_input() {
                    self.paste_clipboard();
                }
                self.paste_down = paste;
                // Fichiers glissés depuis l'Explorateur / le Finder.
                let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
                if !dropped.is_empty() {
                    self.import_files(&dropped);
                }
                self.nav_rail(ctx);
                let p = palette(ctx);
                let mut ws_actions = Vec::new();
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(p.surface).inner_margin(Margin { left: 28, right: 20, top: 20, bottom: 0 }))
                    .show(ctx, |ui| match self.page {
                        Page::Home => {
                            let status = self.hotkeys.status.clone();
                            ws_actions = self.workspace.ui(ui, &self.settings, &status);
                        }
                        Page::Settings => self.settings_page(ui, ctx),
                        Page::About => self.about_page(ui, ctx),
                    });
                self.workspace_actions(ctx, ws_actions);
            }
        }

        self.ocr_dialog(ctx);
        self.snackbar_ui(ctx);
        if self.finishing_video {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
        // Après `update`, eframe dessine et présente l'image (pilote graphique), puis attend les évènements.
        crate::diag::step("après update : rendu graphique ou attente d'évènements");
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }
}

/// Menu de la zone de notification (identifiants fixes : il est reconstruit quand la langue
/// ou les raccourcis changent).
fn tray_menu(ids: &mut HashMap<String, &'static str>, settings: &Settings) -> anyhow::Result<Menu> {
    let menu = Menu::new();
    let hk = &settings.hotkeys;
    let entries: [(&'static str, String); 6] = [
        ("region", crate::i18n::trf("Capturer une zone\t{0}", &[&pretty_hotkey(&hk.region)])),
        ("full", crate::i18n::trf("Plein écran\t{0}", &[&pretty_hotkey(&hk.fullscreen)])),
        ("window", crate::i18n::trf("Fenêtre\t{0}", &[&pretty_hotkey(&hk.window)])),
        ("last", crate::i18n::trf("Dernière zone\t{0}", &[&pretty_hotkey(&hk.last_region)])),
        ("delayed", crate::i18n::trf("Capture différée ({0} s)", &[&settings.delay_seconds])),
        ("video", crate::i18n::trf("Enregistrer une vidéo\t{0}", &[&pretty_hotkey(&hk.video)])),
    ];
    for (key, label) in entries {
        let item = MenuItem::with_id(key, label, true, None);
        ids.insert(key.to_string(), key);
        menu.append(&item)?;
    }
    menu.append(&PredefinedMenuItem::separator())?;
    for (key, label) in [("open", crate::i18n::tr("Ouvrir l'espace de travail")), ("folder", crate::i18n::tr("Ouvrir le dossier des captures")), ("settings", crate::i18n::tr("Paramètres"))] {
        let item = MenuItem::with_id(key, label, true, None);
        ids.insert(key.to_string(), key);
        menu.append(&item)?;
    }
    menu.append(&PredefinedMenuItem::separator())?;
    let quit = MenuItem::with_id("quit", crate::i18n::tr("Quitter"), true, None);
    ids.insert("quit".to_string(), "quit");
    menu.append(&quit)?;
    Ok(menu)
}

fn tray_tooltip(settings: &Settings) -> String {
    crate::i18n::trf("ScreenCapture — {0} pour capturer", &[&pretty_hotkey(&settings.hotkeys.region)])
}

fn build_tray(ids: &mut HashMap<String, &'static str>, settings: &Settings) -> anyhow::Result<TrayIcon> {
    let menu = tray_menu(ids, settings)?;

    let rgba = crate::icon::app_icon(32);
    let icon = tray_icon::Icon::from_rgba(rgba, 32, 32)?;
    Ok(TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        // Sur Mac, un clic sur l'icône de la barre des menus ouvre le menu, comme partout ailleurs.
        .with_menu_on_left_click(cfg!(target_os = "macos"))
        .with_tooltip(tray_tooltip(settings))
        .with_icon(icon)
        .build()?)
}

pub fn copy_image(img: &RgbaImage) -> anyhow::Result<()> {
    crate::diag::step("copie dans le presse-papiers");
    let mut cb = arboard::Clipboard::new()?;
    cb.set_image(arboard::ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: std::borrow::Cow::Borrowed(img.as_raw()),
    })?;
    Ok(())
}

pub fn save_image(img: &RgbaImage, path: &Path, jpeg_quality: u8) -> anyhow::Result<()> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if ext == "jpg" || ext == "jpeg" {
        // Le JPEG n'a pas de transparence : composition sur fond blanc.
        let mut rgb = image::RgbImage::new(img.width(), img.height());
        for (x, y, p) in img.enumerate_pixels() {
            let a = p[3] as u32;
            let mix = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
            rgb.put_pixel(x, y, image::Rgb([mix(p[0]), mix(p[1]), mix(p[2])]));
        }
        let file = std::io::BufWriter::new(std::fs::File::create(path)?);
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(file, jpeg_quality);
        enc.encode_image(&rgb)?;
    } else {
        img.save(path)?;
    }
    Ok(())
}

/// Thread relais : transmet les évènements et réveille l'interface, pour les appelants qui
/// ne doivent jamais attendre le verrou du contexte egui (hook clavier).
fn relay(tx: Sender<AppEvent>, ctx: egui::Context) -> Sender<AppEvent> {
    let (relay_tx, relay_rx) = channel::<AppEvent>();
    let _ = std::thread::Builder::new().name("relais".into()).spawn(move || {
        for ev in relay_rx {
            if tx.send(ev).is_err() {
                break;
            }
            ctx.request_repaint();
        }
    });
    relay_tx
}

/// Durée de conservation : Jamais, 7 / 30 / 90 jours, 1 an ou un nombre de jours personnalisé.
fn days_picker(ui: &mut egui::Ui, id: &str, days: &mut u32) -> bool {
    let presets: [(u32, &str); 5] = [(0, crate::i18n::tr("Jamais")), (7, crate::i18n::tr("7 jours")), (30, crate::i18n::tr("30 jours")), (90, crate::i18n::tr("90 jours")), (365, crate::i18n::tr("1 an"))];
    let mut changed = false;
    let custom = !presets.iter().any(|(d, _)| d == days);
    if custom {
        changed |= ui.add(egui::DragValue::new(days).range(1..=3650).suffix(" jours")).changed();
    }
    let label = presets.iter().find(|(d, _)| d == days).map(|(_, l)| l.to_string()).unwrap_or_else(|| crate::i18n::tr("Personnalisée").into());
    egui::ComboBox::from_id_salt(id).selected_text(label).width(140.0).show_ui(ui, |ui| {
        for (d, l) in presets {
            if ui.selectable_label(*days == d && !custom, l).clicked() && *days != d {
                *days = d;
                changed = true;
            }
        }
        if ui.selectable_label(custom, crate::i18n::tr("Personnalisée…")).clicked() && !custom {
            *days = 14;
            changed = true;
        }
    });
    changed
}
