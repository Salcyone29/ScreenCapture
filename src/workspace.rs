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

const IMAGE_EXT: [&str; 5] = ["png", "jpg", "jpeg", "bmp", "gif"];
const VIDEO_EXT: [&str; 3] = ["mp4", "webm", "mkv"];

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
    Delete(PathBuf),
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
    confirm_delete: Option<PathBuf>,
}

impl Workspace {
    pub fn new(ctx: &egui::Context) -> Self {
        let (req_tx, req_rx) = channel::<PathBuf>();
        let (res_tx, res_rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            while let Ok(path) = req_rx.recv() {
                let thumb = image::open(&path).ok().map(|img| {
                    let t = img.thumbnail(440, 248).to_rgba8();
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
        }
    }

    pub fn refresh(&mut self) {
        self.last_scan = None;
    }

    pub fn forget(&mut self, path: &Path) {
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
            ui.label(egui::RichText::new("Nouvelle capture").font(theme::medium(22.0)).color(p.on_surface));
            ui.add_space(12.0);
            let shortcut = |k: CaptureKind| -> String {
                let hk = &settings.hotkeys;
                let s = match k {
                    CaptureKind::Region => &hk.region,
                    CaptureKind::Fullscreen => &hk.fullscreen,
                    CaptureKind::Window => &hk.window,
                    CaptureKind::LastRegion => &hk.last_region,
                    CaptureKind::Video => &hk.video,
                    CaptureKind::Delayed => return format!("Après {} s", settings.delay_seconds),
                };
                let ok = hotkeys_ok.iter().find(|(c, _)| *c == k).map(|(_, ok)| *ok).unwrap_or(false);
                if s.is_empty() {
                    "Aucun raccourci".into()
                } else if ok {
                    pretty_hotkey(s)
                } else {
                    format!("{} ⚠", pretty_hotkey(s))
                }
            };
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                let cards = [
                    (CaptureKind::Region, ICON_SCREENSHOT_REGION, "Zone", true),
                    (CaptureKind::Fullscreen, ICON_SCREENSHOT_MONITOR, "Plein écran", false),
                    (CaptureKind::Window, ICON_WEB_ASSET, "Fenêtre", false),
                    (CaptureKind::LastRegion, ICON_CROP_FREE, "Dernière zone", false),
                    (CaptureKind::Delayed, ICON_TIMER, "Différée", false),
                    (CaptureKind::Video, ICON_VIDEOCAM, "Vidéo", false),
                ];
                for (kind, icon, title, emph) in cards {
                    if theme::action_card(ui, icon, title, &shortcut(kind), emph).clicked() {
                        actions.push(WsAction::Capture(kind));
                    }
                }
            });

            ui.add_space(28.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Historique").font(theme::medium(22.0)).color(p.on_surface));
                ui.label(egui::RichText::new(format!("{} élément{}", self.items.len(), if self.items.len() > 1 { "s" } else { "" })).color(p.on_surface_variant));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::icon_button(ui, ICON_FOLDER_OPEN, false, "Ouvrir le dossier des captures").clicked() {
                        actions.push(WsAction::OpenFolder);
                    }
                    if theme::icon_button(ui, ICON_IMAGE, false, "Ouvrir une image dans l'éditeur").clicked() {
                        actions.push(WsAction::OpenFile);
                    }
                    if theme::icon_button(ui, ICON_CONTENT_PASTE, false, &crate::platform::sc("Éditer l'image du presse-papiers (Ctrl+V)")).clicked() {
                        actions.push(WsAction::Paste);
                    }
                    if theme::icon_button(ui, ICON_REFRESH, false, "Actualiser").clicked() {
                        self.refresh();
                    }
                });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                for (f, label) in [(Filter::All, "Tout"), (Filter::Images, "Images"), (Filter::Videos, "Vidéos")] {
                    if theme::filter_chip(ui, label, self.filter == f).clicked() {
                        self.filter = f;
                    }
                }
                ui.add_space(12.0);
                ui.label(egui::RichText::new(ICON_SEARCH).size(20.0).color(p.on_surface_variant));
                theme::text_field(ui, &mut self.search, 200.0, "Rechercher…");
            });
            ui.add_space(14.0);

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
                    ui.label(egui::RichText::new("Aucune capture pour l'instant").font(theme::medium(16.0)).color(p.on_surface));
                    ui.label(egui::RichText::new(format!("Appuyez sur {} pour capturer une zone de l'écran.", pretty_hotkey(&settings.hotkeys.region))).color(p.on_surface_variant));
                });
                return;
            }

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                for i in visible {
                    self.card(ui, i, &mut actions);
                }
            });
            ui.add_space(24.0);
        });

        if let Some(path) = self.confirm_delete.clone() {
            let mut keep_open = true;
            egui::Window::new("Supprimer la capture ?")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("« {} » sera envoyé dans la Corbeille.", path.file_name().unwrap_or_default().to_string_lossy()));
                    ui.add_space(16.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::button(ui, ButtonKind::Danger, Some(ICON_DELETE), "Supprimer").clicked() {
                            actions.push(WsAction::Delete(path.clone()));
                            keep_open = false;
                        }
                        if theme::button(ui, ButtonKind::Text, None, "Annuler").clicked() {
                            keep_open = false;
                        }
                    });
                });
            if !keep_open {
                self.confirm_delete = None;
            }
        }
        actions
    }

    fn card(&mut self, ui: &mut Ui, i: usize, actions: &mut Vec<WsAction>) {
        let p = palette(ui.ctx());
        let it = &self.items[i];
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
        let painter = ui.painter();
        let hovered = resp.hovered();
        if hovered {
            painter.rect_filled(rect.translate(vec2(0.0, 2.0)), 12.0, Color32::from_black_alpha(35));
        }
        painter.rect(rect, 12.0, if hovered { p.surface_high } else { p.surface_low }, Stroke::new(1.0_f32, p.outline_variant), StrokeKind::Inside);
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

        // Barre d'actions au survol.
        let mut consumed = false;
        let show_bar = hovered || resp.contains_pointer();
        {
            let bar = Rect::from_min_size(pos2(thumb_area.right() - 4.0 - 4.0 * 34.0, thumb_area.bottom() - 40.0), vec2(4.0 * 34.0, 34.0));
            if show_bar {
                painter.rect_filled(bar, 17.0, p.surface_high);
            }
            let icons: [(&str, &str, u8); 4] = [
                (if is_video { ICON_PLAY_ARROW } else { ICON_EDIT }, if is_video { "Lire" } else { "Éditer" }, 0),
                (ICON_CONTENT_COPY, "Copier", 1),
                (ICON_FOLDER_OPEN, if cfg!(target_os = "macos") { "Afficher dans le Finder" } else { "Afficher dans l'Explorateur" }, 2),
                (ICON_DELETE, "Supprimer", 3),
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
                        _ => self.confirm_delete = Some(path.clone()),
                    }
                }
            }
        }

        if !consumed && resp.double_clicked() {
            actions.push(if is_video { WsAction::Open(path.clone()) } else { WsAction::Edit(path.clone()) });
        }
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        resp.context_menu(|ui| {
            if !is_video && ui.button(format!("{ICON_EDIT}  Éditer")).clicked() {
                actions.push(WsAction::Edit(path.clone()));
                ui.close_menu();
            }
            if ui.button(format!("{ICON_OPEN_IN_NEW}  Ouvrir avec l'application par défaut")).clicked() {
                actions.push(WsAction::Open(path.clone()));
                ui.close_menu();
            }
            if ui.button(format!("{ICON_CONTENT_COPY}  Copier")).clicked() {
                actions.push(WsAction::Copy(path.clone()));
                ui.close_menu();
            }
            if ui.button(format!("{ICON_FOLDER_OPEN}  Afficher dans {}", crate::platform::FILE_MANAGER)).clicked() {
                actions.push(WsAction::Reveal(path.clone()));
                ui.close_menu();
            }
            ui.separator();
            if ui.button(format!("{ICON_DELETE}  Supprimer")).clicked() {
                self.confirm_delete = Some(path.clone());
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
        format!("{:.1} Mo", b as f64 / (1u64 << 20) as f64)
    } else {
        format!("{} Ko", (b + 1023) / 1024)
    }
}

/// « ctrl+shift+PrintScreen » → « Ctrl + Maj + Impr. écran ».
pub fn pretty_hotkey(s: &str) -> String {
    s.split('+')
        .map(|t| match t.trim().to_lowercase().as_str() {
            "ctrl" | "control" => if cfg!(target_os = "macos") { "⌃" } else { "Ctrl" }.to_string(),
            "shift" => if cfg!(target_os = "macos") { "⇧" } else { "Maj" }.to_string(),
            "alt" | "option" => if cfg!(target_os = "macos") { "⌥" } else { "Alt" }.to_string(),
            "super" | "win" | "meta" | "cmd" | "command" => if cfg!(target_os = "macos") { "⌘" } else { "Win" }.to_string(),
            "printscreen" => "Impr. écran".to_string(),
            other => {
                let o = other.trim_start_matches("key").trim_start_matches("digit");
                o.to_uppercase()
            }
        })
        .collect::<Vec<_>>()
        .join(" + ")
}
