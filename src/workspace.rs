//! Espace de travail : actions de capture et historique des fichiers (vignettes).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant, SystemTime};

use eframe::egui::{self, pos2, vec2, Align2, Color32, ColorImage, FontId, Rect, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui};
use egui_material_icons::icons::*;

use crate::app::CaptureKind;
use crate::settings::Settings;
use crate::theme::{self, palette, ButtonKind};

pub const IMAGE_EXT: [&str; 5] = ["png", "jpg", "jpeg", "bmp", "gif"];
const VIDEO_EXT: [&str; 3] = ["mp4", "webm", "mkv"];

#[derive(Clone)]
pub struct Item {
    pub path: PathBuf,
    pub name: String,
    pub modified: SystemTime,
    pub size: u64,
    pub is_video: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Images,
    Videos,
}

pub enum WsAction {
    Capture(CaptureKind),
    Edit(PathBuf),
    Copy(PathBuf),
    /// Envoyer ces captures dans la Corbeille (confirmation déjà faite).
    Delete(Vec<PathBuf>),
    /// Épingler (ou désépingler si toutes le sont) : protégées de la suppression automatique.
    TogglePin(Vec<PathBuf>),
    Open(PathBuf),
    Reveal(PathBuf),
    OpenFolder,
    Paste,
    OpenFile,
}

pub struct Workspace {
    items: Vec<Item>,
    thumbs: HashMap<PathBuf, TextureHandle>,
    pending: HashSet<PathBuf>,
    req_tx: Sender<PathBuf>,
    res_rx: Receiver<(PathBuf, Option<ColorImage>)>,
    last_scan: Option<Instant>,
    filter: Filter,
    search: String,
    /// Captures dont la suppression attend confirmation.
    confirm_delete: Option<Vec<PathBuf>>,
    /// Captures sélectionnées (clic, Ctrl+clic, Maj+clic, Ctrl+A) ; Suppr les supprime.
    selected: Vec<PathBuf>,
    /// Point de départ d'une sélection par plage (Maj+clic).
    anchor: Option<PathBuf>,
    /// Mode sélection (activé par la case d'une vignette) : un simple clic ajoute ou retire.
    check_mode: bool,
    /// Captures visibles, dans l'ordre d'affichage (pour Maj+clic et Ctrl+A).
    visible_order: Vec<PathBuf>,
    /// Position des vignettes affichées (points, relatifs à la fenêtre), pour les tests.
    pub card_rects: Vec<(PathBuf, Rect)>,
}

impl Workspace {
    pub fn new(ctx: &egui::Context) -> Self {
        let (req_tx, req_rx) = channel::<PathBuf>();
        let (res_tx, res_rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            while let Ok(path) = req_rx.recv() {
                let thumb = image::open(&path).ok().map(|img| {
                    let t = fit_thumbnail(&img.to_rgba8(), 440, 248);
                    ColorImage::from_rgba_unmultiplied([t.width() as usize, t.height() as usize], t.as_raw())
                });
                if res_tx.send((path, thumb)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self {
            items: Vec::new(),
            thumbs: HashMap::new(),
            pending: HashSet::new(),
            req_tx,
            res_rx,
            last_scan: None,
            filter: Filter::All,
            search: String::new(),
            confirm_delete: None,
            selected: Vec::new(),
            anchor: None,
            check_mode: false,
            visible_order: Vec::new(),
            card_rects: Vec::new(),
        }
    }

    pub fn refresh(&mut self) {
        self.last_scan = None;
    }

    pub fn forget(&mut self, path: &Path) {
        self.selected.retain(|p| p != path);
        self.thumbs.remove(path);
        self.pending.remove(path);
        self.refresh();
    }

    fn scan(&mut self, dir: &Path) {
        let mut items = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let path = e.path();
                let ext = path.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
                let is_video = VIDEO_EXT.contains(&ext.as_str());
                if !is_video && !IMAGE_EXT.contains(&ext.as_str()) {
                    continue;
                }
                if path.to_string_lossy().ends_with(".tmp.mp4") {
                    continue;
                }
                let Ok(meta) = e.metadata() else { continue };
                items.push(Item {
                    name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    size: meta.len(),
                    is_video,
                    path,
                });
            }
        }
        items.sort_by(|a, b| b.modified.cmp(&a.modified));
        // Invalide les vignettes des fichiers modifiés.
        for it in &items {
            if let Some(old) = self.items.iter().find(|o| o.path == it.path) {
                if old.modified != it.modified {
                    self.thumbs.remove(&it.path);
                }
            }
        }
        self.items = items;
        self.last_scan = Some(Instant::now());
    }

    pub fn ui(&mut self, ui: &mut Ui, settings: &Settings, hotkeys_ok: &[(CaptureKind, bool)]) -> Vec<WsAction> {
        if self.last_scan.map(|t| t.elapsed() > Duration::from_secs(4)).unwrap_or(true) {
            self.scan(&settings.save_dir);
        }
        while let Ok((path, img)) = self.res_rx.try_recv() {
            self.pending.remove(&path);
            if let Some(img) = img {
                let tex = ui.ctx().load_texture(path.to_string_lossy(), img, TextureOptions::LINEAR);
                self.thumbs.insert(path, tex);
            }
        }

        let p = palette(ui.ctx());
        let mut actions = Vec::new();

        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            ui.add_space(8.0);
            ui.label(egui::RichText::new(crate::i18n::tr("Nouvelle capture")).font(theme::medium(22.0)).color(p.on_surface));
            ui.add_space(12.0);
            let shortcut = |k: CaptureKind| -> String {
                let hk = &settings.hotkeys;
                let s = match k {
                    CaptureKind::Region => &hk.region,
                    CaptureKind::Fullscreen => &hk.fullscreen,
                    CaptureKind::Window => &hk.window,
                    CaptureKind::LastRegion => &hk.last_region,
                    CaptureKind::Video => &hk.video,
                    CaptureKind::Delayed => return crate::i18n::trf("Après {0} s", &[&settings.delay_seconds]),
                    CaptureKind::Scrolling => return crate::i18n::tr("Page entière").into(),
                };
                let ok = hotkeys_ok.iter().find(|(c, _)| *c == k).map(|(_, ok)| *ok).unwrap_or(false);
                if s.is_empty() {
                    crate::i18n::tr("Aucun raccourci").into()
                } else if ok {
                    pretty_hotkey(s)
                } else {
                    format!("{} ⚠", pretty_hotkey(s))
                }
            };
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                let cards = [
                    (CaptureKind::Region, ICON_SCREENSHOT_REGION, crate::i18n::tr("Zone"), true),
                    (CaptureKind::Fullscreen, ICON_SCREENSHOT_MONITOR, crate::i18n::tr("Plein écran"), false),
                    (CaptureKind::Window, ICON_WEB_ASSET, crate::i18n::tr("Fenêtre"), false),
                    (CaptureKind::LastRegion, ICON_CROP_FREE, crate::i18n::tr("Dernière zone"), false),
                    (CaptureKind::Delayed, ICON_TIMER, crate::i18n::tr("Différée"), false),
                    (CaptureKind::Video, ICON_VIDEOCAM, crate::i18n::tr("Vidéo"), false),
                    (CaptureKind::Scrolling, ICON_SWIPE_VERTICAL, crate::i18n::tr("Défilement"), false),
                ];
                for (kind, icon, title, emph) in cards {
                    if theme::action_card(ui, icon, title, &shortcut(kind), emph).clicked() {
                        actions.push(WsAction::Capture(kind));
                    }
                }
            });

            ui.add_space(28.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(crate::i18n::tr("Historique")).font(theme::medium(22.0)).color(p.on_surface));
                ui.label(egui::RichText::new(if self.items.len() > 1 { crate::i18n::trf("{0} éléments", &[&self.items.len()]) } else { crate::i18n::trf("{0} élément", &[&self.items.len()]) }).color(p.on_surface_variant));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::icon_button(ui, ICON_FOLDER_OPEN, false, crate::i18n::tr("Ouvrir le dossier des captures")).clicked() {
                        actions.push(WsAction::OpenFolder);
                    }
                    if theme::icon_button(ui, ICON_IMAGE, false, crate::i18n::tr("Ouvrir une image dans l'éditeur")).clicked() {
                        actions.push(WsAction::OpenFile);
                    }
                    if theme::icon_button(ui, ICON_CONTENT_PASTE, false, &crate::platform::sc(crate::i18n::tr("Coller l'image du presse-papiers dans le dossier des captures (Ctrl+V)"))).clicked() {
                        actions.push(WsAction::Paste);
                    }
                    if theme::icon_button(ui, ICON_REFRESH, false, crate::i18n::tr("Actualiser")).clicked() {
                        self.refresh();
                    }
                });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                for (f, label) in [(Filter::All, crate::i18n::tr("Tout")), (Filter::Images, crate::i18n::tr("Images")), (Filter::Videos, crate::i18n::tr("Vidéos"))] {
                    if theme::filter_chip(ui, label, self.filter == f).clicked() {
                        self.filter = f;
                    }
                }
                ui.add_space(12.0);
                ui.label(egui::RichText::new(ICON_SEARCH).size(20.0).color(p.on_surface_variant));
                theme::text_field(ui, &mut self.search, 200.0, crate::i18n::tr("Rechercher…"));
            });
            ui.add_space(14.0);
            // Hauteur toujours réservée : les vignettes ne bougent pas quand la sélection commence
            // (sinon le clic suivant tomberait à côté).
            let width = ui.available_width();
            ui.allocate_ui_with_layout(vec2(width, 44.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_min_size(vec2(width, 44.0));
                if self.selected.is_empty() {
                    ui.label(egui::RichText::new(ICON_CHECK_CIRCLE).size(18.0).color(p.outline));
                    ui.label(egui::RichText::new(crate::i18n::tr("Cliquer sur la case ronde d'une vignette pour sélectionner plusieurs captures.")).size(12.5).color(p.on_surface_variant));
                } else {
                    self.selection_bar(ui, settings, &mut actions);
                }
            });
            ui.add_space(10.0);

            let q = self.search.to_lowercase();
            let visible: Vec<usize> = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, it)| match self.filter {
                    Filter::All => true,
                    Filter::Images => !it.is_video,
                    Filter::Videos => it.is_video,
                })
                .filter(|(_, it)| q.is_empty() || it.name.to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect();

            if visible.is_empty() {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(ICON_PHOTO_LIBRARY).size(64.0).color(p.outline));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(crate::i18n::tr("Aucune capture pour l'instant")).font(theme::medium(16.0)).color(p.on_surface));
                    ui.label(egui::RichText::new(crate::i18n::trf("Appuyez sur {0} pour capturer une zone de l'écran.", &[&pretty_hotkey(&settings.hotkeys.region)])).color(p.on_surface_variant));
                });
                return;
            }

            // Captures masquées par un filtre ou une recherche : retirées de la sélection.
            self.visible_order = visible.iter().map(|&i| self.items[i].path.clone()).collect();
            let order = &self.visible_order;
            self.selected.retain(|s| order.contains(s));
            self.card_rects.clear();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                for i in visible {
                    let pinned = crate::retention::is_pinned(settings, &self.items[i].path);
                    self.card(ui, i, pinned, &mut actions);
                }
            });
            ui.add_space(24.0);
        });

        // Suppr : supprimer les captures sélectionnées (après confirmation) ; Ctrl+A : tout
        // sélectionner ; Échap : désélectionner.
        let ctx = ui.ctx().clone();
        if self.confirm_delete.is_none() && !ctx.wants_keyboard_input() {
            let (del, esc, all) = ctx.input(|i| (i.key_pressed(egui::Key::Delete), i.key_pressed(egui::Key::Escape), i.modifiers.command && i.key_pressed(egui::Key::A)));
            if del && !self.selected.is_empty() {
                self.confirm_delete = Some(self.selected.clone());
            } else if esc {
                self.selected.clear();
                self.check_mode = false;
            } else if all {
                self.selected = self.visible_order.clone();
            }
        }

        if let Some(paths) = self.confirm_delete.clone() {
            let (title, body) = match paths.as_slice() {
                [one] => (crate::i18n::tr("Supprimer la capture ?").to_string(), crate::i18n::trf("« {0} » sera envoyé dans la Corbeille.", &[&one.file_name().unwrap_or_default().to_string_lossy()])),
                many => (crate::i18n::trf("Supprimer {0} captures ?", &[&many.len()]), crate::i18n::trf("Les {0} captures sélectionnées seront envoyées dans la Corbeille.", &[&many.len()])),
            };
            if let Some(ok) = theme::confirm_dialog(&ctx, "confirm_delete", ICON_DELETE, &title, &body, crate::i18n::tr("Supprimer")) {
                if ok {
                    actions.push(WsAction::Delete(paths));
                }
                self.confirm_delete = None;
            }
        }
        actions
    }

    /// Bandeau affiché dès qu'une capture est sélectionnée : nombre, tout sélectionner,
    /// épingler, supprimer, désélectionner.
    fn selection_bar(&mut self, ui: &mut Ui, settings: &Settings, actions: &mut Vec<WsAction>) {
        let p = palette(ui.ctx());
        let n = self.selected.len();
        egui::Frame::new()
            .fill(p.secondary_container)
            .corner_radius(egui::CornerRadius::same(24))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if theme::icon_button(ui, ICON_CLOSE, false, crate::i18n::tr("Désélectionner (Échap)")).clicked() {
                        self.selected.clear();
                        self.check_mode = false;
                    }
                    let label = if n > 1 { crate::i18n::trf("{0} captures sélectionnées", &[&n]) } else { crate::i18n::tr("1 capture sélectionnée").to_string() };
                    ui.label(egui::RichText::new(label).font(theme::medium(14.0)).color(p.on_secondary_container));
                    ui.add_space(12.0);
                    if n < self.visible_order.len() && theme::button(ui, ButtonKind::Text, Some(ICON_SELECT_ALL), &crate::platform::sc(crate::i18n::tr("Tout sélectionner (Ctrl+A)"))).clicked() {
                        self.selected = self.visible_order.clone();
                        self.check_mode = true;
                    }
                    let all_pinned = self.selected.iter().all(|s| crate::retention::is_pinned(settings, s));
                    let (icon, pin) = if all_pinned { (ICON_KEEP_OFF, crate::i18n::tr("Désépingler")) } else { (ICON_KEEP, crate::i18n::tr("Épingler")) };
                    if theme::button(ui, ButtonKind::Text, Some(icon), pin).clicked() {
                        actions.push(WsAction::TogglePin(self.selected.clone()));
                    }
                    if theme::button(ui, ButtonKind::Danger, Some(ICON_DELETE), crate::i18n::tr("Supprimer (Suppr)")).clicked() {
                        self.confirm_delete = Some(self.selected.clone());
                    }
                });
            });
    }

    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }

    /// Clic sur une vignette : sélection simple, Ctrl+clic (ajouter / retirer) ou Maj+clic (plage).
    fn click_select(&mut self, path: &Path, ctrl: bool, shift: bool) {
        if shift {
            let anchor = self.anchor.clone().unwrap_or_else(|| path.to_path_buf());
            let a = self.visible_order.iter().position(|p| *p == anchor);
            let b = self.visible_order.iter().position(|p| p == path);
            if let (Some(a), Some(b)) = (a, b) {
                let range = self.visible_order[a.min(b)..=a.max(b)].to_vec();
                if ctrl {
                    for p in range {
                        if !self.selected.contains(&p) {
                            self.selected.push(p);
                        }
                    }
                } else {
                    self.selected = range;
                }
            }
            return;
        }
        if ctrl {
            if let Some(i) = self.selected.iter().position(|p| p == path) {
                self.selected.remove(i);
            } else {
                self.selected.push(path.to_path_buf());
            }
        } else {
            self.selected = vec![path.to_path_buf()];
        }
        self.anchor = Some(path.to_path_buf());
    }

    /// Captures concernées par une action sur `path` : toute la sélection si `path` en fait partie.
    fn targets(&self, path: &Path) -> Vec<PathBuf> {
        if self.selected.len() > 1 && self.selected.iter().any(|p| p == path) {
            self.selected.clone()
        } else {
            vec![path.to_path_buf()]
        }
    }

    fn card(&mut self, ui: &mut Ui, i: usize, pinned: bool, actions: &mut Vec<WsAction>) {
        let p = palette(ui.ctx());
        // Copie : la sélection (clic) modifie `self` pendant l'affichage de la vignette.
        let it = self.items[i].clone();
        let path = it.path.clone();
        let size = vec2(220.0, 178.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        if !ui.is_rect_visible(rect) {
            return;
        }
        if !it.is_video && !self.thumbs.contains_key(&path) && !self.pending.contains(&path) {
            self.pending.insert(path.clone());
            let _ = self.req_tx.send(path.clone());
        }
        self.card_rects.push((path.clone(), rect));
        // Case de sélection (coin haut gauche) : visible au survol, et sur toutes les vignettes
        // en mode sélection ; un clic dessus ajoute ou retire la capture sans touche Ctrl.
        let check = Rect::from_center_size(rect.min + vec2(24.0, 24.0), vec2(30.0, 30.0));
        let check_resp = ui.interact(check, ui.id().with(("card_check", i)), Sense::click());
        let check_clicked = check_resp.clicked();
        if check_clicked {
            self.check_mode = true;
            self.click_select(&path, true, false);
        } else if resp.clicked() {
            let (ctrl, shift) = ui.input(|i| (i.modifiers.command, i.modifiers.shift));
            self.click_select(&path, ctrl || (self.check_mode && !shift), shift);
            if self.selected.is_empty() {
                self.check_mode = false;
            }
        } else if resp.secondary_clicked() && !self.selected.contains(&path) {
            // Clic droit hors de la sélection : sélectionner cette capture seule.
            self.click_select(&path, false, false);
        }
        let selected = self.selected.contains(&path);
        let painter = ui.painter();
        let hovered = resp.hovered();
        if hovered {
            painter.rect_filled(rect.translate(vec2(0.0, 2.0)), 12.0, Color32::from_black_alpha(35));
        }
        let border = if selected { Stroke::new(2.5_f32, p.primary) } else { Stroke::new(1.0_f32, p.outline_variant) };
        let fill = if selected { p.secondary_container } else if hovered { p.surface_high } else { p.surface_low };
        painter.rect(rect, 12.0, fill, border, StrokeKind::Inside);
        let thumb_area = Rect::from_min_size(rect.min + vec2(8.0, 8.0), vec2(size.x - 16.0, 124.0));
        painter.rect_filled(thumb_area, 8.0, p.surface_highest);
        if let Some(tex) = self.thumbs.get(&path) {
            let ts = tex.size_vec2();
            let k = (thumb_area.width() / ts.x).min(thumb_area.height() / ts.y);
            let r = Rect::from_center_size(thumb_area.center(), ts * k);
            painter.image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        } else if it.is_video {
            painter.circle_filled(thumb_area.center(), 26.0, p.primary_container);
            painter.text(thumb_area.center(), Align2::CENTER_CENTER, if it.name.ends_with(".gif") { ICON_GIF } else { ICON_PLAY_ARROW }, FontId::proportional(30.0), p.on_primary_container);
        } else {
            painter.text(thumb_area.center(), Align2::CENTER_CENTER, ICON_IMAGE, FontId::proportional(30.0), p.outline);
        }
        if pinned {
            // Pastille d'épingle, toujours visible.
            let c = pos2(thumb_area.right() - 16.0, thumb_area.top() + 16.0);
            painter.circle_filled(c, 13.0, p.primary);
            painter.text(c, Align2::CENTER_CENTER, ICON_KEEP, FontId::proportional(15.0), p.on_primary);
        }
        if it.is_video {
            let badge = Rect::from_min_size(thumb_area.min + vec2(6.0, 6.0), vec2(54.0, 20.0));
            painter.rect_filled(badge, 10.0, Color32::from_black_alpha(150));
            let ext = it.path.extension().unwrap_or_default().to_string_lossy().to_uppercase();
            painter.text(badge.center(), Align2::CENTER_CENTER, ext, theme::medium(11.0), Color32::WHITE);
        }
        let name = ellipsize(&it.name, 28);
        painter.text(pos2(rect.left() + 12.0, rect.bottom() - 34.0), Align2::LEFT_CENTER, name, theme::medium(13.0), p.on_surface);
        let dt: chrono::DateTime<chrono::Local> = it.modified.into();
        painter.text(
            pos2(rect.left() + 12.0, rect.bottom() - 15.0),
            Align2::LEFT_CENTER,
            format!("{}  •  {}", dt.format("%d/%m/%Y %H:%M"), human_size(it.size)),
            FontId::proportional(11.5),
            p.on_surface_variant,
        );
        let is_video = it.is_video;
        if self.check_mode || hovered || selected || check_resp.hovered() {
            let painter = ui.painter();
            let c = check.center();
            painter.circle_filled(c, 13.0, if selected { p.primary } else { Color32::from_black_alpha(110) });
            painter.circle_stroke(c, 13.0, Stroke::new(2.0_f32, Color32::WHITE));
            if selected {
                painter.text(c, Align2::CENTER_CENTER, ICON_CHECK, FontId::proportional(17.0), p.on_primary);
            }
        }
        let check_resp = check_resp.on_hover_text(if selected { crate::i18n::tr("Retirer de la sélection") } else { crate::i18n::tr("Ajouter à la sélection") });
        let _ = check_resp;

        // Barre d'actions au survol.
        let mut consumed = false;
        let show_bar = hovered || resp.contains_pointer();
        {
            let bar = Rect::from_min_size(pos2(thumb_area.right() - 4.0 - 5.0 * 34.0, thumb_area.bottom() - 40.0), vec2(5.0 * 34.0, 34.0));
            if show_bar {
                painter.rect_filled(bar, 17.0, p.surface_high);
            }
            let icons: [(&str, &str, u8); 5] = [
                (if is_video { ICON_PLAY_ARROW } else { ICON_EDIT }, if is_video { crate::i18n::tr("Lire") } else { crate::i18n::tr("Éditer") }, 0),
                (ICON_CONTENT_COPY, crate::i18n::tr("Copier"), 1),
                (if pinned { ICON_KEEP_OFF } else { ICON_KEEP }, if pinned { crate::i18n::tr("Désépingler") } else { crate::i18n::tr("Épingler : jamais supprimée automatiquement") }, 4),
                (ICON_FOLDER_OPEN, crate::platform::reveal_label(), 2),
                (ICON_DELETE, crate::i18n::tr("Supprimer"), 3),
            ];
            for (k, (icon, tip, id)) in icons.iter().enumerate() {
                let r = Rect::from_min_size(bar.min + vec2(k as f32 * 34.0, 0.0), vec2(34.0, 34.0));
                let br = ui.interact(r, ui.id().with(("card_btn", i, k)), Sense::click()).on_hover_text(*tip);
                if !show_bar && !br.hovered() {
                    continue;
                }
                if br.hovered() {
                    ui.painter().circle_filled(r.center(), 15.0, theme::blend(p.surface_high, p.on_surface, 0.12));
                }
                ui.painter().text(r.center(), Align2::CENTER_CENTER, *icon, FontId::proportional(18.0), if *id == 3 { p.error } else { p.on_surface_variant });
                if br.clicked() {
                    consumed = true;
                    match id {
                        0 => actions.push(if is_video { WsAction::Open(path.clone()) } else { WsAction::Edit(path.clone()) }),
                        1 => actions.push(WsAction::Copy(path.clone())),
                        2 => actions.push(WsAction::Reveal(path.clone())),
                        4 => actions.push(WsAction::TogglePin(self.targets(&path))),
                        _ => self.confirm_delete = Some(self.targets(&path)),
                    }
                }
            }
        }

        if !consumed && !check_clicked && resp.double_clicked() {
            actions.push(if is_video { WsAction::Open(path.clone()) } else { WsAction::Edit(path.clone()) });
        }
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        resp.context_menu(|ui| {
            if !is_video && ui.button(crate::i18n::trf("{0}  Éditer", &[&ICON_EDIT])).clicked() {
                actions.push(WsAction::Edit(path.clone()));
                ui.close_menu();
            }
            if ui.button(crate::i18n::trf("{0}  Ouvrir avec l'application par défaut", &[&ICON_OPEN_IN_NEW])).clicked() {
                actions.push(WsAction::Open(path.clone()));
                ui.close_menu();
            }
            if ui.button(crate::i18n::trf("{0}  Copier", &[&ICON_CONTENT_COPY])).clicked() {
                actions.push(WsAction::Copy(path.clone()));
                ui.close_menu();
            }
            if ui.button(format!("{ICON_FOLDER_OPEN}  {}", crate::platform::reveal_label())).clicked() {
                actions.push(WsAction::Reveal(path.clone()));
                ui.close_menu();
            }
            ui.separator();
            let targets = self.targets(&path);
            let pin_label = match (pinned, targets.len()) {
                (true, 1) => crate::i18n::trf("{0}  Désépingler", &[&ICON_KEEP_OFF]),
                (false, 1) => crate::i18n::trf("{0}  Épingler", &[&ICON_KEEP]),
                (_, n) => crate::i18n::trf("{0}  Épingler / désépingler les {1} captures", &[&ICON_KEEP, &n]),
            };
            if ui.button(pin_label).clicked() {
                actions.push(WsAction::TogglePin(targets.clone()));
                ui.close_menu();
            }
            let label = if targets.len() > 1 { crate::i18n::trf("{0}  Supprimer les {1} captures", &[&ICON_DELETE, &targets.len()]) } else { crate::i18n::trf("{0}  Supprimer", &[&ICON_DELETE]) };
            if ui.button(label).clicked() {
                self.confirm_delete = Some(targets);
                ui.close_menu();
            }
        });
    }
}

fn ellipsize(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    }
}

fn human_size(b: u64) -> String {
    if b >= 1 << 20 {
        crate::i18n::trf("{0} Mo", &[&(format!("{:.1}", b as f64 / (1u64 << 20) as f64))])
    } else {
        crate::i18n::trf("{0} Ko", &[&((b + 1023) / 1024)])
    }
}

/// « ctrl+shift+PrintScreen » → « Ctrl + Maj + Impr. écran ».
pub fn pretty_hotkey(s: &str) -> String {
    s.split('+')
        .map(|t| match t.trim().to_lowercase().as_str() {
            "ctrl" | "control" => if cfg!(target_os = "macos") { "⌃" } else { crate::i18n::tr("Ctrl") }.to_string(),
            "shift" => if cfg!(target_os = "macos") { "⇧" } else { crate::i18n::tr("Maj") }.to_string(),
            "alt" | "option" => if cfg!(target_os = "macos") { "⌥" } else { crate::i18n::tr("Alt") }.to_string(),
            "super" | "win" | "meta" | "cmd" | "command" => if cfg!(target_os = "macos") { "⌘" } else { crate::i18n::tr("Win") }.to_string(),
            "printscreen" => crate::i18n::tr("Impr. écran").to_string(),
            other => {
                let o = other.trim_start_matches("key").trim_start_matches("digit");
                o.to_uppercase()
            }
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Vignette qui tient dans `max_w × max_h` en gardant les proportions ; une petite image n'est
/// pas agrandie (l'affichage s'en charge), ce qui évite toute déformation.
pub fn fit_thumbnail(img: &image::RgbaImage, max_w: u32, max_h: u32) -> image::RgbaImage {
    let (w, h) = img.dimensions();
    if w <= max_w && h <= max_h {
        return img.clone();
    }
    let k = (max_w as f64 / w as f64).min(max_h as f64 / h as f64);
    let (tw, th) = (((w as f64 * k).round() as u32).max(1), ((h as f64 * k).round() as u32).max(1));
    image::imageops::thumbnail(img, tw, th)
}
