//! Paramètres persistants (JSON dans %APPDATA%\ScreenCapture).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::capture::Rect;
use crate::editor::model::{NumberFormat, COLORS};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Png,
    Jpg,
}

impl ImageFormat {
    pub fn ext(self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpg => "jpg",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoFormat {
    Mp4,
    Gif,
    Webm,
}

impl VideoFormat {
    pub fn ext(self) -> &'static str {
        match self {
            VideoFormat::Mp4 => "mp4",
            VideoFormat::Gif => "gif",
            VideoFormat::Webm => "webm",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AfterCapture {
    OpenEditor,
    ShowWorkspace,
    Nothing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Accent {
    Violet,
    Blue,
    Green,
    Orange,
    Red,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Hotkeys {
    pub region: String,
    pub fullscreen: String,
    pub window: String,
    pub last_region: String,
    pub video: String,
    /// Remplace Win+Maj+S (Outil Capture de Windows) par la capture de zone.
    pub override_snip: bool,
}

impl Default for Hotkeys {
    /// Mac : pas de touche Impr. écran, et ⌘⇧3/4/5 sont pris par macOS.
    #[cfg(target_os = "macos")]
    fn default() -> Self {
        Self {
            region: "super+shift+Digit2".into(),
            fullscreen: "super+shift+Digit1".into(),
            window: "super+shift+Digit7".into(),
            last_region: "super+shift+Digit8".into(),
            video: "super+shift+Digit9".into(),
            override_snip: false,
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn default() -> Self {
        Self {
            region: "PrintScreen".into(),
            fullscreen: "ctrl+PrintScreen".into(),
            window: "alt+PrintScreen".into(),
            last_region: "shift+PrintScreen".into(),
            video: "ctrl+shift+PrintScreen".into(),
            override_snip: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub save_dir: PathBuf,
    pub image_format: ImageFormat,
    pub jpeg_quality: u8,
    pub after_capture: AfterCapture,
    pub copy_to_clipboard: bool,
    pub auto_save: bool,
    pub capture_cursor: bool,
    /// Loupe de précision à côté du curseur pendant la sélection.
    pub show_magnifier: bool,
    pub delay_seconds: u32,
    pub hotkeys: Hotkeys,
    pub theme: ThemeChoice,
    pub accent: Accent,
    pub close_to_tray: bool,
    pub start_in_tray: bool,
    pub last_region: Option<Rect>,
    pub ffmpeg_path: String,
    pub video_format: VideoFormat,
    pub video_fps: u32,
    pub video_show_region: bool,
    pub video_cursor: bool,
    pub audio_device: Option<String>,
    /// Enregistrer le son du micro avec les vidéos (modifiable au moment de choisir la zone).
    pub record_audio: bool,
    /// Conservation : images supprimées après ce nombre de jours (0 = jamais).
    pub retention_images_days: u32,
    /// Conservation : vidéos supprimées après ce nombre de jours (0 = jamais).
    pub retention_videos_days: u32,
    /// Taille maximale du dossier des captures en Mo (0 = sans limite) : les plus anciennes partent.
    pub retention_max_mb: u64,
    /// Suppression vers la Corbeille (récupérable) plutôt que définitive.
    pub retention_to_trash: bool,
    /// Captures épinglées (noms de fichiers) : jamais supprimées automatiquement.
    pub pinned: Vec<String>,
    /// Langue de l'interface.
    pub language: crate::i18n::Language,
    /// Rechercher les nouvelles versions sur GitHub (au démarrage, puis une fois par jour).
    pub check_updates: bool,
    pub effect_border: bool,
    pub effect_border_width: u32,
    pub effect_border_color: [u8; 4],
    pub effect_shadow: bool,
    pub number_format: NumberFormat,
    /// Texte du tampon personnalisé de l'éditeur.
    pub custom_stamp: String,
    /// Journal de diagnostic (activé par un triple-clic sur la version, page « À propos »).
    pub diagnostic: bool,
    /// Nuancier de l'éditeur, modifiable dans les Paramètres.
    pub palette: Vec<[u8; 4]>,
}

impl Default for Settings {
    fn default() -> Self {
        let save_dir = dirs::picture_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
            .join("ScreenCapture");
        Self {
            save_dir,
            image_format: ImageFormat::Png,
            jpeg_quality: 90,
            after_capture: AfterCapture::OpenEditor,
            copy_to_clipboard: true,
            auto_save: true,
            capture_cursor: false,
            show_magnifier: true,
            delay_seconds: 3,
            hotkeys: Hotkeys::default(),
            theme: ThemeChoice::System,
            accent: Accent::Violet,
            close_to_tray: true,
            start_in_tray: false,
            last_region: None,
            ffmpeg_path: String::new(),
            video_format: VideoFormat::Mp4,
            video_fps: 30,
            video_show_region: true,
            video_cursor: true,
            audio_device: None,
            record_audio: true,
            retention_images_days: 0,
            retention_videos_days: 0,
            retention_max_mb: 0,
            retention_to_trash: true,
            pinned: Vec::new(),
            language: crate::i18n::Language::System,
            check_updates: true,
            effect_border: false,
            effect_border_width: 2,
            effect_border_color: [80, 80, 80, 255],
            effect_shadow: false,
            number_format: NumberFormat::Decimal,
            custom_stamp: String::new(),
            diagnostic: false,
            palette: COLORS.to_vec(),
        }
    }
}

fn config_path() -> PathBuf {
    if let Some(data) = crate::update::portable_data_dir() {
        return data.join("settings.json");
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ScreenCapture")
        .join("settings.json")
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read_to_string(config_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = config_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}
