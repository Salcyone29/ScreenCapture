//! Thème Material Design 3 pour egui : jetons de couleur, typographie et composants.

use std::sync::Arc;

use eframe::egui::{
    self, epaint::Shadow, Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Id, Margin,
    Response, Sense, Stroke, StrokeKind, Ui, Vec2,
};

use crate::settings::{Accent, ThemeChoice};

/// Jetons de couleur Material 3.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub dark: bool,
    pub primary: Color32,
    pub on_primary: Color32,
    pub primary_container: Color32,
    pub on_primary_container: Color32,
    pub secondary_container: Color32,
    pub on_secondary_container: Color32,
    pub surface: Color32,
    pub surface_low: Color32,
    pub surface_container: Color32,
    pub surface_high: Color32,
    pub surface_highest: Color32,
    pub on_surface: Color32,
    pub on_surface_variant: Color32,
    pub outline: Color32,
    pub outline_variant: Color32,
    pub error: Color32,
    pub inverse_surface: Color32,
    pub inverse_on_surface: Color32,
}

const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

impl Palette {
    pub fn new(dark: bool, accent: Accent) -> Self {
        // (primary, on_primary, primary_container, on_primary_container, secondary_container, on_secondary_container)
        let (p, op, pc, opc, sc, osc) = match (accent, dark) {
            (Accent::Violet, false) => (0x6750A4, 0xFFFFFF, 0xEADDFF, 0x21005D, 0xE8DEF8, 0x1D192B),
            (Accent::Violet, true) => (0xD0BCFF, 0x381E72, 0x4F378B, 0xEADDFF, 0x4A4458, 0xE8DEF8),
            (Accent::Blue, false) => (0x0061A4, 0xFFFFFF, 0xD1E4FF, 0x001D36, 0xD7E3F7, 0x101C2B),
            (Accent::Blue, true) => (0x9ECAFF, 0x003258, 0x00497D, 0xD1E4FF, 0x3B4858, 0xD7E3F7),
            (Accent::Green, false) => (0x386A20, 0xFFFFFF, 0xB8F397, 0x042100, 0xD9E7CB, 0x131F0D),
            (Accent::Green, true) => (0x9DD67D, 0x0C3900, 0x205107, 0xB8F397, 0x3C4B35, 0xD9E7CB),
            (Accent::Orange, false) => (0x8B5000, 0xFFFFFF, 0xFFDCBE, 0x2C1600, 0xF2DFD1, 0x251A11),
            (Accent::Orange, true) => (0xFFB870, 0x4A2800, 0x693C00, 0xFFDCBE, 0x51443A, 0xF2DFD1),
            (Accent::Red, false) => (0xB3261E, 0xFFFFFF, 0xF9DEDC, 0x410E0B, 0xF5DDDA, 0x2C1512),
            (Accent::Red, true) => (0xF2B8B5, 0x601410, 0x8C1D18, 0xF9DEDC, 0x534341, 0xF5DDDA),
        };
        let base = Self {
            dark,
            primary: hex(p),
            on_primary: hex(op),
            primary_container: hex(pc),
            on_primary_container: hex(opc),
            secondary_container: hex(sc),
            on_secondary_container: hex(osc),
            surface: hex(0xFEF7FF),
            surface_low: hex(0xF7F2FA),
            surface_container: hex(0xF3EDF7),
            surface_high: hex(0xECE6F0),
            surface_highest: hex(0xE6E0E9),
            on_surface: hex(0x1D1B20),
            on_surface_variant: hex(0x49454F),
            outline: hex(0x79747E),
            outline_variant: hex(0xCAC4D0),
            error: hex(0xB3261E),
            inverse_surface: hex(0x322F35),
            inverse_on_surface: hex(0xF5EFF7),
        };
        if !dark {
            return base;
        }
        Self {
            surface: hex(0x141218),
            surface_low: hex(0x1D1B20),
            surface_container: hex(0x211F26),
            surface_high: hex(0x2B2930),
            surface_highest: hex(0x36343B),
            on_surface: hex(0xE6E0E9),
            on_surface_variant: hex(0xCAC4D0),
            outline: hex(0x938F99),
            outline_variant: hex(0x49454F),
            error: hex(0xF2B8B5),
            inverse_surface: hex(0xE6E0E9),
            inverse_on_surface: hex(0x322F35),
            ..base
        }
    }

    /// Couche d'état Material (survol 8 %, appui 12 %).
    pub fn state_layer(&self, base: Color32, on: Color32, resp: &Response) -> Color32 {
        let t = if resp.is_pointer_button_down_on() {
            0.12
        } else if resp.hovered() {
            0.08
        } else {
            0.0
        };
        blend(base, on, t)
    }
}

pub fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), a.a().max((t * 255.0) as u8))
}

/// Remplace le curseur système par une croix bicolore (contour sombre, cœur blanc),
/// lisible sur n'importe quel fond quel que soit le thème de pointeurs de Windows.
pub fn crosshair_cursor(ctx: &egui::Context) {
    let Some(c) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    ctx.set_cursor_icon(egui::CursorIcon::None);
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, Id::new("crosshair-cursor")));
    let (arm, gap) = (11.0_f32, 3.0_f32);
    let arms = [
        [egui::pos2(c.x - arm, c.y), egui::pos2(c.x - gap, c.y)],
        [egui::pos2(c.x + gap, c.y), egui::pos2(c.x + arm, c.y)],
        [egui::pos2(c.x, c.y - arm), egui::pos2(c.x, c.y - gap)],
        [egui::pos2(c.x, c.y + gap), egui::pos2(c.x, c.y + arm)],
    ];
    for seg in arms {
        painter.line_segment(seg, Stroke::new(3.0_f32, Color32::from_black_alpha(210)));
    }
    for seg in arms {
        painter.line_segment(seg, Stroke::new(1.0_f32, Color32::WHITE));
    }
    painter.circle_filled(c, 1.5, Color32::from_black_alpha(210));
    painter.circle_filled(c, 0.6, Color32::WHITE);
}

pub fn palette(ctx: &egui::Context) -> Palette {
    ctx.data(|d| d.get_temp::<Palette>(Id::NULL)).unwrap_or_else(|| Palette::new(false, Accent::Violet))
}

/// Charge Segoe UI (Windows) + la police d'icônes Material Symbols.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    for (name, file) in [("segoe", "segoeui.ttf"), ("segoe-semibold", "seguisb.ttf")] {
        if let Ok(bytes) = std::fs::read(format!("{windir}\\Fonts\\{file}")) {
            fonts.font_data.insert(name.into(), Arc::new(FontData::from_owned(bytes)));
        }
    }
    if fonts.font_data.contains_key("segoe") {
        fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "segoe".into());
    }
    if fonts.font_data.contains_key("segoe-semibold") {
        let mut fam = vec!["segoe-semibold".to_string()];
        fam.extend(fonts.families[&FontFamily::Proportional].iter().skip(1).cloned());
        fonts.families.insert(FontFamily::Name("medium".into()), fam);
    } else {
        let fam = fonts.families[&FontFamily::Proportional].clone();
        fonts.families.insert(FontFamily::Name("medium".into()), fam);
    }
    ctx.set_fonts(fonts);
    egui_material_icons::initialize(ctx);
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}

pub fn apply(ctx: &egui::Context, choice: ThemeChoice, accent: Accent) -> Palette {
    let dark = match choice {
        ThemeChoice::Dark => true,
        ThemeChoice::Light => false,
        ThemeChoice::System => ctx.system_theme().map(|t| t == egui::Theme::Dark).unwrap_or(false),
    };
    let p = Palette::new(dark, accent);
    ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });

    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;
    *v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.override_text_color = Some(p.on_surface);
    v.panel_fill = p.surface;
    v.window_fill = p.surface_high;
    v.window_stroke = Stroke::NONE;
    v.window_corner_radius = CornerRadius::same(28);
    v.menu_corner_radius = CornerRadius::same(8);
    v.window_shadow = Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(60) };
    v.popup_shadow = Shadow { offset: [0, 3], blur: 12, spread: 0, color: Color32::from_black_alpha(50) };
    v.extreme_bg_color = p.surface_highest;
    v.faint_bg_color = p.surface_low;
    v.code_bg_color = p.surface_high;
    v.hyperlink_color = p.primary;
    v.selection.bg_fill = blend(p.primary, p.surface, 0.55);
    v.selection.stroke = Stroke::new(1.5_f32, p.primary);
    v.text_cursor.stroke = Stroke::new(2.0_f32, p.primary);
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Circle;

    let r = CornerRadius::same(8);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.surface;
    w.noninteractive.weak_bg_fill = p.surface_container;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.outline_variant);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.on_surface);
    w.noninteractive.corner_radius = r;
    w.inactive.bg_fill = p.surface_highest;
    w.inactive.weak_bg_fill = p.surface_high;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, p.outline_variant);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, p.on_surface_variant);
    w.inactive.corner_radius = r;
    w.hovered.bg_fill = blend(p.surface_highest, p.on_surface, 0.08);
    w.hovered.weak_bg_fill = blend(p.surface_high, p.on_surface, 0.08);
    w.hovered.bg_stroke = Stroke::new(1.0_f32, p.outline);
    w.hovered.fg_stroke = Stroke::new(1.5_f32, p.on_surface);
    w.hovered.corner_radius = r;
    w.hovered.expansion = 0.0;
    w.active.bg_fill = p.primary;
    w.active.weak_bg_fill = blend(p.surface_high, p.on_surface, 0.12);
    w.active.bg_stroke = Stroke::new(1.0_f32, p.primary);
    w.active.fg_stroke = Stroke::new(2.0_f32, p.on_surface);
    w.active.corner_radius = r;
    w.active.expansion = 0.0;
    w.open = w.active;

    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.interact_size.y = 32.0;
    style.spacing.slider_width = 180.0;
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.window_margin = Margin::same(24);
    style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.0));
    style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.0));
    style.text_styles.insert(egui::TextStyle::Heading, medium(22.0));
    style.text_styles.insert(egui::TextStyle::Small, FontId::proportional(12.0));
    ctx.set_style(style);
    ctx.data_mut(|d| d.insert_temp(Id::NULL, p));
    p
}

// ---------------------------------------------------------------------------------------------
// Composants
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Filled,
    Tonal,
    Text,
    Danger,
}

/// Bouton Material (pilule 40 px) avec icône optionnelle.
pub fn button(ui: &mut Ui, kind: ButtonKind, icon: Option<&str>, label: &str) -> Response {
    let p = palette(ui.ctx());
    let (bg, fg, stroke) = match kind {
        ButtonKind::Filled => (p.primary, p.on_primary, Stroke::NONE),
        ButtonKind::Tonal => (p.secondary_container, p.on_secondary_container, Stroke::NONE),
        ButtonKind::Text => (Color32::TRANSPARENT, p.primary, Stroke::NONE),
        ButtonKind::Danger => (p.error, if p.dark { hex(0x601410) } else { Color32::WHITE }, Stroke::NONE),
    };
    let font = medium(14.0);
    let icon_font = FontId::proportional(18.0);
    let text_w = ui.fonts(|f| f.layout_no_wrap(label.to_owned(), font.clone(), fg).size().x);
    let icon_w = if icon.is_some() { 18.0 + if label.is_empty() { 0.0 } else { 8.0 } } else { 0.0 };
    let pad = if icon.is_some() { 16.0 } else { 24.0 };
    let size = Vec2::new(pad + icon_w + text_w + 24.0, 40.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let base = if bg == Color32::TRANSPARENT { Color32::from_rgba_unmultiplied(fg.r(), fg.g(), fg.b(), 0) } else { bg };
        let fill = if bg == Color32::TRANSPARENT {
            let t = if resp.is_pointer_button_down_on() { 0.12 } else if resp.hovered() { 0.08 } else { 0.0 };
            Color32::from_rgba_unmultiplied(fg.r(), fg.g(), fg.b(), (t * 255.0) as u8)
        } else {
            p.state_layer(base, fg, &resp)
        };
        let painter = ui.painter();
        if kind == ButtonKind::Filled && resp.hovered() {
            painter.rect_filled(rect.translate(Vec2::new(0.0, 1.0)), 20.0, Color32::from_black_alpha(40));
        }
        painter.rect(rect, 20.0, fill, stroke, StrokeKind::Inside);
        let mut x = rect.left() + pad;
        if let Some(icon) = icon {
            painter.text(egui::pos2(x + 9.0, rect.center().y), Align2::CENTER_CENTER, icon, icon_font, fg);
            x += icon_w;
        }
        painter.text(egui::pos2(x, rect.center().y), Align2::LEFT_CENTER, label, font, fg);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Bouton icône standard 40 px (ou « toggle » si `selected`).
pub fn icon_button(ui: &mut Ui, icon: &str, selected: bool, tooltip: &str) -> Response {
    let p = palette(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let (bg, fg) = if selected { (p.secondary_container, p.on_secondary_container) } else { (Color32::TRANSPARENT, p.on_surface_variant) };
        let fill = if selected {
            p.state_layer(bg, fg, &resp)
        } else {
            let t = if resp.is_pointer_button_down_on() { 0.12 } else if resp.hovered() { 0.08 } else { 0.0 };
            Color32::from_rgba_unmultiplied(fg.r(), fg.g(), fg.b(), (t * 255.0) as u8)
        };
        ui.painter().circle_filled(rect.center(), 20.0, fill);
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, icon, FontId::proportional(22.0), fg);
    }
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if tooltip.is_empty() {
        resp
    } else {
        resp.on_hover_text(tooltip)
    }
}

/// Élément de rail de navigation (indicateur pilule + libellé).
pub fn nav_item(ui: &mut Ui, icon: &str, label: &str, selected: bool) -> Response {
    let p = palette(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(80.0, 60.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let pill = egui::Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 18.0), Vec2::new(56.0, 32.0));
        let fill = if selected {
            p.state_layer(p.secondary_container, p.on_secondary_container, &resp)
        } else {
            let t = if resp.hovered() { 0.08 } else { 0.0 };
            Color32::from_rgba_unmultiplied(p.on_surface.r(), p.on_surface.g(), p.on_surface.b(), (t * 255.0) as u8)
        };
        ui.painter().rect_filled(pill, 16.0, fill);
        let fg = if selected { p.on_secondary_container } else { p.on_surface_variant };
        ui.painter().text(pill.center(), Align2::CENTER_CENTER, icon, FontId::proportional(24.0), fg);
        let lf = if selected { medium(12.0) } else { FontId::proportional(12.0) };
        ui.painter().text(egui::pos2(rect.center().x, rect.top() + 46.0), Align2::CENTER_CENTER, label, lf, if selected { p.on_surface } else { p.on_surface_variant });
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Carte « action de capture » : grande icône dans un conteneur tonal + libellé + raccourci.
pub fn action_card(ui: &mut Ui, icon: &str, title: &str, subtitle: &str, emphasized: bool) -> Response {
    let p = palette(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(148.0, 112.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let base = if emphasized { p.primary_container } else { p.surface_low };
        let on = if emphasized { p.on_primary_container } else { p.on_surface };
        let fill = p.state_layer(base, on, &resp);
        if resp.hovered() {
            ui.painter().rect_filled(rect.translate(Vec2::new(0.0, 2.0)), 16.0, Color32::from_black_alpha(30));
        }
        ui.painter().rect(rect, 16.0, fill, if emphasized { Stroke::NONE } else { Stroke::new(1.0_f32, p.outline_variant) }, StrokeKind::Inside);
        let ic = egui::pos2(rect.left() + 32.0, rect.top() + 32.0);
        ui.painter().circle_filled(ic, 20.0, if emphasized { p.primary } else { p.secondary_container });
        ui.painter().text(ic, Align2::CENTER_CENTER, icon, FontId::proportional(22.0), if emphasized { p.on_primary } else { p.on_secondary_container });
        ui.painter().text(egui::pos2(rect.left() + 14.0, rect.bottom() - 36.0), Align2::LEFT_CENTER, title, medium(14.0), on);
        ui.painter().text(egui::pos2(rect.left() + 14.0, rect.bottom() - 16.0), Align2::LEFT_CENTER, subtitle, FontId::proportional(11.5), if emphasized { on } else { p.on_surface_variant });
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Puce de filtre Material.
pub fn filter_chip(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let p = palette(ui.ctx());
    let font = medium(13.0);
    let tw = ui.fonts(|f| f.layout_no_wrap(label.to_owned(), font.clone(), p.on_surface).size().x);
    let check = if selected { 26.0 } else { 0.0 };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(tw + 32.0 + check, 32.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let (bg, fg, stroke) = if selected {
            (p.secondary_container, p.on_secondary_container, Stroke::NONE)
        } else {
            (p.surface, p.on_surface_variant, Stroke::new(1.0_f32, p.outline_variant))
        };
        ui.painter().rect(rect, 8.0, p.state_layer(bg, fg, &resp), stroke, StrokeKind::Inside);
        let mut x = rect.left() + 16.0;
        if selected {
            ui.painter().text(egui::pos2(x + 8.0, rect.center().y), Align2::CENTER_CENTER, egui_material_icons::icons::ICON_CHECK, FontId::proportional(18.0), fg);
            x += check;
        }
        ui.painter().text(egui::pos2(x, rect.center().y), Align2::LEFT_CENTER, label, font, fg);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Interrupteur Material 3.
pub fn switch(ui: &mut Ui, on: &mut bool) -> Response {
    let p = palette(ui.ctx());
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(52.0, 32.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_with_time(resp.id, *on, 0.12);
    if ui.is_rect_visible(rect) {
        let track = blend(p.surface_highest, p.primary, t);
        let stroke = if *on { Stroke::NONE } else { Stroke::new(2.0_f32, p.outline) };
        ui.painter().rect(rect, 16.0, track, stroke, StrokeKind::Inside);
        let r = 8.0 + 4.0 * t + if resp.is_pointer_button_down_on() { 2.0 } else { 0.0 };
        let cx = egui::lerp((rect.left() + 16.0)..=(rect.right() - 16.0), t);
        let knob = blend(p.outline, p.on_primary, t);
        if resp.hovered() {
            ui.painter().circle_filled(egui::pos2(cx, rect.center().y), 20.0, Color32::from_rgba_unmultiplied(p.on_surface.r(), p.on_surface.g(), p.on_surface.b(), 20));
        }
        ui.painter().circle_filled(egui::pos2(cx, rect.center().y), r, knob);
        if *on {
            ui.painter().text(egui::pos2(cx, rect.center().y), Align2::CENTER_CENTER, egui_material_icons::icons::ICON_CHECK, FontId::proportional(14.0), p.primary);
        }
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Ligne de paramètre : titre + description à gauche, contrôle à droite.
pub fn setting_row(ui: &mut Ui, title: &str, desc: &str, control: impl FnOnce(&mut Ui)) {
    let p = palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.set_min_height(48.0);
        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(title).font(FontId::proportional(15.0)).color(p.on_surface));
            if !desc.is_empty() {
                ui.label(egui::RichText::new(desc).size(12.5).color(p.on_surface_variant));
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), control);
    });
}

/// Carte (surface container) avec titre de section.
pub fn card<R>(ui: &mut Ui, title: &str, icon: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .fill(p.surface_low)
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::same(20))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(icon).size(20.0).color(p.primary));
                ui.label(egui::RichText::new(title).font(medium(16.0)).color(p.on_surface));
            });
            ui.add_space(6.0);
            add(ui)
        })
        .inner
}

/// Variante d'entrée texte « outlined ».
pub fn text_field(ui: &mut Ui, value: &mut String, width: f32, hint: &str) -> Response {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .stroke(Stroke::new(1.0_f32, p.outline))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.add(egui::TextEdit::singleline(value).desired_width(width).frame(false).hint_text(hint))
        })
        .inner
}

/// Pastille de couleur sélectionnable.
pub fn swatch(ui: &mut Ui, color: Color32, selected: bool) -> Response {
    let p = palette(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let c = rect.center();
        if selected {
            ui.painter().circle_stroke(c, 13.0, Stroke::new(2.0_f32, p.primary));
        }
        if color.a() < 255 {
            ui.painter().circle_filled(c, 10.0, Color32::WHITE);
            ui.painter().line_segment([c + Vec2::new(-7.0, 7.0), c + Vec2::new(7.0, -7.0)], Stroke::new(2.0_f32, Color32::from_rgb(220, 50, 50)));
        }
        ui.painter().circle(c, 10.0, color, Stroke::new(1.0_f32, p.outline_variant));
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}
