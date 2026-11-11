//! Enregistrement vidéo de l'écran via FFmpeg : gdigrab + micro DirectShow sous Windows,
//! avfoundation (écran et micro) sous macOS.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::Instant;

use crate::capture::Rect;
use crate::platform::hidden_command;
use crate::settings::{Settings, VideoFormat};

#[cfg(windows)]
const FFMPEG: &str = "ffmpeg.exe";
#[cfg(not(windows))]
const FFMPEG: &str = "ffmpeg";

/// Commande d'installation conseillée dans les messages.
#[cfg(windows)]
pub const FFMPEG_HINT: &str = "winget install ffmpeg";
#[cfg(target_os = "macos")]
pub const FFMPEG_HINT: &str = "brew install ffmpeg";

/// Recherche FFmpeg : chemin configuré, dossier de l'exécutable, PATH, puis installations connues.
pub fn find_ffmpeg(settings: &Settings) -> Option<PathBuf> {
    let configured = settings.ffmpeg_path.trim();
    if !configured.is_empty() && Path::new(configured).is_file() {
        return Some(configured.into());
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for c in [dir.join(FFMPEG), dir.join("ffmpeg").join(FFMPEG)] {
                if c.is_file() {
                    return Some(c);
                }
            }
        }
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&paths) {
            let c = d.join(FFMPEG);
            if c.is_file() {
                return Some(c);
            }
        }
    }
    known_ffmpeg()
}

/// Une application lancée depuis le Finder n'hérite pas du PATH du terminal : Homebrew et MacPorts.
#[cfg(target_os = "macos")]
fn known_ffmpeg() -> Option<PathBuf> {
    ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg", "/opt/local/bin/ffmpeg"]
        .into_iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
}

#[cfg(windows)]
fn known_ffmpeg() -> Option<PathBuf> {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    // Paquets winget (Gyan.FFmpeg, BtbN…) : le PATH du processus peut précéder l'installation.
    if let Ok(rd) = std::fs::read_dir(format!("{local}\\Microsoft\\WinGet\\Packages")) {
        for pkg in rd.flatten().filter(|e| e.file_name().to_string_lossy().to_lowercase().contains("ffmpeg")) {
            for build in std::fs::read_dir(pkg.path()).into_iter().flatten().flatten() {
                let c = build.path().join("bin").join("ffmpeg.exe");
                if c.is_file() {
                    return Some(c);
                }
            }
        }
    }
    [
        format!("{local}\\Microsoft\\WinGet\\Links\\ffmpeg.exe"),
        "C:\\ffmpeg\\bin\\ffmpeg.exe".to_string(),
        "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe".to_string(),
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|p| p.is_file())
}

/// Micros actifs (branchés et activés) selon Windows (Core Audio), sans passer par FFmpeg.
#[cfg(windows)]
pub fn list_microphones() -> Vec<String> {
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::{eCapture, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ};
    // Appelée depuis un thread de travail : appartement COM multithread.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else {
            return Vec::new();
        };
        let Ok(col) = en.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE) else {
            return Vec::new();
        };
        (0..col.GetCount().unwrap_or(0))
            .filter_map(|i| {
                let store = col.Item(i).ok()?.OpenPropertyStore(STGM_READ).ok()?;
                let name = store.GetValue(&PKEY_Device_FriendlyName).ok()?.to_string();
                (!name.is_empty()).then_some(name)
            })
            .collect()
    }
}

/// Micros vus par FFmpeg (avfoundation), qui sert aussi à l'enregistrement.
#[cfg(target_os = "macos")]
pub fn list_microphones() -> Vec<String> {
    find_ffmpeg(&Settings::load()).map(|f| list_audio_devices(&f)).unwrap_or_default()
}

/// Nom FFmpeg correspondant au micro choisi (DirectShow tronque parfois les noms longs).
fn resolve_dshow_audio(ffmpeg: &Path, name: &str) -> Option<String> {
    let devices = list_audio_devices(ffmpeg);
    devices
        .iter()
        .find(|d| d.as_str() == name)
        .or_else(|| devices.iter().find(|d| name.starts_with(d.as_str()) || d.starts_with(name)))
        .cloned()
}

/// Liste les périphériques audio DirectShow.
#[cfg(windows)]
pub fn list_audio_devices(ffmpeg: &Path) -> Vec<String> {
    let out = hidden_command(&ffmpeg.to_string_lossy())
        .args(["-hide_banner", "-list_devices", "true", "-f", "dshow", "-i", "dummy"])
        .stdin(Stdio::null())
        .output();
    let Ok(out) = out else { return Vec::new() };
    let text = String::from_utf8_lossy(&out.stderr);
    text.lines()
        .filter(|l| l.contains("(audio)"))
        .filter_map(|l| {
            let s = l.find('"')?;
            let e = l[s + 1..].find('"')?;
            Some(l[s + 1..s + 1 + e].to_string())
        })
        .collect()
}

/// Liste les micros avfoundation : lignes « [n] Nom » après « AVFoundation audio devices: ».
#[cfg(target_os = "macos")]
pub fn list_audio_devices(ffmpeg: &Path) -> Vec<String> {
    let out = hidden_command(&ffmpeg.to_string_lossy())
        .args(["-hide_banner", "-f", "avfoundation", "-list_devices", "true", "-i", ""])
        .stdin(Stdio::null())
        .output();
    let Ok(out) = out else { return Vec::new() };
    let text = String::from_utf8_lossy(&out.stderr);
    text.lines()
        .skip_while(|l| !l.contains("audio devices"))
        .skip(1)
        .filter_map(|l| {
            let rest = l.split("] [").nth(1)?;
            let (_, name) = rest.split_once("] ")?;
            Some(name.trim().to_string())
        })
        .collect()
}

pub struct Recorder {
    child: Child,
    temp: PathBuf,
    pub output: PathBuf,
    format: VideoFormat,
    ffmpeg: PathBuf,
    pub started: Instant,
    pub region: Rect,
    /// Avertissement à afficher au démarrage (ex. micro introuvable).
    pub warning: Option<String>,
    /// Le son du micro est enregistré (sinon il est à zéro).
    pub with_audio: bool,
    /// Un micro est ouvert : le son peut être activé ou coupé pendant l'enregistrement.
    pub audio_available: bool,
}

impl Recorder {
    pub fn start(settings: &Settings, region: Rect, output: PathBuf) -> anyhow::Result<Self> {
        let ffmpeg = find_ffmpeg(settings).ok_or_else(|| {
            anyhow::anyhow!(crate::i18n::trf("FFmpeg introuvable. Installez-le ({0}) ou indiquez son chemin dans les paramètres.", &[&FFMPEG_HINT]))
        })?;
        // x264 / VP9 exigent des dimensions paires.
        let region = Rect::new(region.x, region.y, region.w & !1, region.h & !1);
        anyhow::ensure!(region.w >= 16 && region.h >= 16, crate::i18n::tr("zone trop petite pour une vidéo"));
        let temp = if settings.video_format == VideoFormat::Gif { output.with_extension("tmp.mp4") } else { output.clone() };
        let fps = settings.video_fps.clamp(5, 60).to_string();

        let mut cmd = hidden_command(&ffmpeg.to_string_lossy());
        cmd.args(["-hide_banner", "-loglevel", "error", "-y"]);
        // Micro débranché ou introuvable : enregistrer sans le son plutôt qu'échouer.
        let mut warning = None;
        // Le micro choisi est ouvert même « sans le son » : le son est alors à zéro et peut être
        // rétabli pendant l'enregistrement (filtre volume piloté en direct, voir `set_audio`).
        let audio = settings.audio_device.as_ref().filter(|_| settings.video_format != VideoFormat::Gif).and_then(|name| {
            let dev = resolve_dshow_audio(&ffmpeg, name);
            if dev.is_none() {
                warning = Some(crate::i18n::trf("Micro « {0} » introuvable : enregistrement sans le son", &[&name]));
            }
            dev
        });
        crate::diag::log(&format!(
            "enregistrement vidéo {region:?} : {}",
            match (&audio, &warning) {
                (Some(dev), _) if settings.record_audio => format!("avec le son du micro « {dev} »"),
                (Some(dev), _) => format!("sans le son (micro « {dev} » coupé, activable pendant l'enregistrement)"),
                (None, Some(w)) => w.clone(),
                (None, None) => "sans le son".to_string(),
            }
        ));
        add_inputs(&mut cmd, settings, region, &fps, audio.as_deref())?;
        if audio.is_some() {
            let volume = if settings.record_audio { 1 } else { 0 };
            cmd.args(["-filter:a", &format!("volume=volume={volume}:eval=frame")]);
        }
        match settings.video_format {
            VideoFormat::Webm => {
                cmd.args(["-c:v", "libvpx-vp9", "-deadline", "realtime", "-cpu-used", "8", "-b:v", "0", "-crf", "34", "-pix_fmt", "yuv420p"]);
                if audio.is_some() {
                    cmd.args(["-c:a", "libopus"]);
                }
            }
            _ => {
                cmd.args(["-c:v", "libx264", "-preset", "ultrafast", "-crf", "23", "-pix_fmt", "yuv420p"]);
                if audio.is_some() {
                    cmd.args(["-c:a", "aac", "-b:a", "160k"]);
                }
            }
        }
        cmd.arg(&temp);
        cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
        if let Some(dir) = output.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let child = cmd.spawn()?;
        let audio_available = audio.is_some();
        let with_audio = audio_available && settings.record_audio;
        Ok(Self { child, temp, output, format: settings.video_format, ffmpeg, started: Instant::now(), region, warning, with_audio, audio_available })
    }

    /// Arrête proprement l'enregistrement (touche « q ») et convertit en GIF si nécessaire.
    /// Bloquant : à appeler depuis un thread de travail.
    /// Active ou coupe le son du micro pendant l'enregistrement (commande envoyée à FFmpeg).
    pub fn set_audio(&mut self, on: bool) {
        if !self.audio_available || on == self.with_audio {
            return;
        }
        if let Some(stdin) = self.child.stdin.as_mut() {
            let cmd = format!("cvolume -1 volume {}\n", if on { 1 } else { 0 });
            let ok = stdin.write_all(cmd.as_bytes()).and_then(|_| stdin.flush()).is_ok();
            crate::diag::log(&format!("micro {} pendant l'enregistrement : {}", if on { "activé" } else { "coupé" }, if ok { "ok" } else { "échec" }));
            if ok {
                self.with_audio = on;
            }
        }
    }

    pub fn finish(mut self) -> anyhow::Result<PathBuf> {
        if let Some(stdin) = self.child.stdin.as_mut() {
            let _ = stdin.write_all(b"q");
            let _ = stdin.flush();
        }
        drop(self.child.stdin.take());
        let status = self.child.wait()?;
        anyhow::ensure!(self.temp.is_file(), "{}", crate::i18n::trf("FFmpeg n'a produit aucun fichier (code {0})", &[&status]));
        if self.format == VideoFormat::Gif {
            let filter = "fps=15,scale='min(iw,1280)':-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4";
            let st = hidden_command(&self.ffmpeg.to_string_lossy())
                .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
                .arg(&self.temp)
                .args(["-vf", filter])
                .arg(&self.output)
                .stdin(Stdio::null())
                .status()?;
            let _ = std::fs::remove_file(&self.temp);
            anyhow::ensure!(st.success(), crate::i18n::tr("conversion GIF échouée"));
        }
        Ok(self.output)
    }

    pub fn abort(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.temp);
    }
}

/// Entrées FFmpeg sous Windows : bureau (gdigrab) et micro DirectShow.
#[cfg(windows)]
fn add_inputs(cmd: &mut std::process::Command, settings: &Settings, region: Rect, fps: &str, audio: Option<&str>) -> anyhow::Result<()> {
    cmd.args(["-thread_queue_size", "512", "-f", "gdigrab", "-framerate", fps]);
    cmd.args(["-draw_mouse", if settings.video_cursor { "1" } else { "0" }]);
    if settings.video_show_region {
        cmd.args(["-show_region", "1"]);
    }
    cmd.args(["-offset_x", &region.x.to_string(), "-offset_y", &region.y.to_string()]);
    cmd.args(["-video_size", &format!("{}x{}", region.w, region.h), "-i", "desktop"]);
    if let Some(dev) = audio {
        cmd.args(["-thread_queue_size", "512", "-f", "dshow", "-i", &format!("audio={dev}")]);
    }
    Ok(())
}

/// Entrée FFmpeg sous macOS : écran entier et micro (avfoundation), puis recadrage sur la zone.
#[cfg(target_os = "macos")]
fn add_inputs(cmd: &mut std::process::Command, settings: &Settings, region: Rect, fps: &str, audio: Option<&str>) -> anyhow::Result<()> {
    use crate::capture::imp::{display_scale, displays, scale};
    let (cx, cy) = (region.x + region.w / 2, region.y + region.h / 2);
    let all = displays();
    let index = all.iter().position(|(_, r)| r.contains(cx, cy)).unwrap_or(0);
    let (display, screen) = all.get(index).copied().ok_or_else(|| anyhow::anyhow!(crate::i18n::tr("aucun écran")))?;
    let region = region.intersect(&screen).ok_or_else(|| anyhow::anyhow!(crate::i18n::tr("zone hors de l'écran")))?;
    // L'écran est capturé dans sa propre résolution, qui peut être inférieure à l'échelle commune.
    let k = display_scale(display) / scale();
    let px = |v: i32| ((v as f64 * k).round() as i32) & !1;
    let crop = format!("crop={}:{}:{}:{}", px(region.w), px(region.h), px(region.x - screen.x), px(region.y - screen.y));
    let filter = if k == 1.0 { crop } else { format!("{crop},scale={}:{}", region.w & !1, region.h & !1) };
    cmd.args(["-thread_queue_size", "512", "-f", "avfoundation", "-framerate", fps]);
    cmd.args(["-capture_cursor", if settings.video_cursor { "1" } else { "0" }]);
    cmd.args(["-i", &format!("Capture screen {index}:{}", audio.unwrap_or("none"))]);
    cmd.args(["-vf", &filter]);
    Ok(())
}
