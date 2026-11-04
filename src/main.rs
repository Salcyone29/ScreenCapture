#![windows_subsystem = "windows"]
//! ScreenCapture — capture d'écran, annotation et enregistrement vidéo pour Windows.

mod app;
mod capture;
mod diag;
mod editor;
mod icon;
mod overlay;
mod platform;
mod recorder;
mod settings;
mod texture;
mod theme;
mod workspace;

use std::sync::Arc;

/// Journal de diagnostic (builds de débogage uniquement) : %TEMP%\screencapture.log
pub fn debug_log(msg: &str) {
    if cfg!(debug_assertions) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(std::env::temp_dir().join("screencapture.log")) {
            let _ = writeln!(f, "{} {msg}", chrono::Local::now().format("%H:%M:%S%.3f"));
        }
    }
}

use eframe::egui;

fn main() -> eframe::Result<()> {
    diag::init();
    let from_startup = std::env::args().any(|a| a == "--tray");
    // Démarrage avec Windows : ne pas faire apparaître une instance déjà lancée.
    if !platform::acquire_single_instance(!from_startup) {
        return Ok(());
    }
    let settings = settings::Settings::load();
    let start_hidden = settings.start_in_tray || from_startup;
    let icon = egui::IconData { rgba: icon::app_icon(64), width: 64, height: 64 };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ScreenCapture")
            .with_app_id("ScreenCapture")
            .with_inner_size([1200.0, 780.0])
            .with_min_inner_size([820.0, 560.0])
            .with_icon(Arc::new(icon)),
        ..Default::default()
    };
    eframe::run_native(
        "ScreenCapture",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, settings, start_hidden)))),
    )
}
