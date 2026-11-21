//! Capture d'écran : types et fonctions communs ; l'implémentation dépend du système.
//!
//! Les rectangles sont en pixels de l'écran : pixels physiques du bureau virtuel sous Windows,
//! points macOS multipliés par l'échelle Retina (voir `capture_mac`) sous macOS.

use image::RgbaImage;
use serde::{Deserialize, Serialize};

#[cfg(windows)]
#[path = "capture_win.rs"]
pub(crate) mod imp;
#[cfg(target_os = "macos")]
#[path = "capture_mac.rs"]
pub(crate) mod imp;

pub use imp::{capture_rect, cursor_pos, foreground_window_rect, monitors, post_wheel, virtual_screen, visible_windows, wheel_target, work_area_at};

/// Rectangle en pixels de l'écran, coordonnées du bureau virtuel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn from_points(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self::new(x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs())
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }

    pub fn intersect(&self, o: &Rect) -> Option<Rect> {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = (self.x + self.w).min(o.x + o.w);
        let y1 = (self.y + self.h).min(o.y + o.h);
        (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
    }
}

#[derive(Clone, Debug)]
pub struct WindowInfo {
    pub rect: Rect,
}

/// Moniteur contenant le point, sinon le premier.
pub fn monitor_at(x: i32, y: i32) -> Rect {
    let mons = monitors();
    mons.iter().copied().find(|m| m.contains(x, y)).unwrap_or(mons[0])
}

/// Ramène une fenêtre sur un écran connecté : si sa barre de titre n'est plus visible
/// (écran débranché), elle est recentrée sur l'écran principal et réduite à sa zone de travail.
pub fn fit_on_screen(r: Rect) -> Rect {
    let title = Rect::new(r.x + 8, r.y, (r.w - 16).max(1), 32);
    let visible: i32 = monitors().iter().filter_map(|m| m.intersect(&title)).map(|i| i.w).sum();
    if visible >= title.w.min(120) {
        return r;
    }
    let wa = work_area_at(0, 0);
    let (w, h) = (r.w.min(wa.w), r.h.min(wa.h));
    Rect::new(wa.x + (wa.w - w) / 2, wa.y + (wa.h - h) / 2, w, h)
}

/// Recadre une image du bureau virtuel vers une zone absolue.
pub fn crop_virtual(img: &RgbaImage, origin: Rect, r: Rect) -> Option<RgbaImage> {
    let r = r.intersect(&origin)?;
    Some(image::imageops::crop_imm(img, (r.x - origin.x) as u32, (r.y - origin.y) as u32, r.w as u32, r.h as u32).to_image())
}
