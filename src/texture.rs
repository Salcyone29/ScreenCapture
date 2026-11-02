//! Texture découpée en tuiles pour afficher des images plus grandes que la limite GPU.

use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, TextureHandle, TextureOptions};
use image::RgbaImage;

const TILE: u32 = 4096;

pub struct TiledTexture {
    pub width: u32,
    pub height: u32,
    tiles: Vec<(u32, u32, u32, u32, TextureHandle)>,
}

impl TiledTexture {
    pub fn new(ctx: &egui::Context, name: &str, img: &RgbaImage, options: TextureOptions) -> Self {
        let mut t = Self { width: 0, height: 0, tiles: Vec::new() };
        t.update(ctx, name, img, options);
        t
    }

    pub fn update(&mut self, ctx: &egui::Context, name: &str, img: &RgbaImage, options: TextureOptions) {
        let same_layout = self.width == img.width() && self.height == img.height() && !self.tiles.is_empty();
        let mut idx = 0;
        let mut new_tiles = Vec::new();
        for ty in (0..img.height()).step_by(TILE as usize) {
            for tx in (0..img.width()).step_by(TILE as usize) {
                let w = TILE.min(img.width() - tx);
                let h = TILE.min(img.height() - ty);
                let sub = image::imageops::crop_imm(img, tx, ty, w, h).to_image();
                let ci = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], sub.as_raw());
                if same_layout {
                    self.tiles[idx].4.set(ci, options);
                } else {
                    new_tiles.push((tx, ty, w, h, ctx.load_texture(format!("{name}-{idx}"), ci, options)));
                }
                idx += 1;
            }
        }
        if !same_layout {
            self.tiles = new_tiles;
            self.width = img.width();
            self.height = img.height();
        }
    }

    /// Dessine l'image entière dans `dest` (en points écran).
    pub fn paint(&self, painter: &egui::Painter, dest: Rect, tint: Color32) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let sx = dest.width() / self.width as f32;
        let sy = dest.height() / self.height as f32;
        for (tx, ty, w, h, tex) in &self.tiles {
            let r = Rect::from_min_size(
                Pos2::new(dest.left() + *tx as f32 * sx, dest.top() + *ty as f32 * sy),
                egui::vec2(*w as f32 * sx, *h as f32 * sy),
            );
            painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), tint);
        }
    }
}
