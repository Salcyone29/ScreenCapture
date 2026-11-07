//! Surcouche plein écran de sélection : zone libre, fenêtre survolée ou écran entier,
//! avec loupe pixel, réticule et pipette (comme la fenêtre de capture de Screenpresso).

use eframe::egui::{
    self, pos2, vec2, Align2, Color32, ColorImage, CornerRadius, FontId, Key, Pos2, Rect as ERect, Stroke, StrokeKind,
    TextureHandle, TextureOptions,
};
use image::RgbaImage;

use crate::capture::{self, Rect, WindowInfo};
use crate::texture::TiledTexture;
use crate::theme::{self, palette};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Purpose {
    Image,
    Video,
}

pub enum OverlayResult {
    Pending,
    Cancel,
    Selected(Rect),
    CopyColor(String),
}

pub struct Overlay {
    pub purpose: Purpose,
    pub origin: Rect,
    pub image: RgbaImage,
    texture: Option<TiledTexture>,
    windows: Vec<WindowInfo>,
    monitors: Vec<Rect>,
    drag_start: Option<(i32, i32)>,
    current: Option<Rect>,
    mag: Option<TextureHandle>,
    /// Afficher la loupe de précision (réglage « Loupe de précision »).
    pub magnifier: bool,
    frames: u32,
}

const MAG_PIXELS: i32 = 15;
const MAG_SIZE: f32 = 150.0;

impl Overlay {
    pub fn new(purpose: Purpose, origin: Rect, image: RgbaImage) -> Self {
        Self {
            purpose,
            origin,
            image,
            texture: None,
            windows: capture::visible_windows(),
            monitors: capture::monitors(),
            drag_start: None,
            current: None,
            mag: None,
            magnifier: true,
            frames: 0,
        }
    }

    fn color_at(&self, x: i32, y: i32) -> [u8; 3] {
        let ix = (x - self.origin.x).clamp(0, self.image.width() as i32 - 1) as u32;
        let iy = (y - self.origin.y).clamp(0, self.image.height() as i32 - 1) as u32;
        let p = self.image.get_pixel(ix, iy);
        [p[0], p[1], p[2]]
    }

    fn hovered_window(&self, x: i32, y: i32) -> Option<Rect> {
        self.windows
            .iter()
            .find(|w| w.rect.contains(x, y))
            .and_then(|w| w.rect.intersect(&self.origin))
    }

    fn monitor_at(&self, x: i32, y: i32) -> Rect {
        self.monitors.iter().copied().find(|m| m.contains(x, y)).unwrap_or(self.origin)
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> OverlayResult {
        self.frames += 1;
        let p = palette(ctx);
        let accent = if self.purpose == Purpose::Video { Color32::from_rgb(229, 57, 53) } else { p.primary };
        if self.texture.is_none() {
            self.texture = Some(TiledTexture::new(ctx, "overlay", &self.image, TextureOptions::NEAREST));
        }
        let ppp = crate::platform::units_per_point(ctx);
        let o = self.origin;
        let to_pt = move |x: i32, y: i32| pos2((x - o.x) as f32 / ppp, (y - o.y) as f32 / ppp);
        let rect_pt = |r: Rect| ERect::from_min_max(to_pt(r.x, r.y), to_pt(r.x + r.w, r.y + r.h));

        let mut result = OverlayResult::Pending;
        let (pressed, released, down, esc, enter, key_c, right) = ctx.input(|i| {
            (
                i.pointer.primary_pressed(),
                i.pointer.primary_released(),
                i.pointer.primary_down(),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Enter),
                i.key_pressed(Key::C),
                i.pointer.secondary_clicked(),
            )
        });
        // Position lue auprès du système : celle d'egui date d'avant l'affichage de la sélection
        // (fenêtre de l'espace de travail ou garée hors écran) tant que la souris n'a pas bougé,
        // ce qui faisait apparaître la loupe ailleurs pendant les premières images.
        let cursor = capture::cursor_pos();

        if esc || right {
            return OverlayResult::Cancel;
        }
        if enter {
            return OverlayResult::Selected(self.monitor_at(cursor.0, cursor.1));
        }
        if key_c {
            let c = self.color_at(cursor.0, cursor.1);
            return OverlayResult::CopyColor(format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]));
        }
        // Ignore les toutes premières images (relâchement éventuel d'un clic de menu).
        if self.frames > 2 {
            if pressed {
                self.drag_start = Some(cursor);
            }
            if down {
                if let Some(s) = self.drag_start {
                    if (cursor.0 - s.0).abs() > 3 || (cursor.1 - s.1).abs() > 3 {
                        let r = Rect::from_points(s.0, s.1, cursor.0, cursor.1);
                        self.current = r.intersect(&self.origin);
                    }
                }
            }
            if released && self.drag_start.is_some() {
                self.drag_start = None;
                result = match self.current.take() {
                    Some(r) if r.w >= 4 && r.h >= 4 => OverlayResult::Selected(r),
                    _ => OverlayResult::Selected(self.hovered_window(cursor.0, cursor.1).unwrap_or_else(|| self.monitor_at(cursor.0, cursor.1))),
                };
            }
        }

        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            let painter = ui.painter();
            let full = rect_pt(self.origin);
            if let Some(t) = &self.texture {
                t.paint(painter, full, Color32::WHITE);
            }

            let sel = self.current.or_else(|| if self.drag_start.is_none() { self.hovered_window(cursor.0, cursor.1) } else { None });
            let scrim = Color32::from_black_alpha(if sel.is_some() { 120 } else { 80 });
            match sel {
                Some(r) => {
                    let s = rect_pt(r);
                    painter.rect_filled(ERect::from_min_max(full.min, pos2(full.max.x, s.min.y)), 0.0, scrim);
                    painter.rect_filled(ERect::from_min_max(pos2(full.min.x, s.max.y), full.max), 0.0, scrim);
                    painter.rect_filled(ERect::from_min_max(pos2(full.min.x, s.min.y), pos2(s.min.x, s.max.y)), 0.0, scrim);
                    painter.rect_filled(ERect::from_min_max(pos2(s.max.x, s.min.y), pos2(full.max.x, s.max.y)), 0.0, scrim);
                    painter.rect_stroke(s, 0.0, Stroke::new(2.0_f32, accent), StrokeKind::Outside);
                    if self.current.is_some() {
                        for c in [s.left_top(), s.right_top(), s.left_bottom(), s.right_bottom()] {
                            painter.circle(c, 4.0, Color32::WHITE, Stroke::new(2.0_f32, accent));
                        }
                    }
                    let label = format!("{} × {}", r.w, r.h);
                    let chip_pos = if s.min.y > 34.0 { s.left_top() + vec2(0.0, -8.0) } else { s.left_top() + vec2(8.0, 30.0) };
                    let galley = painter.layout_no_wrap(label, theme::medium(13.0), p.on_primary);
                    let chip = ERect::from_min_size(chip_pos - vec2(0.0, 26.0), galley.size() + vec2(20.0, 10.0));
                    painter.rect_filled(chip, 13.0, accent);
                    painter.galley(chip.min + vec2(10.0, 5.0), galley, p.on_primary);
                }
                None => {
                    painter.rect_filled(full, 0.0, scrim);
                }
            }

            // Réticule bicolore (sombre + clair côte à côte) : lisible sur fond clair comme sombre.
            let c = to_pt(cursor.0, cursor.1);
            let px = 1.0 / ppp;
            let dark = Stroke::new(px, Color32::from_black_alpha(200));
            let light = Stroke::new(px, Color32::from_white_alpha(220));
            painter.line_segment([pos2(full.min.x, c.y), pos2(full.max.x, c.y)], dark);
            painter.line_segment([pos2(full.min.x, c.y + px), pos2(full.max.x, c.y + px)], light);
            painter.line_segment([pos2(c.x, full.min.y), pos2(c.x, full.max.y)], dark);
            painter.line_segment([pos2(c.x + px, full.min.y), pos2(c.x + px, full.max.y)], light);

            // Bandeau d'aide (en haut du moniteur courant).
            let mon = rect_pt(self.monitor_at(cursor.0, cursor.1));
            let help = match self.purpose {
                Purpose::Image => "Glisser : zone  •  Clic : fenêtre  •  Entrée : écran entier  •  C : copier la couleur  •  Échap : annuler",
                Purpose::Video => "Sélectionnez la zone à enregistrer  •  Clic : fenêtre  •  Entrée : écran entier  •  Échap : annuler",
            };
            let g = painter.layout_no_wrap(help.to_string(), FontId::proportional(13.5), p.inverse_on_surface);
            let bar = ERect::from_center_size(pos2(mon.center().x, mon.top() + 36.0), g.size() + vec2(36.0, 18.0));
            painter.rect_filled(bar.translate(vec2(0.0, 2.0)), 22.0, Color32::from_black_alpha(60));
            painter.rect_filled(bar, 22.0, p.inverse_surface);
            painter.galley(bar.min + vec2(18.0, 9.0), g, p.inverse_on_surface);

            if self.magnifier {
                self.paint_magnifier(ctx, painter, cursor, c, full);
            }
        });

        crate::theme::crosshair_cursor(ctx);
        ctx.request_repaint();
        result
    }

    fn paint_magnifier(&mut self, ctx: &egui::Context, painter: &egui::Painter, cursor: (i32, i32), c: Pos2, full: ERect) {
        let p = palette(ctx);
        let half = MAG_PIXELS / 2;
        let mut img = ColorImage::new([MAG_PIXELS as usize, MAG_PIXELS as usize], Color32::BLACK);
        for dy in 0..MAG_PIXELS {
            for dx in 0..MAG_PIXELS {
                let x = cursor.0 - half + dx;
                let y = cursor.1 - half + dy;
                let inside = self.origin.contains(x, y);
                let col = if inside { self.color_at(x, y) } else { [0, 0, 0] };
                img.pixels[(dy * MAG_PIXELS + dx) as usize] = Color32::from_rgb(col[0], col[1], col[2]);
            }
        }
        match &mut self.mag {
            Some(t) => t.set(img, TextureOptions::NEAREST),
            None => self.mag = Some(ctx.load_texture("magnifier", img, TextureOptions::NEAREST)),
        }
        let Some(tex) = &self.mag else { return };

        let mut pos = c + vec2(24.0, 24.0);
        if pos.x + MAG_SIZE > full.max.x {
            pos.x = c.x - 24.0 - MAG_SIZE;
        }
        if pos.y + MAG_SIZE + 52.0 > full.max.y {
            pos.y = c.y - 24.0 - MAG_SIZE - 52.0;
        }
        let card = ERect::from_min_size(pos, vec2(MAG_SIZE, MAG_SIZE + 52.0));
        painter.rect_filled(card.translate(vec2(0.0, 3.0)), 14.0, Color32::from_black_alpha(70));
        painter.rect_filled(card, 14.0, p.surface_high);
        let view = ERect::from_min_size(pos + vec2(6.0, 6.0), vec2(MAG_SIZE - 12.0, MAG_SIZE - 12.0));
        painter.image(tex.id(), view, ERect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
        let cell = view.width() / MAG_PIXELS as f32;
        let center = ERect::from_min_size(view.min + vec2(cell * half as f32, cell * half as f32), vec2(cell, cell));
        painter.rect_stroke(center, 0.0, Stroke::new(1.5_f32, Color32::WHITE), StrokeKind::Outside);
        painter.rect_stroke(view, CornerRadius::same(8), Stroke::new(1.0_f32, p.outline_variant), StrokeKind::Outside);

        let col = self.color_at(cursor.0, cursor.1);
        let sw = ERect::from_min_size(pos2(card.left() + 12.0, card.bottom() - 40.0), vec2(14.0, 14.0));
        painter.rect(sw, 4.0, Color32::from_rgb(col[0], col[1], col[2]), Stroke::new(1.0_f32, p.outline), StrokeKind::Outside);
        painter.text(
            pos2(sw.right() + 8.0, sw.center().y),
            Align2::LEFT_CENTER,
            format!("#{:02X}{:02X}{:02X}", col[0], col[1], col[2]),
            theme::medium(13.0),
            p.on_surface,
        );
        painter.text(
            pos2(card.left() + 12.0, card.bottom() - 14.0),
            Align2::LEFT_CENTER,
            format!("X {}   Y {}", cursor.0, cursor.1),
            FontId::proportional(12.0),
            p.on_surface_variant,
        );
    }
}
