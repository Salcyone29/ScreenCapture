//! Icône de l'application générée par programme (fenêtre + zone de notification).

use tiny_skia::{FillRule, LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};

pub fn app_icon(size: u32) -> Vec<u8> {
    let mut pm = Pixmap::new(size, size).expect("pixmap");
    let s = size as f32 / 64.0;
    let mut bg = Paint::default();
    bg.set_color_rgba8(103, 80, 164, 255);
    bg.anti_alias = true;
    if let Some(r) = tiny_skia::Rect::from_xywh(2.0 * s, 2.0 * s, 60.0 * s, 60.0 * s) {
        let mut pb = PathBuilder::new();
        let k = 14.0 * s;
        let (l, t, rr, b) = (r.left(), r.top(), r.right(), r.bottom());
        pb.move_to(l + k, t);
        pb.line_to(rr - k, t);
        pb.quad_to(rr, t, rr, t + k);
        pb.line_to(rr, b - k);
        pb.quad_to(rr, b, rr - k, b);
        pb.line_to(l + k, b);
        pb.quad_to(l, b, l, b - k);
        pb.line_to(l, t + k);
        pb.quad_to(l, t, l + k, t);
        pb.close();
        if let Some(p) = pb.finish() {
            pm.fill_path(&p, &bg, FillRule::Winding, Transform::identity(), None);
        }
    }
    let mut white = Paint::default();
    white.set_color_rgba8(255, 255, 255, 255);
    white.anti_alias = true;
    let stroke = Stroke { width: 5.0 * s, line_cap: LineCap::Round, ..Default::default() };
    // Coins de sélection.
    let (a, b, l) = (15.0 * s, 49.0 * s, 10.0 * s);
    for (x, y, dx, dy) in [(a, a, 1.0, 1.0), (b, a, -1.0, 1.0), (a, b, 1.0, -1.0), (b, b, -1.0, -1.0)] {
        let mut pb = PathBuilder::new();
        pb.move_to(x, y + dy * l);
        pb.line_to(x, y);
        pb.line_to(x + dx * l, y);
        if let Some(p) = pb.finish() {
            pm.stroke_path(&p, &white, &stroke, Transform::identity(), None);
        }
    }
    let mut accent = Paint::default();
    accent.set_color_rgba8(234, 221, 255, 255);
    accent.anti_alias = true;
    if let Some(c) = PathBuilder::from_circle(32.0 * s, 32.0 * s, 8.0 * s) {
        pm.fill_path(&c, &accent, FillRule::Winding, Transform::identity(), None);
    }
    // Autonome (sans dépendance au reste du crate) pour être réutilisé par `examples/make_icon.rs`.
    pm.pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}
