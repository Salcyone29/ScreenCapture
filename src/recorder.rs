//! Enregistrement vidéo de l'écran via FFmpeg (gdigrab + micro DirectShow optionnel).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::Instant;

use crate::capture::Rect;
use crate::platform::hidden_command;
use crate::settings::{Settings, VideoFormat};

/// Recherche ffmpeg.exe : chemin configuré, dossier de l'exécutable, PATH, puis installations connues.
pub fn find_ffmpeg(settings: &Settings) -> Option<PathBuf> {
    let configured = settings.ffmpeg_path.trim();
    if !configured.is_empty() && Path::new(configured).is_file() {
        return Some(configured.into());
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for c in [dir.join("ffmpeg.exe"), dir.join("ffmpeg").join("ffmpeg.exe")] {
                if c.is_file() {
                    return Some(c);
                }
            }
        }
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&paths) {
            let c = d.join("ffmpeg.exe");
            if c.is_file() {
                return Some(c);
            }
        }
    }
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

/// Nom DirectShow correspondant à un micro Windows (DirectShow tronque parfois les noms longs).
fn resolve_dshow_audio(ffmpeg: &Path, name: &str) -> Option<String> {
    let devices = list_audio_devices(ffmpeg);
    devices
        .iter()
        .find(|d| d.as_str() == name)
        .or_else(|| devices.iter().find(|d| name.starts_with(d.as_str()) || d.starts_with(name)))
        .cloned()
}

/// Liste les périphériques audio DirectShow.
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
}

impl Recorder {
    pub fn start(settings: &Settings, region: Rect, output: PathBuf) -> anyhow::Result<Self> {
        let ffmpeg = find_ffmpeg(settings).ok_or_else(|| {
            anyhow::anyhow!("FFmpeg introuvable. Installez-le (winget install ffmpeg) ou indiquez son chemin dans les paramètres.")
        })?;
        // x264 / VP9 exigent des dimensions paires.
        let region = Rect::new(region.x, region.y, region.w & !1, region.h & !1);
        anyhow::ensure!(region.w >= 16 && region.h >= 16, "zone trop petite pour une vidéo");
        let temp = if settings.video_format == VideoFormat::Gif { output.with_extension("tmp.mp4") } else { output.clone() };
        let fps = settings.video_fps.clamp(5, 60).to_string();

        let mut cmd = hidden_command(&ffmpeg.to_string_lossy());
        cmd.args(["-hide_banner", "-loglevel", "error", "-y"]);
        cmd.args(["-thread_queue_size", "512", "-f", "gdigrab", "-framerate", &fps]);
        cmd.args(["-draw_mouse", if settings.video_cursor { "1" } else { "0" }]);
        if settings.video_show_region {
            cmd.args(["-show_region", "1"]);
        }
        cmd.args(["-offset_x", &region.x.to_string(), "-offset_y", &region.y.to_string()]);
        cmd.args(["-video_size", &format!("{}x{}", region.w, region.h), "-i", "desktop"]);
        // Micro débranché ou introuvable : enregistrer sans le son plutôt qu'échouer.
        let mut warning = None;
        let audio = settings.audio_device.as_ref().filter(|_| settings.video_format != VideoFormat::Gif).and_then(|name| {
            let dev = resolve_dshow_audio(&ffmpeg, name);
            if dev.is_none() {
                warning = Some(format!("Micro « {name} » introuvable : enregistrement sans le son"));
            }
            dev
        });
        if let Some(dev) = &audio {
            cmd.args(["-thread_queue_size", "512", "-f", "dshow", "-i", &format!("audio={dev}")]);
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
        Ok(Self { child, temp, output, format: settings.video_format, ffmpeg, started: Instant::now(), region, warning })
    }

    /// Arrête proprement l'enregistrement (touche « q ») et convertit en GIF si nécessaire.
    /// Bloquant : à appeler depuis un thread de travail.
    pub fn finish(mut self) -> anyhow::Result<PathBuf> {
        if let Some(stdin) = self.child.stdin.as_mut() {
            let _ = stdin.write_all(b"q");
            let _ = stdin.flush();
        }
        drop(self.child.stdin.take());
        let status = self.child.wait()?;
        anyhow::ensure!(self.temp.is_file(), "FFmpeg n'a produit aucun fichier (code {status})");
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
            anyhow::ensure!(st.success(), "conversion GIF échouée");
        }
        Ok(self.output)
    }

    pub fn abort(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.temp);
    }
}
