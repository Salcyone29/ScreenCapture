#![windows_subsystem = "windows"]
//! ScreenCapture — capture d'écran, annotation et enregistrement vidéo pour Windows.

mod app;
mod capture;
mod diag;
mod editor;
mod help;
mod i18n;
mod icon;
mod overlay;
mod platform;
mod recorder;
mod retention;
mod settings;
mod texture;
mod theme;
mod update;
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
    update::cleanup_after_update();
    // Pilotage de l'instance en cours (tests de validation) : `ScreenCapture.exe --cmd region`.
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--cmd") {
        let cmd = args.get(i + 1).map(String::as_str).unwrap_or("show");
        std::process::exit(if platform::send_command(cmd) { 0 } else { 1 });
    }
    let from_startup = std::env::args().any(|a| a == "--tray");
    // Démarrage avec Windows : ne pas faire apparaître une instance déjà lancée.
    if !platform::acquire_single_instance(!from_startup) {
        return Ok(());
    }
    let settings = settings::Settings::load();
    diag::set_enabled(settings.diagnostic);
    i18n::set_language(settings.language);
    diag::log(&format!("langue : {:?} ({:?} dans les paramètres)", i18n::current(), settings.language));
    // macOS : autorisation d'enregistrer l'écran. Absente (ou liée à une ancienne signature), la
    // demander : fenêtre système et ajout de l'application à la liste des Réglages.
    if !platform::screen_capture_allowed() {
        diag::log("autorisation d'enregistrement de l'écran : absente, demandée");
        platform::request_screen_capture();
    }
    let start_hidden = settings.start_in_tray || from_startup;
    let icon = egui::IconData { rgba: icon::app_icon(64), width: 64, height: 64 };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ScreenCapture")
            .with_app_id("ScreenCapture")
            .with_inner_size([1200.0, 780.0])
            .with_min_inner_size([app::MIN_SIZE.0, app::MIN_SIZE.1])
            .with_icon(Arc::new(icon)),
        ..Default::default()
    };
    eframe::run_native(
        "ScreenCapture",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, settings, start_hidden)))),
    )
}
