//! Rendu raster des annotations (tiny-skia + ab_glyph). Sert à l'affichage et à l'export,
//! ce qui garantit un résultat identique à l'écran et dans le fichier.

use std::sync::OnceLock;

use ab_glyph::{point, Font, FontVec, PxScale, ScaleFont};
use eframe::egui::{Pos2, Rect};
use image::RgbaImage;
use tiny_skia::{
    BlendMode, FillRule, FilterQuality, IntSize, LineCap, LineJoin, Paint, Path, PathBuilder, Pattern,
    Pixmap, PremultipliedColorU8, SpreadMode, Stroke, StrokeDash, Transform,
};

use super::model::{number_half_width, number_radius, BubbleKind, Kind, Shape, Style};

struct Fonts {
    regular: Option<FontVec>,
    bold: Option<FontVec>,
}

/// Polices du système, par ordre de préférence : (fichier, indice dans une collection .ttc).
fn system_candidates(bold: bool) -> Vec<(String, u32)> {
    if cfg!(windows) {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        let names: &[&str] = if bold { &["segoeuib.ttf", "arialbd.ttf"] } else { &["segoeui.ttf", "arial.ttf"] };
        names.iter().map(|n| (format!("{windir}\\Fonts\\{n}"), 0)).collect()
    } else {
        // macOS : Arial (Supplemental selon les versions), puis Helvetica (collection .ttc).
        let files: &[(&str, u32)] = if bold {
            &[
                ("/System/Library/Fonts/Supplemental/Arial Bold.ttf", 0),
                ("/Library/Fonts/Arial Bold.ttf", 0),
                ("/System/Library/Fonts/Helvetica.ttc", 1),
            ]
        } else {
            &[
                ("/System/Library/Fonts/Supplemental/Arial.ttf", 0),
                ("/Library/Fonts/Arial.ttf", 0),
                ("/System/Library/Fonts/Helvetica.ttc", 0),
            ]
        };
        files.iter().map(|(f, i)| (f.to_string(), *i)).collect()
    }
}

fn load_font(bold: bool) -> Option<FontVec> {
    system_candidates(bold)
        .into_iter()
        .find_map(|(file, index)| std::fs::read(&file).ok().and_then(|b| FontVec::try_from_vec_and_index(b, index).ok()))
        // Repli toujours disponible : la police intégrée à egui (le texte n'est jamais vide).
        .or_else(|| {
            let defs = eframe::egui::FontDefinitions::default();
            defs.font_data.get("Ubuntu-Light").and_then(|d| FontVec::try_from_vec_and_index(d.font.to_vec(), d.index).ok())
        })
}

fn fonts() -> &'static Fonts {
    static F: OnceLock<Fonts> = OnceLock::new();
    F.get_or_init(|| Fonts { regular: load_font(false), bold: load_font(true) })
}

fn font(bold: bool) -> Option<&'static FontVec> {
    let f = fonts();
    if bold {
        f.bold.as_ref().or(f.regular.as_ref())
    } else {
        f.regular.as_ref()
    }
}

/// Taille (largeur, hauteur) d'un texte multi-lignes.
pub fn measure_text(text: &str, size: f32, bold: bool) -> (f32, f32) {
    let Some(font) = font(bold) else { return (size * text.len() as f32 * 0.5, size * 1.3) };
    let sf = font.as_scaled(PxScale::from(size));
    let line_h = sf.ascent() - sf.descent() + sf.line_gap();
    let mut w: f32 = 0.0;
    let mut n = 0;
    for line in text.split('\n') {
        n += 1;
        let mut x = 0.0;
        let mut prev = None;
        for c in line.chars() {
            let id = font.glyph_id(c);
            if let Some(p) = prev {
                x += sf.kern(p, id);
            }
            x += sf.h_advance(id);
            prev = Some(id);
        }
        w = w.max(x);
    }
    (w.ceil(), (line_h * n as f32).ceil())
}

fn draw_text(pm: &mut Pixmap, text: &str, size: f32, bold: bool, origin: Pos2, color: [u8; 4]) {
    let Some(font) = font(bold) else { return };
    let scale = PxScale::from(size);
    let sf = font.as_scaled(scale);
    let line_h = sf.ascent() - sf.descent() + sf.line_gap();
    let (w, h) = (pm.width() as i32, pm.height() as i32);
    let pixels = pm.pixels_mut();
    for (li, line) in text.split('\n').enumerate() {
        let mut x = origin.x;
        let y = origin.y + li as f32 * line_h + sf.ascent();
        let mut prev = None;
        for c in line.chars() {
            let id = font.glyph_id(c);
            if let Some(p) = prev {
                x += sf.kern(p, id);
            }
            let g = id.with_scale_and_position(scale, point(x, y));
            x += sf.h_advance(id);
            prev = Some(id);
            let Some(og) = font.outline_glyph(g) else { continue };
            let b = og.px_bounds();
            og.draw(|gx, gy, cov| {
                let px = b.min.x as i32 + gx as i32;
                let py = b.min.y as i32 + gy as i32;
                if px < 0 || py < 0 || px >= w || py >= h {
                    return;
                }
                let a = cov.clamp(0.0, 1.0) * color[3] as f32 / 255.0;
                if a <= 0.0 {
                    return;
                }
                let d = &mut pixels[(py * w + px) as usize];
                let inv = 1.0 - a;
                let r = color[0] as f32 * a + d.red() as f32 * inv;
                let gg = color[1] as f32 * a + d.green() as f32 * inv;
                let bb = color[2] as f32 * a + d.blue() as f32 * inv;
                let aa = 255.0 * a + d.alpha() as f32 * inv;
                if let Some(c) = PremultipliedColorU8::from_rgba(r as u8, gg as u8, bb as u8, aa as u8) {
                    *d = c;
                }
            });
        }
    }
}

fn paint(color: [u8; 4]) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(color[0], color[1], color[2], color[3]);
    p.anti_alias = true;
    p
}

fn shadow_paint() -> Paint<'static> {
    paint([0, 0, 0, 70])
}

fn stroke(style: &Style) -> Stroke {
    let mut s = Stroke { width: style.width.max(1.0), line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() };
    if style.dashed {
        s.dash = StrokeDash::new(vec![style.width * 3.0, style.width * 2.0], 0.0);
        s.line_cap = LineCap::Butt;
    }
    s
}

fn rect_path(r: Rect, radius: f32) -> Option<Path> {
    if r.width() < 0.5 || r.height() < 0.5 {
        return None;
    }
    if radius <= 0.0 {
        return PathBuilder::from_rect(tiny_skia::Rect::from_ltrb(r.left(), r.top(), r.right(), r.bottom())?).into();
    }
    let k = 0.552_284_8 * radius;
    let (l, t, rr, b) = (r.left(), r.top(), r.right(), r.bottom());
    let radius = radius.min(r.width() / 2.0).min(r.height() / 2.0);
    let mut pb = PathBuilder::new();
    pb.move_to(l + radius, t);
    pb.line_to(rr - radius, t);
    pb.cubic_to(rr - radius + k, t, rr, t + radius - k, rr, t + radius);
    pb.line_to(rr, b - radius);
    pb.cubic_to(rr, b - radius + k, rr - radius + k, b, rr - radius, b);
    pb.line_to(l + radius, b);
    pb.cubic_to(l + radius - k, b, l, b - radius + k, l, b - radius);
    pb.line_to(l, t + radius);
    pb.cubic_to(l, t + radius - k, l + radius - k, t, l + radius, t);
    pb.close();
    pb.finish()
}

fn oval_path(r: Rect) -> Option<Path> {
    PathBuilder::from_oval(tiny_skia::Rect::from_ltrb(r.left(), r.top(), r.right(), r.bottom())?)
}

fn arrow_path(a: Pos2, b: Pos2, w: f32) -> Option<Path> {
    let d = b - a;
    let len = d.length();
    if len < 2.0 {
        return None;
    }
    let dir = d / len;
    let n = eframe::egui::vec2(-dir.y, dir.x);
    let head_len = (w * 4.5).max(14.0).min(len * 0.6);
    let head_w = head_len * 0.55;
    let base = b - dir * head_len;
    let shaft = (w * 0.6).max(1.0);
    let tail = (w * 0.2).max(0.5);
    let mut pb = PathBuilder::new();
    let pts = [
        a + n * tail,
        base + n * shaft,
        base + n * head_w,
        b,
        base - n * head_w,
        base - n * shaft,
        a - n * tail,
    ];
    pb.move_to(pts[0].x, pts[0].y);
    for p in &pts[1..] {
        pb.line_to(p.x, p.y);
    }
    pb.close();
    pb.finish()
}

fn line_path(pts: &[Pos2]) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let first = pts.first()?;
    pb.move_to(first.x, first.y);
    if pts.len() == 1 {
        pb.line_to(first.x + 0.1, first.y);
    }
    for p in &pts[1..] {
        pb.line_to(p.x, p.y);
    }
    pb.finish()
}

fn shadow_offset(style: &Style) -> Transform {
    let o = (style.width * 0.5).clamp(2.0, 6.0);
    Transform::from_translate(o, o)
}

fn fill_with_shadow(pm: &mut Pixmap, path: &Path, style: &Style, color: [u8; 4]) {
    if style.shadow {
        pm.fill_path(path, &shadow_paint(), FillRule::Winding, shadow_offset(style), None);
    }
    pm.fill_path(path, &paint(color), FillRule::Winding, Transform::identity(), None);
}

fn stroke_with_shadow(pm: &mut Pixmap, path: &Path, style: &Style) {
    let s = stroke(style);
    if style.shadow {
        pm.stroke_path(path, &shadow_paint(), &s, shadow_offset(style), None);
    }
    pm.stroke_path(path, &paint(style.color), &s, Transform::identity(), None);
}

fn pixelate(pm: &mut Pixmap, r: Rect, block: u32) {
    let (w, h) = (pm.width() as i32, pm.height() as i32);
    let x0 = (r.left().floor() as i32).clamp(0, w);
    let y0 = (r.top().floor() as i32).clamp(0, h);
    let x1 = (r.right().ceil() as i32).clamp(0, w);
    let y1 = (r.bottom().ceil() as i32).clamp(0, h);
    let block = block.max(2) as i32;
    let px = pm.pixels_mut();
    let mut by = y0;
    while by < y1 {
        let mut bx = x0;
        let ey = (by + block).min(y1);
        while bx < x1 {
            let ex = (bx + block).min(x1);
            let (mut sr, mut sg, mut sb, mut sa, mut n) = (0u32, 0u32, 0u32, 0u32, 0u32);
            for y in by..ey {
                for x in bx..ex {
                    let c = px[(y * w + x) as usize];
                    sr += c.red() as u32;
                    sg += c.green() as u32;
                    sb += c.blue() as u32;
                    sa += c.alpha() as u32;
                    n += 1;
                }
            }
            if n > 0 {
                if let Some(c) = PremultipliedColorU8::from_rgba((sr / n) as u8, (sg / n) as u8, (sb / n) as u8, (sa / n) as u8) {
                    for y in by..ey {
                        for x in bx..ex {
                            px[(y * w + x) as usize] = c;
                        }
                    }
                }
            }
            bx = ex;
        }
        by = ey;
    }
}

fn contrast_on(c: [u8; 4]) -> [u8; 4] {
    let l = 0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32;
    if l > 160.0 {
        [20, 20, 20, 255]
    } else {
        [255, 255, 255, 255]
    }
}

pub const TEXT_PAD: f32 = 10.0;

/// Taille de la boîte d'une forme textuelle (texte + marges).
pub fn text_box_size(shape: &Shape) -> (f32, f32) {
    match &shape.kind {
        Kind::Text(t) => {
            let (w, h) = measure_text(if t.is_empty() { " " } else { t }, shape.style.font_size, false);
            if shape.style.filled {
                (w + TEXT_PAD * 2.0, h + TEXT_PAD)
            } else {
                (w, h)
            }
        }
        Kind::Bubble(t) => {
            let (w, h) = measure_text(if t.is_empty() { " " } else { t }, shape.style.font_size, false);
            let (w, h) = (w + TEXT_PAD * 2.4, h + TEXT_PAD * 1.6);
            match shape.style.bubble {
                // Le texte doit tenir dans l'ellipse inscrite : boîte agrandie de √2.
                BubbleKind::Oval | BubbleKind::Thought => (w * 1.42, h * 1.42 + TEXT_PAD),
                _ => (w, h),
            }
        }
        Kind::Stamp(t) => {
            let (w, h) = measure_text(t, shape.style.font_size * 1.3, true);
            (w + 36.0, h + 16.0)
        }
        _ => (0.0, 0.0),
    }
}

/// Pointe d'une bulle (coordonnées de l'image).
pub fn bubble_tip(s: &Shape) -> Pos2 {
    let (w, h) = text_box_size(s);
    match s.style.tail {
        Some(o) => s.a + o,
        None => s.a + eframe::egui::vec2((w * 0.18).clamp(12.0, 40.0) - 6.0, h + 18.0),
    }
}

/// Décalage du texte d'une bulle par rapport à son coin haut gauche (texte centré).
pub fn bubble_text_offset(s: &Shape) -> eframe::egui::Vec2 {
    let Kind::Bubble(t) = &s.kind else { return eframe::egui::Vec2::ZERO };
    let (w, h) = text_box_size(s);
    let (tw, th) = measure_text(if t.is_empty() { " " } else { t }, s.style.font_size, false);
    eframe::egui::vec2((w - tw) / 2.0, (h - th) / 2.0)
}

fn draw_bubble(pm: &mut Pixmap, s: &Shape, t: &str) {
    let st = &s.style;
    let (w, h) = text_box_size(s);
    let body = Rect::from_min_size(s.a, eframe::egui::vec2(w, h));
    let tip = bubble_tip(s);
    let c = body.center();
    let d = tip - c;
    let len = d.length();
    if st.bubble == BubbleKind::Thought {
        // Petits nuages de plus en plus petits jusqu'à la pointe.
        if len > 1.0 {
            let dir = d / len;
            // Bord de l'ellipse dans la direction de la pointe.
            let (rx, ry) = (w / 2.0, h / 2.0);
            let edge = 1.0 / ((dir.x / rx).powi(2) + (dir.y / ry).powi(2)).sqrt();
            let base = (h.min(w) * 0.16).clamp(6.0, 18.0);
            let start = (edge + base * 1.2).min(len);
            for (k, f) in [(0.0f32, 1.0f32), (0.55, 0.75), (1.0, 0.5)] {
                let p = c + dir * (start + (len - start) * k);
                if let Some(path) = PathBuilder::from_circle(p.x, p.y, base * f) {
                    fill_with_shadow(pm, &path, st, st.color);
                }
            }
        }
        // Nuage : ellipse bordée de bosses (un seul chemin, une seule ombre).
        let mut pb = PathBuilder::new();
        if let Some(r) = tiny_skia::Rect::from_ltrb(body.left(), body.top(), body.right(), body.bottom()) {
            pb.push_oval(r);
        }
        let bump = (w.min(h) * 0.22).clamp(6.0, 40.0);
        let n = ((std::f32::consts::PI * (w + h) / 2.0) / (bump * 1.4)).round().clamp(8.0, 28.0) as usize;
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let p = c + eframe::egui::vec2(a.cos() * (w / 2.0 - bump * 0.35), a.sin() * (h / 2.0 - bump * 0.35));
            pb.push_circle(p.x, p.y, bump);
        }
        if let Some(path) = pb.finish() {
            fill_with_shadow(pm, &path, st, st.color);
        }
    } else {
        // Pointe : triangle depuis le centre de la bulle (le corps recouvre sa base).
        if len > 1.0 {
            let n = eframe::egui::vec2(-d.y, d.x) / len;
            let half = (w.min(h) * 0.22).clamp(6.0, 26.0);
            let (b1, b2) = (c + n * half, c - n * half);
            let mut pb = PathBuilder::new();
            pb.move_to(b1.x, b1.y);
            pb.line_to(tip.x, tip.y);
            pb.line_to(b2.x, b2.y);
            pb.close();
            if let Some(tail) = pb.finish() {
                fill_with_shadow(pm, &tail, st, st.color);
            }
        }
        let path = match st.bubble {
            BubbleKind::Rectangle => rect_path(body, 0.0),
            BubbleKind::Oval => oval_path(body),
            _ => rect_path(body, 12.0),
        };
        if let Some(p) = path {
            fill_with_shadow(pm, &p, st, st.color);
        }
    }
    draw_text(pm, t, st.font_size, false, s.a + bubble_text_offset(s), contrast_on(st.color));
}

fn draw_shape(pm: &mut Pixmap, s: &Shape) {
    let st = &s.style;
    let r = s.rect();
    match &s.kind {
        Kind::Arrow => {
            if let Some(p) = arrow_path(s.a, s.b, st.width) {
                fill_with_shadow(pm, &p, st, st.color);
            }
        }
        Kind::Line => {
            if let Some(p) = line_path(&[s.a, s.b]) {
                stroke_with_shadow(pm, &p, st);
            }
        }
        Kind::Pen(pts) => {
            if let Some(p) = line_path(pts) {
                stroke_with_shadow(pm, &p, st);
            }
        }
        Kind::Rect => {
            if let Some(p) = rect_path(r, 0.0) {
                if st.filled {
                    fill_with_shadow(pm, &p, st, st.color);
                } else {
                    stroke_with_shadow(pm, &p, st);
                }
            }
        }
        Kind::Ellipse => {
            if let Some(p) = oval_path(r) {
                if st.filled {
                    fill_with_shadow(pm, &p, st, st.color);
                } else {
                    stroke_with_shadow(pm, &p, st);
                }
            }
        }
        Kind::Highlight => {
            if let Some(p) = rect_path(r, 0.0) {
                let mut pt = paint([st.color[0], st.color[1], st.color[2], 255]);
                pt.blend_mode = BlendMode::Multiply;
                pm.fill_path(&p, &pt, FillRule::Winding, Transform::identity(), None);
            }
        }
        Kind::Blur => {
            let block = (st.width * 3.0).max(6.0) as u32;
            pixelate(pm, r, block);
        }
        Kind::Magnifier => {
            let Some(path) = oval_path(r) else { return };
            let snapshot = pm.clone();
            let c = r.center();
            let zoom = 2.0;
            let tr = Transform::from_translate(c.x, c.y).pre_scale(zoom, zoom).pre_translate(-c.x, -c.y);
            if st.shadow {
                pm.fill_path(&path, &shadow_paint(), FillRule::Winding, shadow_offset(st), None);
            }
            let pat = Pattern::new(snapshot.as_ref(), SpreadMode::Pad, FilterQuality::Bicubic, 1.0, tr);
            let mut p = Paint::default();
            p.shader = pat;
            p.anti_alias = true;
            pm.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
            let s2 = Stroke { width: st.width.max(2.0), ..Default::default() };
            pm.stroke_path(&path, &paint(st.color), &s2, Transform::identity(), None);
        }
        Kind::Text(t) => {
            let (w, h) = text_box_size(s);
            let mut origin = s.a;
            if st.filled {
                if let Some(p) = rect_path(Rect::from_min_size(s.a, eframe::egui::vec2(w, h)), 6.0) {
                    fill_with_shadow(pm, &p, st, [255, 255, 255, 235]);
                }
                origin += eframe::egui::vec2(TEXT_PAD, TEXT_PAD / 2.0);
            } else if st.shadow {
                draw_text(pm, t, st.font_size, false, origin + eframe::egui::vec2(1.5, 1.5), [0, 0, 0, 110]);
            }
            draw_text(pm, t, st.font_size, false, origin, st.color);
        }
        Kind::Bubble(t) => draw_bubble(pm, s, t),
        Kind::Number(n, f) => {
            let rad = number_radius(st);
            let label = f.format(*n);
            let half = number_half_width(st, &label);
            let path = if half > rad {
                rect_path(Rect::from_center_size(s.a, eframe::egui::vec2(half * 2.0, rad * 2.0)), rad)
            } else {
                PathBuilder::from_circle(s.a.x, s.a.y, rad)
            };
            if let Some(p) = path {
                fill_with_shadow(pm, &p, st, st.color);
                let w2 = Stroke { width: 2.0, ..Default::default() };
                pm.stroke_path(&p, &paint([255, 255, 255, 230]), &w2, Transform::identity(), None);
            }
            let fs = rad * 1.1;
            let (tw, th) = measure_text(&label, fs, true);
            draw_text(pm, &label, fs, true, s.a - eframe::egui::vec2(tw / 2.0, th / 2.0), contrast_on(st.color));
        }
        Kind::Patch { img, src, fill } => {
            if let Some(r) = tiny_skia::Rect::from_ltrb(src.left(), src.top(), src.right(), src.bottom()) {
                pm.fill_rect(r, &paint(*fill), Transform::identity(), None);
            }
            let dst = s.rect();
            if dst.width() >= 1.0 && dst.height() >= 1.0 {
                let patch = to_pixmap(img);
                let t = Transform::from_row(
                    dst.width() / img.width() as f32,
                    0.0,
                    0.0,
                    dst.height() / img.height() as f32,
                    dst.left(),
                    dst.top(),
                );
                let pp = tiny_skia::PixmapPaint { quality: FilterQuality::Bilinear, ..Default::default() };
                pm.draw_pixmap(0, 0, patch.as_ref(), &pp, t, None);
            }
        }
        Kind::Stamp(t) => {
            let (w, h) = text_box_size(s);
            let body = Rect::from_min_size(s.a, eframe::egui::vec2(w, h));
            let c = [st.color[0], st.color[1], st.color[2], 225];
            if let Some(p) = rect_path(body.shrink(2.0), 8.0) {
                pm.fill_path(&p, &paint([255, 255, 255, 140]), FillRule::Winding, Transform::identity(), None);
                let s4 = Stroke { width: 4.0, ..Default::default() };
                pm.stroke_path(&p, &paint(c), &s4, Transform::identity(), None);
            }
            if let Some(p) = rect_path(body.shrink(8.0), 5.0) {
                let s1 = Stroke { width: 1.5, ..Default::default() };
                pm.stroke_path(&p, &paint(c), &s1, Transform::identity(), None);
            }
            let fs = st.font_size * 1.3;
            let (tw, th) = measure_text(t, fs, true);
            draw_text(pm, t, fs, true, body.center() - eframe::egui::vec2(tw / 2.0, th / 2.0), c);
        }
    }
}

pub fn to_pixmap(img: &RgbaImage) -> Pixmap {
    let mut data = img.as_raw().clone();
    for px in data.chunks_exact_mut(4) {
        let a = px[3] as u16;
        if a < 255 {
            px[0] = (px[0] as u16 * a / 255) as u8;
            px[1] = (px[1] as u16 * a / 255) as u8;
            px[2] = (px[2] as u16 * a / 255) as u8;
        }
    }
    Pixmap::from_vec(data, IntSize::from_wh(img.width().max(1), img.height().max(1)).unwrap()).expect("pixmap")
}

pub fn from_pixmap(pm: Pixmap) -> RgbaImage {
    let (w, h) = (pm.width(), pm.height());
    let mut data = pm.take();
    for px in data.chunks_exact_mut(4) {
        let a = px[3] as u16;
        if a > 0 && a < 255 {
            px[0] = ((px[0] as u16 * 255 + a / 2) / a).min(255) as u8;
            px[1] = ((px[1] as u16 * 255 + a / 2) / a).min(255) as u8;
            px[2] = ((px[2] as u16 * 255 + a / 2) / a).min(255) as u8;
        }
    }
    RgbaImage::from_raw(w, h, data).expect("image")
}

/// Compose l'image de base et toutes les annotations.
pub fn render(base: &RgbaImage, shapes: &[Shape]) -> RgbaImage {
    let mut pm = to_pixmap(base);
    for s in shapes {
        draw_shape(&mut pm, s);
    }
    from_pixmap(pm)
}

/// Couleur de fond autour d'une zone : couleur la plus fréquente sur son pourtour (quantifiée).
pub fn dominant_border_color(img: &RgbaImage, r: Rect) -> [u8; 4] {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let (x0, y0) = ((r.left() as i32).clamp(0, w - 1), (r.top() as i32).clamp(0, h - 1));
    let (x1, y1) = ((r.right() as i32 - 1).clamp(0, w - 1), (r.bottom() as i32 - 1).clamp(0, h - 1));
    let mut counts: std::collections::HashMap<[u8; 3], (u32, [u32; 3])> = std::collections::HashMap::new();
    let mut add = |x: i32, y: i32| {
        let p = img.get_pixel(x as u32, y as u32).0;
        let e = counts.entry([p[0] >> 3, p[1] >> 3, p[2] >> 3]).or_insert((0, [0; 3]));
        e.0 += 1;
        for k in 0..3 {
            e.1[k] += p[k] as u32;
        }
    };
    for x in x0..=x1 {
        add(x, y0);
        add(x, y1);
    }
    for y in y0..=y1 {
        add(x0, y);
        add(x1, y);
    }
    counts
        .values()
        .max_by_key(|(n, _)| *n)
        .map(|(n, s)| [(s[0] / n) as u8, (s[1] / n) as u8, (s[2] / n) as u8, 255])
        .unwrap_or([255, 255, 255, 255])
}

/// Paramètres du raccord en zigzag (pixels).
pub const ZIG_AMPLITUDE: f32 = 10.0;
const ZIG_PERIOD: f32 = 22.0;
pub const ZIG_GAP: u32 = 8;

/// Décalage de la ligne de coupe au point `x` : onde triangulaire dans [-A/2, A/2].
fn zig(x: u32) -> i32 {
    let t = (x as f32 % ZIG_PERIOD) / ZIG_PERIOD;
    (ZIG_AMPLITUDE / 2.0 - ZIG_AMPLITUDE * (t * 2.0 - 1.0).abs()).round() as i32
}

/// Retire les lignes [y0, y1) et recolle les deux morceaux, bord à bord ou avec un raccord
/// en zigzag (deux bords dentelés parallèles séparés d'un liseré blanc).
pub fn cut_rows(img: &RgbaImage, y0: u32, y1: u32, zigzag: bool) -> RgbaImage {
    let (w, h) = img.dimensions();
    let gap = if zigzag { ZIG_GAP } else { 0 };
    let out_h = y0 + gap + (h - y1);
    let mut out = RgbaImage::from_pixel(w, out_h.max(1), image::Rgba([255, 255, 255, 255]));
    let edge = image::Rgba([150, 150, 150, 255]);
    for x in 0..w {
        let z = if zigzag { zig(x) } else { 0 };
        // Haut : lignes source identiques jusqu'à la ligne dentelée.
        let top_end = (y0 as i32 + z).clamp(0, out_h as i32) as u32;
        for y in 0..top_end {
            out.put_pixel(x, y, *img.get_pixel(x, y.min(h - 1)));
        }
        // Bas : décalé pour que la dernière ligne source tombe en bas de l'image.
        let bottom_start = (top_end + gap).min(out_h);
        for y in bottom_start..out_h {
            let sy = (y as i64 - (y0 + gap) as i64 + y1 as i64).clamp(0, h as i64 - 1) as u32;
            out.put_pixel(x, y, *img.get_pixel(x, sy));
        }
        if zigzag {
            if top_end > 0 {
                out.put_pixel(x, top_end - 1, edge);
            }
            if bottom_start < out_h {
                out.put_pixel(x, bottom_start, edge);
            }
        }
    }
    out
}

/// Retire les colonnes [x0, x1) (voir `cut_rows`).
pub fn cut_cols(img: &RgbaImage, x0: u32, x1: u32, zigzag: bool) -> RgbaImage {
    let t = transpose(img);
    transpose(&cut_rows(&t, x0, x1, zigzag))
}

fn transpose(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    RgbaImage::from_fn(h, w, |x, y| *img.get_pixel(y, x))
}

/// Effets d'export : bordure et ombre portée autour de l'image.
pub fn apply_effects(img: &RgbaImage, border: Option<(u32, [u8; 4])>, shadow: bool) -> RgbaImage {
    if border.is_none() && !shadow {
        return img.clone();
    }
    let mut src = to_pixmap(img);
    if let Some((bw, c)) = border {
        if let Some(p) = rect_path(Rect::from_min_size(Pos2::ZERO, eframe::egui::vec2(img.width() as f32, img.height() as f32)), 0.0) {
            let s = Stroke { width: bw as f32 * 2.0, ..Default::default() };
            src.stroke_path(&p, &paint(c), &s, Transform::identity(), None);
        }
    }
    if !shadow {
        return from_pixmap(src);
    }
    let margin = 24u32;
    let (w, h) = (img.width() + margin * 2, img.height() + margin * 2);
    let mut out = Pixmap::new(w, h).expect("pixmap");
    let base = Rect::from_min_size(Pos2::new(margin as f32 + 4.0, margin as f32 + 6.0), eframe::egui::vec2(img.width() as f32, img.height() as f32));
    for i in 0..12 {
        let e = (12 - i) as f32 * 1.5;
        if let Some(p) = rect_path(base.expand(e), e + 2.0) {
            out.fill_path(&p, &paint([0, 0, 0, 9]), FillRule::Winding, Transform::identity(), None);
        }
    }
    out.draw_pixmap(margin as i32, margin as i32, src.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
    from_pixmap(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Image où chaque ligne/colonne porte son indice, pour vérifier le recollage.
    fn ramp(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| image::Rgba([x as u8, y as u8, 0, 255]))
    }

    #[test]
    fn straight_cut_removes_band() {
        let out = cut_rows(&ramp(40, 100), 30, 60, false);
        assert_eq!(out.dimensions(), (40, 70));
        assert_eq!(out.get_pixel(5, 29).0[1], 29);
        assert_eq!(out.get_pixel(5, 30).0[1], 60);
        assert_eq!(out.get_pixel(5, 69).0[1], 99);
        let out = cut_cols(&ramp(100, 40), 10, 50, false);
        assert_eq!(out.dimensions(), (60, 40));
        assert_eq!(out.get_pixel(10, 3).0[0], 50);
    }

    #[test]
    fn zigzag_cut_keeps_ends() {
        let out = cut_rows(&ramp(60, 200), 50, 150, true);
        assert_eq!(out.dimensions(), (60, 50 + ZIG_GAP + 50));
        assert_eq!(out.get_pixel(7, 0).0[1], 0);
        assert_eq!(out.get_pixel(7, out.height() - 1).0[1], 199);
    }
    /// Police de repli intégrée (macOS sans Arial ni Helvetica lisibles) : le texte reste visible.
    #[test]
    fn police_de_repli() {
        let defs = eframe::egui::FontDefinitions::default();
        let d = &defs.font_data["Ubuntu-Light"];
        let f = FontVec::try_from_vec_and_index(d.font.to_vec(), d.index).expect("police intégrée");
        assert!(f.glyph_id('A').0 != 0);
        assert!(load_font(false).is_some() && load_font(true).is_some());
    }

    /// Bulles : chaque type se dessine, et la pointe déplacée colore le pixel visé.
    #[test]
    fn bulles_et_pointe() {
        use super::super::model::BubbleKind;
        let base = RgbaImage::from_pixel(400, 300, image::Rgba([255, 255, 255, 255]));
        for k in BubbleKind::ALL {
            let mut style = Style { bubble: k, shadow: false, color: [200, 0, 0, 255], ..Style::default() };
            style.tail = Some(eframe::egui::vec2(250.0, 200.0));
            let s = Shape { kind: Kind::Bubble("Bonjour".into()), a: eframe::egui::pos2(40.0, 40.0), b: eframe::egui::pos2(40.0, 40.0), style };
            assert_eq!(bubble_tip(&s), eframe::egui::pos2(290.0, 240.0));
            let out = render(&base, &[s.clone()]);
            let c = s.a + eframe::egui::vec2(text_box_size(&s).0 / 2.0, 4.0);
            let (w, h) = text_box_size(&s);
            let centre = s.a + eframe::egui::vec2(w / 2.0, h / 2.0);
            let q = if k == BubbleKind::Thought { bubble_tip(&s) } else { centre + (bubble_tip(&s) - centre) * 0.85 };
            let near_tip = out.get_pixel(q.x as u32, q.y as u32).0;
            assert!(near_tip[1] < 100, "{k:?} : pointe absente {near_tip:?}");
            assert!(out.get_pixel(c.x as u32, (c.y + text_box_size(&s).1 / 2.0) as u32).0[1] < 255, "{k:?} : corps absent");
        }
    }

    /// Planche des bulles (SC_BULLES_PNG=chemin pour l'écrire).
    #[test]
    fn planche_bulles() {
        use super::super::model::BubbleKind;
        let Ok(path) = std::env::var("SC_BULLES_PNG") else { return };
        let base = RgbaImage::from_pixel(900, 260, image::Rgba([240, 240, 240, 255]));
        let shapes: Vec<Shape> = BubbleKind::ALL
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let tail = if i % 2 == 0 { None } else { Some(eframe::egui::vec2(-30.0, 150.0)) };
                Shape {
                    kind: Kind::Bubble(k.label().into()),
                    a: eframe::egui::pos2(30.0 + i as f32 * 220.0, 30.0),
                    b: eframe::egui::pos2(0.0, 0.0),
                    style: Style { bubble: *k, tail, color: [30, 136, 229, 255], ..Style::default() },
                }
            })
            .collect();
        render(&base, &shapes).save(path).unwrap();
    }
}
