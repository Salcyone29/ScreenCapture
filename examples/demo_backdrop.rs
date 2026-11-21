//! Fond de démonstration plein écran (captures d'écran de l'aide) : affiche une image générique
//! sur tout l'écran principal pour que rien du bureau réel n'apparaisse.
//!
//! Usage : `demo_backdrop <image.png> [x y]`
use eframe::egui;

fn main() -> eframe::Result<()> {
    let path = std::env::args().nth(1).expect("image de fond");
    let img = image::open(&path).expect("image lisible").to_rgba8();
    // Écran voulu : un point de cet écran (« x y », facultatif ; écran principal par défaut).
    let at: Vec<f32> = std::env::args().skip(2).filter_map(|a| a.parse().ok()).collect();
    let pos = if at.len() == 2 { [at[0], at[1]] } else { [0.0, 0.0] };
    eframe::run_native(
        "Fond de démonstration",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("Fond de démonstration")
                .with_position(pos)
                .with_fullscreen(true)
                .with_decorations(false),
            ..Default::default()
        },
        Box::new(move |_| Ok(Box::new(Backdrop { img, tex: None }))),
    )
}

struct Backdrop {
    img: image::RgbaImage,
    tex: Option<egui::TextureHandle>,
}

impl eframe::App for Backdrop {
    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        let tex = self.tex.get_or_insert_with(|| {
            let size = [self.img.width() as usize, self.img.height() as usize];
            ctx.load_texture("fond", egui::ColorImage::from_rgba_unmultiplied(size, self.img.as_raw()), egui::TextureOptions::LINEAR)
        });
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            let r = ui.max_rect();
            // Image en « couverture » : remplit l'écran, rognée si les proportions diffèrent.
            let s = tex.size_vec2();
            let k = (r.width() / s.x).max(r.height() / s.y);
            let shown = egui::Rect::from_center_size(r.center(), s * k);
            ui.painter().image(tex.id(), shown, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        });
    }
}
