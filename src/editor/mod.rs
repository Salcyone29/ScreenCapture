//! Éditeur d'images : outils d'annotation, rognage, rotation, redimensionnement, annuler/rétablir.

pub mod model;
pub mod render;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, FontId, Key, Margin, PointerButton, Pos2, Rect, Sense, Stroke,
    StrokeKind, TextureOptions, Ui, Vec2,
};
use egui_material_icons::icons::*;
use image::RgbaImage;

use crate::settings::Settings;
use crate::texture::TiledTexture;
use crate::theme::{self, palette, ButtonKind};
use model::{snap45, snap_square, CutDir, CutEdge, Kind, NumberFormat, Shape, Style, Tool, COLORS, STAMPS};

pub enum EditorAction {
    Close,
    Save,
    SaveAs,
    Copy,
    Ocr,
}

#[derive(Clone)]
struct Snap {
    base: Arc<RgbaImage>,
    shapes: Vec<Shape>,
    next_number: u32,
}

enum Drag {
    None,
    Creating(usize),
    Moving { idx: usize, last: Pos2 },
    Handle { idx: usize, first: bool },
    Crop { start: Pos2 },
    Cut { start: f32 },
    Area { start: Pos2 },
    Pan,
}

pub struct Editor {
    pub path: Option<PathBuf>,
    pub modified: bool,
    base: Arc<RgbaImage>,
    shapes: Vec<Shape>,
    undo: Vec<Snap>,
    redo: Vec<Snap>,
    tool: Tool,
    style: Style,
    tool_styles: HashMap<Tool, Style>,
    /// Vrai dès que l'utilisateur a choisi une couleur : elle s'applique alors à tous les outils.
    color_chosen: bool,
    selected: Option<usize>,
    next_number: u32,
    number_format: NumberFormat,
    stamp: usize,
    composite: RgbaImage,
    texture: Option<TiledTexture>,
    dirty: bool,
    zoom: f32,
    pan: Vec2,
    fit: bool,
    drag: Drag,
    editing: Option<usize>,
    text_buf: String,
    focus_text: bool,
    crop: Option<Rect>,
    /// Bande à retirer (début, fin) le long de l'axe de découpe, en attente de confirmation.
    cut: Option<(f32, f32)>,
    cut_dir: CutDir,
    cut_edge: CutEdge,
    /// Rectangle en cours de sélection par l'outil Déplacer une zone.
    area_sel: Option<Rect>,
    resize_pct: Option<u32>,
    style_edit_open: bool,
    hover_px: Option<(u32, u32)>,
}

impl Editor {
    pub fn new(img: RgbaImage, path: Option<PathBuf>) -> Self {
        Self {
            path,
            modified: false,
            composite: img.clone(),
            base: Arc::new(img),
            shapes: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            tool: Tool::Arrow,
            style: Style::for_tool(Tool::Arrow),
            tool_styles: HashMap::new(),
            color_chosen: false,
            selected: None,
            next_number: 1,
            number_format: NumberFormat::Decimal,
            stamp: 0,
            texture: None,
            dirty: true,
            zoom: 1.0,
            pan: Vec2::ZERO,
            fit: true,
            drag: Drag::None,
            editing: None,
            text_buf: String::new(),
            focus_text: false,
            crop: None,
            cut: None,
            cut_dir: CutDir::Horizontal,
            cut_edge: CutEdge::Zigzag,
            area_sel: None,
            resize_pct: None,
            style_edit_open: false,
            hover_px: None,
        }
    }

    pub fn title(&self) -> String {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Nouvelle capture".into());
        format!("{}{}", name, if self.modified { " •" } else { "" })
    }

    /// Image finale (annotations + effets d'export).
    pub fn export_image(&mut self, settings: &Settings) -> RgbaImage {
        self.commit_text();
        let img = render::render(&self.base, &self.shapes);
        let border = settings.effect_border.then_some((settings.effect_border_width, settings.effect_border_color));
        render::apply_effects(&img, border, settings.effect_shadow)
    }

    // ------------------------------------------------------------------ historique

    fn snapshot(&self) -> Snap {
        Snap { base: self.base.clone(), shapes: self.shapes.clone(), next_number: self.next_number }
    }

    fn push_undo(&mut self) {
        self.undo.push(self.snapshot());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.modified = true;
    }

    fn restore(&mut self, s: Snap) {
        self.base = s.base;
        self.shapes = s.shapes;
        self.next_number = s.next_number;
        self.selected = None;
        self.editing = None;
        self.dirty = true;
        self.modified = true;
    }

    fn undo(&mut self) {
        self.commit_text();
        if let Some(s) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(s);
        }
    }

    fn redo(&mut self) {
        if let Some(s) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.restore(s);
        }
    }

    // ------------------------------------------------------------------ transformations

    fn rotate(&mut self, clockwise: bool) {
        self.commit_text();
        self.push_undo();
        self.bake_patches();
        let (w, h) = (self.base.width() as f32, self.base.height() as f32);
        let img = if clockwise { image::imageops::rotate90(&*self.base) } else { image::imageops::rotate270(&*self.base) };
        let f = move |p: Pos2| if clockwise { pos2(h - p.y, p.x) } else { pos2(p.y, w - p.x) };
        for s in &mut self.shapes {
            if matches!(s.kind, Kind::Text(_) | Kind::Bubble(_) | Kind::Stamp(_)) {
                let size = s.rect().size();
                let c = f(s.rect().center());
                s.a = c - size / 2.0;
                s.b = c + size / 2.0;
            } else {
                s.map_points(f);
            }
        }
        self.base = Arc::new(img);
        self.dirty = true;
        self.fit = true;
    }

    /// Intègre les zones déplacées à l'image de base : nécessaire avant toute transformation
    /// de l'image (rotation, rognage, découpe…), qui invaliderait leurs coordonnées source.
    fn bake_patches(&mut self) {
        if !self.shapes.iter().any(|s| matches!(s.kind, Kind::Patch { .. })) {
            return;
        }
        let (patches, others): (Vec<Shape>, Vec<Shape>) =
            std::mem::take(&mut self.shapes).into_iter().partition(|s| matches!(s.kind, Kind::Patch { .. }));
        self.base = Arc::new(render::render(&self.base, &patches));
        self.shapes = others;
        self.selected = None;
    }

    /// Crée une zone déplaçable à partir du rectangle `r` de l'image.
    fn create_patch(&mut self, r: Rect) {
        let full = Rect::from_min_size(Pos2::ZERO, vec2(self.base.width() as f32, self.base.height() as f32));
        let r = r.intersect(full);
        if r.width() < 4.0 || r.height() < 4.0 {
            return;
        }
        self.commit_text();
        self.push_undo();
        // Les zones restent sous les annotations ; elles copient l'image telle que modifiée
        // par les zones déjà déplacées.
        let k = self.shapes.iter().take_while(|s| matches!(s.kind, Kind::Patch { .. })).count();
        let src = render::render(&self.base, &self.shapes[..k]);
        let img = image::imageops::crop_imm(&src, r.left() as u32, r.top() as u32, r.width() as u32, r.height() as u32).to_image();
        let fill = render::dominant_border_color(&src, r);
        self.shapes.insert(k, Shape { kind: Kind::Patch { img: Arc::new(img), src: r, fill }, a: r.min, b: r.max, style: self.style.clone() });
        self.selected = Some(k);
        self.dirty = true;
    }

    /// Retire la bande [a, b) (lignes ou colonnes selon `cut_dir`) et recolle l'image.
    fn apply_cut(&mut self, a: f32, b: f32) {
        let horizontal = self.cut_dir == CutDir::Horizontal;
        let len = if horizontal { self.base.height() } else { self.base.width() };
        let (v0, v1) = (a.max(0.0).round() as u32, (b.round() as u32).min(len));
        if v1 < v0 + 2 || v1 - v0 >= len.saturating_sub(1) {
            return;
        }
        self.commit_text();
        self.push_undo();
        self.bake_patches();
        let zigzag = self.cut_edge == CutEdge::Zigzag;
        self.base = Arc::new(if horizontal {
            render::cut_rows(&self.base, v0, v1, zigzag)
        } else {
            render::cut_cols(&self.base, v0, v1, zigzag)
        });
        let gap = if zigzag { render::ZIG_GAP as f32 } else { 0.0 };
        let (v0, v1) = (v0 as f32, v1 as f32);
        let map = move |v: f32| if v < v0 { v } else if v >= v1 { v - (v1 - v0) + gap } else { v0 + gap / 2.0 };
        for s in &mut self.shapes {
            if matches!(s.kind, Kind::Text(_) | Kind::Bubble(_) | Kind::Stamp(_) | Kind::Number(..)) {
                // Formes ancrées sur `a` : translation, sans déformation.
                let d = if horizontal { vec2(0.0, map(s.a.y) - s.a.y) } else { vec2(map(s.a.x) - s.a.x, 0.0) };
                s.translate(d);
            } else if horizontal {
                s.map_points(|p| pos2(p.x, map(p.y)));
            } else {
                s.map_points(|p| pos2(map(p.x), p.y));
            }
        }
        self.dirty = true;
        self.fit = true;
    }

    fn apply_crop(&mut self, r: Rect) {
        let full = Rect::from_min_size(Pos2::ZERO, vec2(self.base.width() as f32, self.base.height() as f32));
        let r = r.intersect(full);
        if r.width() < 2.0 || r.height() < 2.0 {
            return;
        }
        self.commit_text();
        self.push_undo();
        self.bake_patches();
        let img = image::imageops::crop_imm(&*self.base, r.left() as u32, r.top() as u32, r.width() as u32, r.height() as u32).to_image();
        let off = r.min.to_vec2();
        for s in &mut self.shapes {
            s.translate(-off);
        }
        self.base = Arc::new(img);
        self.dirty = true;
        self.fit = true;
    }

    fn resize(&mut self, pct: u32) {
        if pct == 100 || pct == 0 {
            return;
        }
        self.commit_text();
        self.push_undo();
        self.bake_patches();
        let k = pct as f32 / 100.0;
        let (w, h) = (((self.base.width() as f32 * k) as u32).max(1), ((self.base.height() as f32 * k) as u32).max(1));
        let img = image::imageops::resize(&*self.base, w, h, image::imageops::FilterType::Lanczos3);
        for s in &mut self.shapes {
            s.map_points(|p| pos2(p.x * k, p.y * k));
            s.style.width *= k;
            s.style.font_size *= k;
            if matches!(s.kind, Kind::Text(_) | Kind::Bubble(_) | Kind::Stamp(_)) {
                let (bw, bh) = render::text_box_size(s);
                s.b = s.a + vec2(bw, bh);
            }
        }
        self.base = Arc::new(img);
        self.dirty = true;
        self.fit = true;
    }

    // ------------------------------------------------------------------ texte

    fn begin_text(&mut self, idx: usize) {
        self.editing = Some(idx);
        self.text_buf = match &self.shapes[idx].kind {
            Kind::Text(t) | Kind::Bubble(t) => t.clone(),
            _ => String::new(),
        };
        self.focus_text = true;
        self.selected = Some(idx);
        self.dirty = true;
    }

    fn commit_text(&mut self) {
        let Some(idx) = self.editing.take() else { return };
        if idx >= self.shapes.len() {
            return;
        }
        let text = self.text_buf.trim_end().to_string();
        if text.is_empty() {
            self.shapes.remove(idx);
            self.selected = None;
        } else {
            let s = &mut self.shapes[idx];
            match &mut s.kind {
                Kind::Text(t) | Kind::Bubble(t) => *t = text,
                _ => {}
            }
            let (w, h) = render::text_box_size(s);
            s.b = s.a + vec2(w, h);
        }
        self.dirty = true;
    }

    // ------------------------------------------------------------------ UI

    pub fn ui(&mut self, ctx: &egui::Context, settings: &mut Settings) -> Vec<EditorAction> {
        let mut actions = Vec::new();
        let p = palette(ctx);
        self.shortcuts(ctx, &mut actions);

        egui::TopBottomPanel::top("ed_top")
            .exact_height(64.0)
            .frame(egui::Frame::new().fill(p.surface_container).inner_margin(Margin::symmetric(12, 12)))
            .show(ctx, |ui| self.top_bar(ui, &mut actions));

        egui::SidePanel::left("ed_tools")
            .exact_width(64.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(p.surface_container).inner_margin(Margin::symmetric(12, 8)))
            .show(ctx, |ui| self.tool_rail(ui));

        egui::SidePanel::right("ed_props")
            .exact_width(272.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(p.surface_container).inner_margin(Margin::same(12)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.properties(ui, settings));
            });

        egui::TopBottomPanel::bottom("ed_status")
            .exact_height(28.0)
            .frame(egui::Frame::new().fill(p.surface_container).inner_margin(Margin::symmetric(16, 4)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let ppp = ctx.pixels_per_point();
                    ui.label(egui::RichText::new(format!("{} × {} px", self.base.width(), self.base.height())).size(12.0).color(p.on_surface_variant));
                    ui.separator();
                    ui.label(egui::RichText::new(format!("{:.0} %", self.zoom * ppp * 100.0)).size(12.0).color(p.on_surface_variant));
                    if let Some((x, y)) = self.hover_px {
                        ui.separator();
                        ui.label(egui::RichText::new(format!("{x}, {y}")).size(12.0).color(p.on_surface_variant));
                    }
                    ui.separator();
                    ui.label(egui::RichText::new(self.hint()).size(12.0).color(p.on_surface_variant));
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.surface_highest))
            .show(ctx, |ui| self.canvas(ui));

        self.text_overlay(ctx);
        self.resize_dialog(ctx);

        if self.dirty {
            self.dirty = false;
            let shapes: Vec<Shape> = self
                .shapes
                .iter()
                .enumerate()
                .filter(|(i, _)| Some(*i) != self.editing)
                .map(|(_, s)| s.clone())
                .collect();
            self.composite = render::render(&self.base, &shapes);
            match &mut self.texture {
                Some(t) => t.update(ctx, "editor", &self.composite, TextureOptions::LINEAR),
                None => self.texture = Some(TiledTexture::new(ctx, "editor", &self.composite, TextureOptions::LINEAR)),
            }
        }
        actions
    }

    fn hint(&self) -> &'static str {
        if self.crop.is_some() {
            return "Entrée : appliquer le rognage • Échap : annuler";
        }
        if self.cut.is_some() {
            return "Entrée : retirer la bande • Échap : annuler";
        }
        match self.tool {
            Tool::Select => "Cliquer pour sélectionner • Glisser pour déplacer • Suppr pour effacer",
            Tool::Arrow | Tool::Line => "Glisser pour tracer • Maj : angles de 45°",
            Tool::Rect | Tool::Ellipse => "Glisser pour tracer • Maj : forme régulière",
            Tool::Text | Tool::Bubble if cfg!(target_os = "macos") => "Cliquer pour écrire • ⌘Entrée ou Échap pour valider",
            Tool::Text | Tool::Bubble => "Cliquer pour écrire • Ctrl+Entrée ou Échap pour valider",
            Tool::Number => "Cliquer pour placer le numéro suivant",
            Tool::Stamp => "Cliquer pour apposer le tampon",
            Tool::Picker => "Cliquer pour prélever une couleur",
            Tool::Crop => "Glisser pour définir la zone à conserver",
            Tool::Cut => "Glisser pour choisir la bande à retirer",
            Tool::MoveArea => "Glisser pour sélectionner une zone, puis la faire glisser • Suppr : la remettre en place",
            _ if cfg!(target_os = "macos") => "Glisser pour tracer • Défiler : déplacer • ⌘+défiler ou pincer : zoom",
            _ => "Glisser pour tracer • Molette : défiler • Ctrl+molette : zoom",
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context, actions: &mut Vec<EditorAction>) {
        if self.editing.is_some() || ctx.wants_keyboard_input() {
            return;
        }
        let (ctrl, shift) = ctx.input(|i| (i.modifiers.command, i.modifiers.shift));
        let pressed = |k: Key| ctx.input(|i| i.key_pressed(k));
        if ctrl {
            if pressed(Key::Z) {
                if shift { self.redo() } else { self.undo() }
            }
            if pressed(Key::Y) {
                self.redo();
            }
            if pressed(Key::C) {
                actions.push(EditorAction::Copy);
            }
            if pressed(Key::S) {
                actions.push(if shift { EditorAction::SaveAs } else { EditorAction::Save });
            }
            if pressed(Key::Num0) {
                self.fit = true;
            }
            return;
        }
        if pressed(Key::Escape) {
            if self.crop.take().is_none() && self.cut.take().is_none() {
                self.selected = None;
            }
        }
        if pressed(Key::Enter) {
            if let Some(r) = self.crop.take() {
                self.apply_crop(r);
            }
            if let Some((a, b)) = self.cut.take() {
                self.apply_cut(a, b);
            }
        }
        if pressed(Key::Delete) || pressed(Key::Backspace) {
            if let Some(i) = self.selected.take() {
                if i < self.shapes.len() {
                    self.push_undo();
                    self.shapes.remove(i);
                    self.dirty = true;
                }
            }
        }
        if let Some(i) = self.selected {
            let step = if ctx.input(|i| i.modifiers.shift) { 10.0 } else { 1.0 };
            let mut d = Vec2::ZERO;
            if pressed(Key::ArrowLeft) { d.x -= step }
            if pressed(Key::ArrowRight) { d.x += step }
            if pressed(Key::ArrowUp) { d.y -= step }
            if pressed(Key::ArrowDown) { d.y += step }
            if d != Vec2::ZERO && i < self.shapes.len() {
                self.push_undo();
                self.shapes[i].translate(d);
                self.dirty = true;
            }
        }
        let keys = [
            (Key::V, Tool::Select), (Key::A, Tool::Arrow), (Key::L, Tool::Line), (Key::R, Tool::Rect),
            (Key::E, Tool::Ellipse), (Key::H, Tool::Highlight), (Key::B, Tool::Blur), (Key::T, Tool::Text),
            (Key::U, Tool::Bubble), (Key::N, Tool::Number), (Key::P, Tool::Pen), (Key::M, Tool::Magnifier),
            (Key::S, Tool::Stamp), (Key::C, Tool::Crop), (Key::X, Tool::Cut), (Key::D, Tool::MoveArea), (Key::I, Tool::Picker),
        ];
        for (k, t) in keys {
            if pressed(k) {
                self.set_tool(t);
            }
        }
    }

    fn set_tool(&mut self, t: Tool) {
        self.commit_text();
        if t == self.tool {
            return;
        }
        // Épaisseur, police, etc. restent propres à chaque outil, mais la couleur
        // courante (choisie au nuancier ou à la pipette) suit d'un outil à l'autre.
        let color = self.style.color;
        self.tool_styles.insert(self.tool, self.style.clone());
        self.style = self.tool_styles.get(&t).cloned().unwrap_or_else(|| Style::for_tool(t));
        if self.color_chosen {
            self.style.color = color;
        }
        self.tool = t;
        self.crop = None;
        self.cut = None;
        self.area_sel = None;
        if t != Tool::Select {
            self.selected = None;
        }
    }

    fn top_bar(&mut self, ui: &mut Ui, actions: &mut Vec<EditorAction>) {
        let p = palette(ui.ctx());
        ui.horizontal_centered(|ui| {
            if theme::icon_button(ui, ICON_ARROW_BACK, false, "Retour à l'espace de travail").clicked() {
                actions.push(EditorAction::Close);
            }
            ui.add_space(4.0);
            let mut title = self.title();
            if title.chars().count() > 34 {
                title = format!("{}…", title.chars().take(33).collect::<String>());
            }
            ui.label(egui::RichText::new(title).font(theme::medium(18.0)).color(p.on_surface));
            ui.add_space(16.0);
            ui.add_enabled_ui(!self.undo.is_empty(), |ui| {
                if theme::icon_button(ui, ICON_UNDO, false, &crate::platform::sc("Annuler (Ctrl+Z)")).clicked() {
                    self.undo();
                }
            });
            ui.add_enabled_ui(!self.redo.is_empty(), |ui| {
                if theme::icon_button(ui, ICON_REDO, false, &crate::platform::sc("Rétablir (Ctrl+Y)")).clicked() {
                    self.redo();
                }
            });
            ui.separator();
            if theme::icon_button(ui, ICON_ZOOM_OUT, false, "Zoom arrière").clicked() {
                self.fit = false;
                self.zoom = (self.zoom / 1.25).max(0.05);
            }
            if theme::icon_button(ui, ICON_FIT_SCREEN, self.fit, &crate::platform::sc("Ajuster à la fenêtre (Ctrl+0)")).clicked() {
                self.fit = true;
                self.pan = Vec2::ZERO;
            }
            if theme::icon_button(ui, ICON_ZOOM_IN, false, "Zoom avant").clicked() {
                self.fit = false;
                self.zoom = (self.zoom * 1.25).min(16.0);
            }
            ui.separator();
            if theme::icon_button(ui, ICON_ROTATE_LEFT, false, "Pivoter à gauche").clicked() {
                self.rotate(false);
            }
            if theme::icon_button(ui, ICON_ROTATE_RIGHT, false, "Pivoter à droite").clicked() {
                self.rotate(true);
            }
            if theme::icon_button(ui, ICON_PHOTO_SIZE_SELECT_LARGE, false, "Redimensionner").clicked() {
                self.resize_pct = Some(50);
            }
            ui.separator();
            if theme::icon_button(ui, ICON_DOCUMENT_SCANNER, false, "Extraire le texte (OCR Windows)").clicked() {
                actions.push(EditorAction::Ocr);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::button(ui, ButtonKind::Filled, Some(ICON_SAVE), "Enregistrer").on_hover_text(crate::platform::sc("Ctrl+S")).clicked() {
                    actions.push(EditorAction::Save);
                }
                if theme::icon_button(ui, ICON_SAVE_AS, false, &crate::platform::sc("Enregistrer sous… (Ctrl+Maj+S)")).clicked() {
                    actions.push(EditorAction::SaveAs);
                }
                if theme::button(ui, ButtonKind::Tonal, Some(ICON_CONTENT_COPY), "Copier").on_hover_text(crate::platform::sc("Ctrl+C")).clicked() {
                    actions.push(EditorAction::Copy);
                }
            });
        });
    }

    fn tool_rail(&mut self, ui: &mut Ui) {
        // Défilement si la fenêtre est trop basse pour afficher tous les outils.
        egui::ScrollArea::vertical().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                for t in Tool::ALL {
                    if theme::icon_button(ui, t.icon(), self.tool == t, t.label()).clicked() {
                        self.set_tool(t);
                    }
                }
            });
        });
    }

    fn properties(&mut self, ui: &mut Ui, settings: &mut Settings) {
        let p = palette(ui.ctx());
        let section = |ui: &mut Ui, t: &str| {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(t).font(theme::medium(13.0)).color(p.primary));
        };

        // Le panneau édite la forme sélectionnée, sinon le style des prochaines formes de l'outil.
        let selected = self.selected.filter(|i| *i < self.shapes.len());
        let tool_style = self.style.clone();
        if let Some(i) = selected {
            self.style = self.shapes[i].style.clone();
        }
        let before = self.style.clone();
        // Curseurs à la largeur du panneau (moins la zone de valeur) pour ne jamais déborder.
        ui.spacing_mut().slider_width = (ui.available_width() - 76.0).max(60.0);

        section(ui, if self.selected.is_some() { "Forme sélectionnée" } else { "Style" });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
            let colors = if settings.palette.is_empty() { COLORS.to_vec() } else { settings.palette.clone() };
            for c in colors {
                if theme::swatch(ui, Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]), self.style.color == c)
                    .on_hover_text(if c[3] == 0 { "Transparent" } else { "" })
                    .clicked()
                {
                    self.style.color = c;
                    self.color_chosen = true;
                }
            }
            let mut c32 = Color32::from_rgba_unmultiplied(self.style.color[0], self.style.color[1], self.style.color[2], self.style.color[3]);
            if egui::widgets::color_picker::color_edit_button_srgba(ui, &mut c32, egui::widgets::color_picker::Alpha::OnlyBlend).changed() {
                self.style.color = c32.to_srgba_unmultiplied();
                self.color_chosen = true;
            }
        });

        section(ui, "Épaisseur");
        ui.add(egui::Slider::new(&mut self.style.width, 1.0..=40.0).suffix(" px"));
        section(ui, "Taille du texte");
        ui.add(egui::Slider::new(&mut self.style.font_size, 10.0..=120.0).suffix(" px"));

        ui.add_space(4.0);
        for (label, val) in [("Remplissage", 0), ("Ombre portée", 1), ("Pointillés", 2)] {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let v = match val {
                        0 => &mut self.style.filled,
                        1 => &mut self.style.shadow,
                        _ => &mut self.style.dashed,
                    };
                    theme::switch(ui, v);
                });
            });
        }

        if self.style != before {
            if let Some(i) = self.selected.filter(|i| *i < self.shapes.len()) {
                if !self.style_edit_open {
                    self.push_undo();
                    self.style_edit_open = true;
                }
                self.shapes[i].style = self.style.clone();
                if matches!(self.shapes[i].kind, Kind::Text(_) | Kind::Bubble(_) | Kind::Stamp(_)) {
                    let (w, h) = render::text_box_size(&self.shapes[i]);
                    self.shapes[i].b = self.shapes[i].a + vec2(w, h);
                }
                self.dirty = true;
                // Les réglages de l'outil ne prennent pas ceux de la forme, sauf la couleur.
                let color = self.style.color;
                self.style = tool_style;
                self.style.color = color;
            }
        } else {
            if !ui.ctx().input(|i| i.pointer.any_down()) {
                self.style_edit_open = false;
            }
            if selected.is_some() {
                self.style = tool_style;
            }
        }

        if let Some(i) = self.selected.filter(|i| *i < self.shapes.len()) {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if theme::icon_button(ui, ICON_FLIP_TO_FRONT, false, "Premier plan").clicked() {
                    self.push_undo();
                    let s = self.shapes.remove(i);
                    self.shapes.push(s);
                    self.selected = Some(self.shapes.len() - 1);
                    self.dirty = true;
                }
                if theme::icon_button(ui, ICON_FLIP_TO_BACK, false, "Arrière-plan").clicked() {
                    self.push_undo();
                    let s = self.shapes.remove(i);
                    self.shapes.insert(0, s);
                    self.selected = Some(0);
                    self.dirty = true;
                }
                if theme::icon_button(ui, ICON_DELETE, false, "Supprimer (Suppr)").clicked() {
                    self.push_undo();
                    self.shapes.remove(i);
                    self.selected = None;
                    self.dirty = true;
                }
            });
        }

        // Numérotation : réglages de l'outil, ou du numéro sélectionné.
        self.number_format = settings.number_format;
        let sel_number = self.selected.filter(|i| *i < self.shapes.len()).and_then(|i| match self.shapes[i].kind {
            Kind::Number(n, f) => Some((i, n, f)),
            _ => None,
        });
        if self.tool == Tool::Number || sel_number.is_some() {
            section(ui, "Numérotation");
            let current = sel_number.map_or(self.number_format, |s| s.2);
            let mut pick = None;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                for f in NumberFormat::ALL {
                    if theme::filter_chip(ui, f.sample(), current == f).clicked() {
                        pick = Some(f);
                    }
                }
            });
            if let Some(f) = pick.filter(|f| *f != current) {
                if let Some((i, n, _)) = sel_number {
                    self.push_undo();
                    self.shapes[i].kind = Kind::Number(n, f);
                    self.dirty = true;
                } else {
                    self.number_format = f;
                    settings.number_format = f;
                    settings.save();
                }
            }

            ui.add_space(4.0);
            let mut value = sel_number.map_or(self.next_number, |s| s.1);
            let (mut edited, mut undo) = (false, false);
            ui.horizontal(|ui| {
                ui.label(if sel_number.is_some() { "Valeur" } else { "Prochain" });
                let r = ui
                    .add(
                        egui::DragValue::new(&mut value)
                            .range(1..=9999)
                            .speed(0.1)
                            .custom_formatter(move |n, _| current.format(n as u32))
                            .custom_parser(move |s| current.parse(s).map(f64::from)),
                    )
                    .on_hover_text("Glisser ou double-cliquer pour saisir (ex. 5, e, IV)");
                edited = r.changed();
                undo = r.drag_started() || (r.changed() && !r.dragged());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::icon_button(ui, ICON_RESTART_ALT, false, "Réinitialiser à 1").clicked() && value != 1 {
                        value = 1;
                        edited = true;
                        undo = true;
                    }
                });
            });
            if edited {
                if let Some((i, _, f)) = sel_number {
                    if undo {
                        self.push_undo();
                    }
                    self.shapes[i].kind = Kind::Number(value, f);
                    self.dirty = true;
                } else {
                    self.next_number = value;
                }
            }

            let numbers: Vec<usize> = (0..self.shapes.len()).filter(|&i| matches!(self.shapes[i].kind, Kind::Number(..))).collect();
            if !numbers.is_empty() {
                ui.add_space(2.0);
                let mixed = numbers.iter().any(|&i| !matches!(self.shapes[i].kind, Kind::Number(_, f) if f == current));
                if mixed && theme::button(ui, ButtonKind::Text, Some(ICON_FORMAT_LIST_NUMBERED), "Appliquer ce style à tous").clicked() {
                    self.push_undo();
                    for &i in &numbers {
                        if let Kind::Number(n, _) = self.shapes[i].kind {
                            self.shapes[i].kind = Kind::Number(n, current);
                        }
                    }
                    self.dirty = true;
                }
                if theme::button(ui, ButtonKind::Text, Some(ICON_SORT), "Renuméroter dans l'ordre")
                    .on_hover_text("Renumérote les pastilles dans l'ordre de création, à partir de 1")
                    .clicked()
                {
                    self.push_undo();
                    for (k, &i) in numbers.iter().enumerate() {
                        if let Kind::Number(_, f) = self.shapes[i].kind {
                            self.shapes[i].kind = Kind::Number(k as u32 + 1, f);
                        }
                    }
                    self.next_number = numbers.len() as u32 + 1;
                    self.dirty = true;
                }
            }
        }

        if self.tool == Tool::Cut {
            section(ui, "Découpe");
            ui.label(egui::RichText::new("Retire une bande de l'image et recolle les deux morceaux.").size(12.0).color(p.on_surface_variant));
            ui.add_space(4.0);
            ui.label("Bande à retirer");
            ui.horizontal_wrapped(|ui| {
                for (d, label) in [(CutDir::Horizontal, "Horizontale"), (CutDir::Vertical, "Verticale")] {
                    if theme::filter_chip(ui, label, self.cut_dir == d).clicked() && self.cut_dir != d {
                        self.cut_dir = d;
                        self.cut = None;
                    }
                }
            });
            ui.add_space(4.0);
            ui.label("Raccord");
            ui.horizontal_wrapped(|ui| {
                for (e, label) in [(CutEdge::Straight, "Droit"), (CutEdge::Zigzag, "Zigzag")] {
                    if theme::filter_chip(ui, label, self.cut_edge == e).clicked() {
                        self.cut_edge = e;
                    }
                }
            });
        }

        if self.tool == Tool::MoveArea {
            section(ui, "Déplacer une zone");
            ui.label(
                egui::RichText::new("Sélectionnez une zone de l'image puis faites-la glisser. L'emplacement d'origine est rempli avec la couleur de fond détectée.")
                    .size(12.0)
                    .color(p.on_surface_variant),
            );
        }
        if let Some(i) = self.selected.filter(|i| *i < self.shapes.len()) {
            if let Kind::Patch { fill, .. } = &mut self.shapes[i].kind {
                let mut c = Color32::from_rgba_unmultiplied(fill[0], fill[1], fill[2], fill[3]);
                let mut changed = false;
                ui.horizontal(|ui| {
                    ui.label("Remplissage de l'origine");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        changed = ui.color_edit_button_srgba(&mut c).changed();
                    });
                });
                if changed {
                    *fill = c.to_srgba_unmultiplied();
                    self.dirty = true;
                    self.modified = true;
                }
            }
        }

        if self.tool == Tool::Stamp {
            section(ui, "Tampons");
            ui.horizontal_wrapped(|ui| {
                for (i, s) in STAMPS.iter().enumerate() {
                    if theme::filter_chip(ui, s, self.stamp == i).clicked() {
                        self.stamp = i;
                    }
                }
            });
        }

        ui.add_space(10.0);
        ui.separator();
        section(ui, "Effets à l'export");
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Bordure");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                changed |= theme::switch(ui, &mut settings.effect_border).changed();
            });
        });
        if settings.effect_border {
            changed |= ui.add(egui::Slider::new(&mut settings.effect_border_width, 1..=20).suffix(" px")).changed();
            ui.horizontal(|ui| {
                ui.label("Couleur de la bordure");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut c = Color32::from_rgba_unmultiplied(settings.effect_border_color[0], settings.effect_border_color[1], settings.effect_border_color[2], settings.effect_border_color[3]);
                    if ui.color_edit_button_srgba(&mut c).changed() {
                        settings.effect_border_color = c.to_srgba_unmultiplied();
                        changed = true;
                    }
                });
            });
        }
        ui.horizontal(|ui| {
            ui.label("Ombre portée");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                changed |= theme::switch(ui, &mut settings.effect_shadow).changed();
            });
        });
        if changed {
            settings.save();
        }
    }

    // ------------------------------------------------------------------ canevas

    fn canvas(&mut self, ui: &mut Ui) {
        let p = palette(ui.ctx());
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let area = resp.rect;
        ui.ctx().memory_mut(|m| m.data.insert_temp(egui::Id::new("ed_canvas_rect"), area));
        let ppp = ui.ctx().pixels_per_point();
        let img_size = vec2(self.base.width() as f32, self.base.height() as f32);

        if self.fit {
            let z = ((area.width() - 48.0) / img_size.x).min((area.height() - 48.0) / img_size.y);
            self.zoom = z.min(1.0 / ppp).max(0.02);
            self.pan = Vec2::ZERO;
        }

        // Zoom (Ctrl+molette / pincement) autour du curseur, défilement à la molette.
        if resp.hovered() {
            let (zoom_delta, scroll, hover) = ui.ctx().input(|i| (i.zoom_delta(), i.smooth_scroll_delta, i.pointer.hover_pos()));
            if (zoom_delta - 1.0).abs() > 1e-3 {
                if let Some(h) = hover {
                    let origin = area.center() - img_size * self.zoom / 2.0 + self.pan;
                    let img_pt = (h - origin) / self.zoom;
                    self.zoom = (self.zoom * zoom_delta).clamp(0.02, 16.0);
                    self.fit = false;
                    let new_origin = h - img_pt * self.zoom;
                    self.pan += new_origin - (area.center() - img_size * self.zoom / 2.0 + self.pan);
                }
            } else if scroll != Vec2::ZERO {
                self.pan += scroll;
                self.fit = false;
            }
        }

        let origin = area.center() - img_size * self.zoom / 2.0 + self.pan;
        let img_rect = Rect::from_min_size(origin, img_size * self.zoom);
        let zoom = self.zoom;
        let to_img = move |s: Pos2| ((s - origin) / zoom).to_pos2();
        let to_scr = move |i: Pos2| origin + i.to_vec2() * zoom;

        // Image avec ombre (élévation 2).
        painter.rect_filled(img_rect.translate(vec2(0.0, 3.0)).expand(1.0), 2.0, Color32::from_black_alpha(45));
        if let Some(t) = &self.texture {
            t.paint(&painter.with_clip_rect(area), img_rect, Color32::WHITE);
        }

        self.hover_px = resp.hover_pos().map(to_img).and_then(|q| {
            (q.x >= 0.0 && q.y >= 0.0 && q.x < img_size.x && q.y < img_size.y).then(|| (q.x as u32, q.y as u32))
        });

        // Curseur selon l'outil.
        if resp.hovered() {
            match self.tool {
                Tool::Select => ui.ctx().set_cursor_icon(egui::CursorIcon::Default),
                Tool::Text | Tool::Bubble => ui.ctx().set_cursor_icon(egui::CursorIcon::Text),
                Tool::Picker => ui.ctx().set_cursor_icon(egui::CursorIcon::Cell),
                _ => crate::theme::crosshair_cursor(ui.ctx()),
            }
        }

        let shift = ui.input(|i| i.modifiers.shift);

        // Panoramique : bouton du milieu.
        if resp.drag_started_by(PointerButton::Middle) {
            self.drag = Drag::Pan;
        }

        if resp.drag_started_by(PointerButton::Primary) {
            let start = ui.input(|i| i.pointer.press_origin()).unwrap_or_default();
            let ip = to_img(start);
            self.commit_text();
            self.on_drag_start(ip, start, &to_scr);
        }

        if resp.dragged() {
            let delta = resp.drag_delta();
            if let Some(cur) = resp.interact_pointer_pos() {
                let ip = to_img(cur);
                self.on_drag(ip, delta, shift);
            }
        }

        if resp.drag_stopped() {
            self.on_drag_end();
        }

        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                self.on_click(to_img(pos));
            }
        }
        if resp.double_clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let ip = to_img(pos);
                if let Some(i) = self.hit(ip) {
                    if self.shapes[i].is_textual() {
                        self.push_undo();
                        self.begin_text(i);
                    }
                }
            }
        }

        // Sélection et poignées.
        if let Some(i) = self.selected.filter(|i| *i < self.shapes.len() && Some(*i) != self.editing) {
            let s = &self.shapes[i];
            let r = Rect::from_min_max(to_scr(s.rect().min), to_scr(s.rect().max)).expand(4.0);
            painter.rect_stroke(r, 2.0, Stroke::new(1.0_f32, p.primary), StrokeKind::Outside);
            if s.has_handles() {
                for h in [s.a, s.b] {
                    let c = to_scr(h);
                    painter.circle(c, 6.0, Color32::WHITE, Stroke::new(2.0_f32, p.primary));
                }
            }
        }

        // Zone de rognage.
        if let Some(cr) = self.crop {
            let r = Rect::from_min_max(to_scr(cr.min), to_scr(cr.max));
            let dim = Color32::from_black_alpha(130);
            painter.rect_filled(Rect::from_min_max(img_rect.min, pos2(img_rect.max.x, r.min.y)), 0.0, dim);
            painter.rect_filled(Rect::from_min_max(pos2(img_rect.min.x, r.max.y), img_rect.max), 0.0, dim);
            painter.rect_filled(Rect::from_min_max(pos2(img_rect.min.x, r.min.y), pos2(r.min.x, r.max.y)), 0.0, dim);
            painter.rect_filled(Rect::from_min_max(pos2(r.max.x, r.min.y), pos2(img_rect.max.x, r.max.y)), 0.0, dim);
            painter.rect_stroke(r, 0.0, Stroke::new(2.0_f32, p.primary), StrokeKind::Outside);
            painter.text(r.left_top() + vec2(0.0, -6.0), Align2::LEFT_BOTTOM, format!("{:.0} × {:.0}", cr.width(), cr.height()), FontId::proportional(12.0), Color32::WHITE);
            if matches!(self.drag, Drag::None) && cr.width() > 2.0 {
                let mut apply = false;
                let mut cancel = false;
                egui::Area::new(egui::Id::new("crop_actions"))
                    .fixed_pos(pos2(r.right() - 220.0, r.bottom() + 10.0))
                    .order(egui::Order::Foreground)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::new().fill(p.surface_high).corner_radius(CornerRadius::same(24)).inner_margin(Margin::same(6)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                cancel = theme::button(ui, ButtonKind::Text, None, "Annuler").clicked();
                                apply = theme::button(ui, ButtonKind::Filled, Some(ICON_CROP), "Rogner").clicked();
                            });
                        });
                    });
                if apply {
                    self.crop = None;
                    self.apply_crop(cr);
                } else if cancel {
                    self.crop = None;
                }
            }
        }

        // Bande à découper : zone hachurée en rouge et barre de confirmation.
        if let Some((a, b)) = self.cut {
            let horizontal = self.cut_dir == CutDir::Horizontal;
            let r = if horizontal {
                Rect::from_min_max(pos2(img_rect.left(), to_scr(pos2(0.0, a)).y), pos2(img_rect.right(), to_scr(pos2(0.0, b)).y))
            } else {
                Rect::from_min_max(pos2(to_scr(pos2(a, 0.0)).x, img_rect.top()), pos2(to_scr(pos2(b, 0.0)).x, img_rect.bottom()))
            };
            let red = Color32::from_rgb(229, 57, 53);
            painter.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(229, 57, 53, 70));
            let edges = if horizontal {
                [[r.left_top(), r.right_top()], [r.left_bottom(), r.right_bottom()]]
            } else {
                [[r.left_top(), r.left_bottom()], [r.right_top(), r.right_bottom()]]
            };
            for e in edges {
                painter.extend(egui::Shape::dashed_line(&e, Stroke::new(2.0_f32, red), 8.0, 5.0));
            }
            let label = format!("{:.0} px retirés", b - a);
            let lp = if horizontal { pos2(area.center().x.max(img_rect.left() + 60.0), r.center().y) } else { pos2(r.center().x, img_rect.top() + 16.0) };
            painter.text(lp, Align2::CENTER_CENTER, label, FontId::proportional(13.0), red);
            if matches!(self.drag, Drag::None) && b - a >= 2.0 {
                let (mut apply, mut cancel) = (false, false);
                let at = if horizontal { pos2(img_rect.right().min(area.right()) - 230.0, r.bottom() + 10.0) } else { pos2(r.right() + 10.0, img_rect.top() + 30.0) };
                egui::Area::new(egui::Id::new("cut_actions"))
                    .fixed_pos(at)
                    .order(egui::Order::Foreground)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::new().fill(p.surface_high).corner_radius(CornerRadius::same(24)).inner_margin(Margin::same(6)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                cancel = theme::button(ui, ButtonKind::Text, None, "Annuler").clicked();
                                apply = theme::button(ui, ButtonKind::Filled, Some(ICON_CONTENT_CUT), "Découper").clicked();
                            });
                        });
                    });
                if apply {
                    self.cut = None;
                    self.apply_cut(a, b);
                } else if cancel {
                    self.cut = None;
                }
            }
        }

        // Zone en cours de sélection (outil Déplacer une zone).
        if let Some(sel) = self.area_sel {
            let r = Rect::from_min_max(to_scr(sel.min), to_scr(sel.max));
            painter.rect_filled(r, 0.0, p.primary.gamma_multiply(0.15));
            let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
            painter.extend(egui::Shape::dashed_line(&pts, Stroke::new(1.5_f32, p.primary), 6.0, 4.0));
        }

        // Aperçu d'outils sans glisser (loupe de la pipette).
        if self.tool == Tool::Picker {
            if let (Some((x, y)), Some(h)) = (self.hover_px, resp.hover_pos()) {
                let px = self.composite.get_pixel(x.min(self.composite.width() - 1), y.min(self.composite.height() - 1));
                let c = Color32::from_rgb(px[0], px[1], px[2]);
                painter.circle(h + vec2(24.0, 24.0), 14.0, c, Stroke::new(2.0_f32, Color32::WHITE));
            }
        }

        if matches!(self.drag, Drag::Pan) && resp.dragged_by(PointerButton::Middle) {
            self.pan += resp.drag_delta();
            self.fit = false;
        }
    }

    fn hit(&self, p: Pos2) -> Option<usize> {
        let tol = 4.0 / self.zoom.max(0.05);
        self.shapes.iter().enumerate().rev().find(|(_, s)| s.hit(p, tol)).map(|(i, _)| i)
    }

    fn on_drag_start(&mut self, ip: Pos2, scr: Pos2, to_scr: &dyn Fn(Pos2) -> Pos2) {
        match self.tool {
            Tool::Select => {
                if let Some(i) = self.selected.filter(|i| *i < self.shapes.len()) {
                    let s = &self.shapes[i];
                    if s.has_handles() {
                        for (first, h) in [(true, s.a), (false, s.b)] {
                            if (to_scr(h) - scr).length() < 10.0 {
                                self.push_undo();
                                self.drag = Drag::Handle { idx: i, first };
                                return;
                            }
                        }
                    }
                }
                if let Some(i) = self.hit(ip) {
                    self.selected = Some(i);
                    self.push_undo();
                    self.drag = Drag::Moving { idx: i, last: ip };
                } else {
                    self.selected = None;
                }
            }
            Tool::Crop => {
                self.drag = Drag::Crop { start: ip };
                self.crop = Some(Rect::from_two_pos(ip, ip));
            }
            Tool::Cut => {
                let v = if self.cut_dir == CutDir::Horizontal { ip.y } else { ip.x };
                self.drag = Drag::Cut { start: v };
                self.cut = Some((v, v));
            }
            Tool::MoveArea => {
                // Glisser sur une zone existante la déplace ; ailleurs, on en sélectionne une nouvelle.
                let hit = self.hit(ip).filter(|i| matches!(self.shapes[*i].kind, Kind::Patch { .. }));
                if let Some(i) = hit {
                    self.selected = Some(i);
                    self.push_undo();
                    self.drag = Drag::Moving { idx: i, last: ip };
                } else {
                    self.selected = None;
                    self.drag = Drag::Area { start: ip };
                    self.area_sel = Some(Rect::from_two_pos(ip, ip));
                }
            }
            Tool::Text | Tool::Bubble | Tool::Number | Tool::Stamp | Tool::Picker => {}
            t => {
                let kind = match t {
                    Tool::Arrow => Kind::Arrow,
                    Tool::Line => Kind::Line,
                    Tool::Rect => Kind::Rect,
                    Tool::Ellipse => Kind::Ellipse,
                    Tool::Highlight => Kind::Highlight,
                    Tool::Blur => Kind::Blur,
                    Tool::Magnifier => Kind::Magnifier,
                    _ => Kind::Pen(vec![ip]),
                };
                self.push_undo();
                self.shapes.push(Shape { kind, a: ip, b: ip, style: self.style.clone() });
                self.drag = Drag::Creating(self.shapes.len() - 1);
                self.selected = None;
            }
        }
    }

    fn on_drag(&mut self, ip: Pos2, _delta: Vec2, shift: bool) {
        match self.drag {
            Drag::Creating(i) => {
                let s = &mut self.shapes[i];
                match &mut s.kind {
                    Kind::Pen(pts) => {
                        if pts.last().map(|l| (*l - ip).length() > 1.0).unwrap_or(true) {
                            pts.push(ip);
                        }
                    }
                    Kind::Arrow | Kind::Line => s.b = if shift { snap45(s.a, ip) } else { ip },
                    Kind::Magnifier => s.b = snap_square(s.a, ip),
                    _ => s.b = if shift { snap_square(s.a, ip) } else { ip },
                }
                self.dirty = true;
            }
            Drag::Moving { idx, last } => {
                self.shapes[idx].translate(ip - last);
                self.drag = Drag::Moving { idx, last: ip };
                self.dirty = true;
            }
            Drag::Handle { idx, first } => {
                let s = &mut self.shapes[idx];
                if first {
                    s.a = ip;
                } else {
                    s.b = ip;
                }
                self.dirty = true;
            }
            Drag::Crop { start } => {
                let img = Rect::from_min_size(Pos2::ZERO, vec2(self.base.width() as f32, self.base.height() as f32));
                let r = Rect::from_two_pos(start, ip).intersect(img);
                self.crop = Some(Rect::from_min_max(r.min.round(), r.max.round()));
            }
            Drag::Cut { start } => {
                let (v, len) = if self.cut_dir == CutDir::Horizontal {
                    (ip.y, self.base.height() as f32)
                } else {
                    (ip.x, self.base.width() as f32)
                };
                let (a, b) = (start.min(v).clamp(0.0, len), start.max(v).clamp(0.0, len));
                self.cut = Some((a.round(), b.round()));
            }
            Drag::Area { start } => {
                let img = Rect::from_min_size(Pos2::ZERO, vec2(self.base.width() as f32, self.base.height() as f32));
                let r = Rect::from_two_pos(start, ip).intersect(img);
                self.area_sel = Some(Rect::from_min_max(r.min.round(), r.max.round()));
            }
            _ => {}
        }
    }

    fn on_drag_end(&mut self) {
        if let Drag::Creating(i) = self.drag {
            let s = &self.shapes[i];
            let tiny = match &s.kind {
                Kind::Pen(pts) => pts.len() < 2,
                Kind::Arrow | Kind::Line => (s.b - s.a).length() < 3.0,
                _ => s.rect().width() < 3.0 || s.rect().height() < 3.0,
            };
            if tiny {
                self.shapes.pop();
                self.undo.pop();
            } else {
                self.selected = Some(i);
            }
            self.dirty = true;
        }
        if let Drag::Area { .. } = self.drag {
            if let Some(r) = self.area_sel.take() {
                self.create_patch(r);
            }
        }
        if let (Drag::Cut { .. }, Some((a, b))) = (&self.drag, self.cut) {
            if b - a < 2.0 {
                self.cut = None;
            }
        }
        self.drag = Drag::None;
    }

    fn on_click(&mut self, ip: Pos2) {
        let inside = ip.x >= 0.0 && ip.y >= 0.0 && ip.x < self.base.width() as f32 && ip.y < self.base.height() as f32;
        match self.tool {
            Tool::Select => self.selected = self.hit(ip),
            Tool::Text | Tool::Bubble => {
                if let Some(i) = self.hit(ip).filter(|i| self.shapes[*i].is_textual()) {
                    self.push_undo();
                    self.begin_text(i);
                    return;
                }
                if !inside {
                    return;
                }
                self.push_undo();
                let kind = if self.tool == Tool::Text { Kind::Text(String::new()) } else { Kind::Bubble(String::new()) };
                self.shapes.push(Shape { kind, a: ip, b: ip, style: self.style.clone() });
                self.begin_text(self.shapes.len() - 1);
            }
            Tool::Number if inside => {
                self.push_undo();
                self.shapes.push(Shape { kind: Kind::Number(self.next_number, self.number_format), a: ip, b: ip, style: self.style.clone() });
                self.next_number += 1;
                self.dirty = true;
            }
            Tool::Stamp if inside => {
                self.push_undo();
                let mut s = Shape { kind: Kind::Stamp(STAMPS[self.stamp].to_string()), a: ip, b: ip, style: self.style.clone() };
                let (w, h) = render::text_box_size(&s);
                s.a = ip - vec2(w, h) / 2.0;
                s.b = s.a + vec2(w, h);
                self.shapes.push(s);
                self.selected = Some(self.shapes.len() - 1);
                self.dirty = true;
            }
            Tool::Picker if inside => {
                let px = self.composite.get_pixel(ip.x as u32, ip.y as u32);
                self.style.color = [px[0], px[1], px[2], 255];
                self.color_chosen = true;
            }
            _ => {}
        }
    }

    fn text_overlay(&mut self, ctx: &egui::Context) {
        let Some(idx) = self.editing else { return };
        if idx >= self.shapes.len() {
            self.editing = None;
            return;
        }
        let p = palette(ctx);
        let ppp = ctx.pixels_per_point();
        let _ = ppp;
        let shape = self.shapes[idx].clone();
        let Some(tex_origin) = self.screen_origin(ctx) else { return };
        let pos = tex_origin + shape.a.to_vec2() * self.zoom;
        let font_px = (shape.style.font_size * self.zoom).max(10.0);
        let c = shape.style.color;
        let mut done = false;
        egui::Area::new(egui::Id::new("text_edit_area"))
            .fixed_pos(pos - vec2(4.0, 4.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(if matches!(shape.kind, Kind::Bubble(_)) { Color32::from_rgba_unmultiplied(c[0], c[1], c[2], 230) } else { Color32::from_white_alpha(230) })
                    .stroke(Stroke::new(1.5_f32, p.primary))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(4))
                    .show(ui, |ui| {
                        let text_color = if matches!(shape.kind, Kind::Bubble(_)) { Color32::WHITE } else { Color32::from_rgb(c[0], c[1], c[2]) };
                        let te = egui::TextEdit::multiline(&mut self.text_buf)
                            .font(FontId::proportional(font_px))
                            .text_color(text_color)
                            .frame(false)
                            .desired_rows(1)
                            .desired_width((font_px * 8.0).max(120.0))
                            .hint_text("Saisissez votre texte…");
                        let r = ui.add(te);
                        if self.focus_text {
                            r.request_focus();
                            self.focus_text = false;
                        }
                        if ui.input(|i| i.key_pressed(Key::Escape) || (i.modifiers.command && i.key_pressed(Key::Enter))) {
                            done = true;
                        }
                        if r.lost_focus() && !ui.input(|i| i.key_pressed(Key::Tab)) {
                            done = true;
                        }
                    });
            });
        if let Kind::Text(t) | Kind::Bubble(t) = &mut self.shapes[idx].kind {
            *t = self.text_buf.clone();
        }
        if done {
            self.commit_text();
        }
    }

    fn screen_origin(&self, ctx: &egui::Context) -> Option<Pos2> {
        // Recalcule l'origine de l'image à partir de la zone centrale mémorisée.
        let area = ctx.memory(|m| m.data.get_temp::<Rect>(egui::Id::new("ed_canvas_rect")))?;
        let img_size = vec2(self.base.width() as f32, self.base.height() as f32);
        Some(area.center() - img_size * self.zoom / 2.0 + self.pan)
    }

    fn resize_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut pct) = self.resize_pct else { return };
        let mut open = true;
        let mut apply = false;
        egui::Window::new("Redimensionner l'image")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.add(egui::Slider::new(&mut pct, 10..=400).suffix(" %"));
                let (w, h) = (self.base.width() * pct / 100, self.base.height() * pct / 100);
                ui.label(format!("Nouvelle taille : {w} × {h} px"));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    for v in [25, 50, 75, 150, 200] {
                        if theme::filter_chip(ui, &format!("{v} %"), pct == v).clicked() {
                            pct = v;
                        }
                    }
                });
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    apply = theme::button(ui, ButtonKind::Filled, None, "Appliquer").clicked();
                });
            });
        self.resize_pct = Some(pct);
        if apply {
            self.resize_pct = None;
            self.resize(pct);
        } else if !open {
            self.resize_pct = None;
        }
    }
}
