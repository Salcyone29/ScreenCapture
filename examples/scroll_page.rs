//! Page de test pour la capture avec défilement : une fenêtre avec un en-tête fixe et une longue
//! liste qui défile. Écrit dans le fichier donné le rectangle de la liste (pixels de l'écran).
//!
//! Usage : `scroll_page <fichier-rect>`
use eframe::egui;

pub const ROWS: usize = 120;
pub const ROW_H: f32 = 30.0;

fn main() -> eframe::Result<()> {
    let out = std::env::args().nth(1).expect("fichier du rectangle");
    eframe::run_native(
        "Page de test défilante",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("Page de test défilante")
                .with_position([200.0, 120.0])
                .with_inner_size([620.0, 520.0])
                .with_always_on_top(),
            ..Default::default()
        },
        Box::new(move |_| Ok(Box::new(Page { out, written: false }))),
    )
}

struct Page {
    out: String,
    written: bool,
}

impl eframe::App for Page {
    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        egui::TopBottomPanel::top("entete")
            .exact_height(48.0)
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(40, 60, 120)).inner_margin(12.0))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("En-tête fixe de la page").size(18.0).color(egui::Color32::WHITE));
            });
        let list = egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().animated(true).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for i in 0..ROWS {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_H), egui::Sense::hover());
                        let hue = (i * 37 % 255) as u8;
                        ui.painter().rect_filled(r.shrink2(egui::vec2(0.0, 2.0)), 4.0, egui::Color32::from_rgb(hue, 255 - hue, (i * 11 % 255) as u8));
                        ui.painter().text(r.left_center() + egui::vec2(12.0, 0.0), egui::Align2::LEFT_CENTER, format!("Ligne {i} — contenu de démonstration"), egui::FontId::proportional(16.0), egui::Color32::BLACK);
                    }
                });
            });
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
        if !self.written && ctx.cumulative_pass_nr() > 5 {
            if let Some(outer) = ctx.input(|i| i.viewport().inner_rect) {
                let ppp = ctx.pixels_per_point();
                let r = list.response.rect;
                let (x, y) = ((outer.min.x + r.min.x) * ppp, (outer.min.y + r.min.y) * ppp);
                let (w, h) = (r.width() * ppp, r.height() * ppp);
                std::fs::write(&self.out, format!("{} {} {} {}", x.round(), y.round(), w.round(), h.round())).unwrap();
                self.written = true;
            }
        }
    }
}
