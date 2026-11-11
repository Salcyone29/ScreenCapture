//! Sonde : fenêtre eframe vide (même titre et mêmes tailles que ScreenCapture), pour vérifier si
//! un défaut de redimensionnement entre écrans d'échelles différentes vient d'eframe/winit.
use eframe::egui;

fn main() -> eframe::Result<()> {
    let title = std::env::args().nth(1).unwrap_or_else(|| "ScreenCapture".into());
    eframe::run_native(
        "ScreenCapture",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title(title)
                .with_inner_size([1200.0, 780.0])
                .with_min_inner_size([820.0, 560.0]),
            ..Default::default()
        },
        Box::new(|_| Ok(Box::new(Probe))),
    )
}

struct Probe;

impl eframe::App for Probe {
    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("échelle {:.2}", ctx.pixels_per_point()));
        });
    }
}
