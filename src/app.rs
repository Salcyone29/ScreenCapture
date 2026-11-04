//! Application : machine à états (espace de travail / éditeur / surcouche / enregistrement),
//! icône de notification, raccourcis globaux et page des paramètres.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, vec2, Align2, Color32, CornerRadius, FontId, Key, Margin, Vec2};
use egui_material_icons::icons::*;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use image::RgbaImage;
use raw_window_handle::HasWindowHandle;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::capture::{self, Rect};
use crate::editor::{Editor, EditorAction};
use crate::overlay::{Overlay, OverlayResult, Purpose};
use crate::platform::{self, SavedWindow, Window};
use crate::recorder::{self, Recorder};
use crate::settings::{Accent, AfterCapture, ImageFormat, Settings, ThemeChoice, VideoFormat};
use crate::theme::{self, palette, ButtonKind};
use crate::workspace::{pretty_hotkey, Workspace, WsAction};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CaptureKind {
    Region,
    Fullscreen,
    Window,
    LastRegion,
    Delayed,
    Video,
}

pub enum AppEvent {
    Hotkey(u32),
    Shortcut(CaptureKind),
    Menu(String),
    TrayOpen,
    Captured { kind: CaptureKind, image: RgbaImage, origin: Rect, target: Option<Rect> },
    CaptureFailed(String),
    Ocr(Result<String, String>),
    VideoDone(Result<PathBuf, String>),
    AudioDevices(Vec<String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Workspace,
    Editor,
    Overlay,
    Recording,
    Popup,
}

/// Miniature affichée en bas à droite après une capture (option « Espace de travail »).
struct CapturePopup {
    image: RgbaImage,
    path: Option<PathBuf>,
    texture: Option<egui::TextureHandle>,
    since: Instant,
}

const POPUP_DELAY: Duration = Duration::from_secs(6);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Settings,
    About,
}

struct Hotkeys {
    manager: Option<GlobalHotKeyManager>,
    registered: Vec<(HotKey, CaptureKind)>,
    status: Vec<(CaptureKind, bool)>,
    /// Raccourcis utilisant Impr. écran, servis par le hook clavier (masque de modificateurs).
    printscreen: Arc<Mutex<Vec<(u8, CaptureKind)>>>,
    override_snip: Arc<std::sync::atomic::AtomicBool>,
    hook_ok: bool,
}

impl Hotkeys {
    fn apply(&mut self, s: &crate::settings::Hotkeys) {
        let Some(m) = &self.manager else { return };
        for (h, _) in self.registered.drain(..) {
            let _ = m.unregister(h);
        }
        self.status.clear();
        let mut prt = Vec::new();
        for (kind, spec) in [
            (CaptureKind::Region, &s.region),
            (CaptureKind::Fullscreen, &s.fullscreen),
            (CaptureKind::Window, &s.window),
            (CaptureKind::LastRegion, &s.last_region),
            (CaptureKind::Video, &s.video),
        ] {
            if spec.trim().is_empty() {
                self.status.push((kind, false));
                continue;
            }
            let ok = match spec.trim().parse::<HotKey>() {
                Ok(h) if h.key == Code::PrintScreen && self.hook_ok => {
                    // Le hook capte la touche quand une autre application a le focus ;
                    // RegisterHotKey prend le relais quand c'est notre fenêtre.
                    let mut mask = 0;
                    for (m, bit) in [
                        (Modifiers::CONTROL, platform::MOD_CTRL),
                        (Modifiers::SHIFT, platform::MOD_SHIFT),
                        (Modifiers::ALT, platform::MOD_ALT),
                        (Modifiers::SUPER, platform::MOD_WIN),
                    ] {
                        if h.mods.contains(m) {
                            mask |= bit;
                        }
                    }
                    if prt.iter().any(|(m, _)| *m == mask) {
                        false
                    } else {
                        prt.push((mask, kind));
                        if m.register(h).is_ok() {
                            self.registered.push((h, kind));
                        }
                        true
                    }
                }
                Ok(h) => {
                    let ok = m.register(h).is_ok();
                    if ok {
                        self.registered.push((h, kind));
                    }
                    ok
                }
                Err(_) => false,
            };
            self.status.push((kind, ok));
        }
        if let Ok(mut p) = self.printscreen.lock() {
            *p = prt;
        }
        self.override_snip.store(s.override_snip && self.hook_ok, std::sync::atomic::Ordering::SeqCst);
    }

    fn kind_of(&self, id: u32) -> Option<CaptureKind> {
        self.registered.iter().find(|(h, _)| h.id() == id).map(|(_, k)| *k)
    }
}

pub struct App {
    win: Window,
    mode: Mode,
    saved: Option<SavedWindow>,
    visible: bool,
    visible_before_capture: bool,
    page: Page,
    settings: Settings,
    workspace: Workspace,
    editor: Option<Editor>,
    overlay: Option<Overlay>,
    recorder: Option<Recorder>,
    popup: Option<CapturePopup>,
    finishing_video: bool,
    tx: Sender<AppEvent>,
    rx: Receiver<AppEvent>,
    hotkeys: Hotkeys,
    _tray: Option<TrayIcon>,
    menu_ids: HashMap<String, &'static str>,
    snackbar: Option<(String, Instant)>,
    ocr_text: Option<String>,
    busy: bool,
    /// Début de la capture en cours, pour ne jamais rester bloqué en « occupé ».
    busy_since: Instant,
    quit: bool,
    applied_theme: Option<(ThemeChoice, Accent, bool)>,
    hotkey_edit: crate::settings::Hotkeys,
    audio_devices: Vec<String>,
    mic_refresh: Option<Instant>,
    /// Pastille du nuancier en cours de modification dans les Paramètres.
    palette_sel: Option<usize>,
    palette_hex: (usize, String),
    screen_check: Instant,
    park_on_start: bool,
    title: String,
    last_shortcut: Option<(CaptureKind, Instant)>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, settings: Settings, start_hidden: bool) -> Self {
        let ctx = cc.egui_ctx.clone();
        theme::install_fonts(&ctx);
        let hwnd = cc.window_handle().map(|h| platform::native_handle(h.as_raw())).unwrap_or(0);
        let (tx, rx) = channel();

        // Raccourcis globaux.
        let manager = GlobalHotKeyManager::new().ok();
        {
            let tx = tx.clone();
            let ctx = ctx.clone();
            GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
                crate::debug_log(&format!("hotkey {} {:?}", e.id, e.state));
                if e.state == HotKeyState::Pressed {
                    let _ = tx.send(AppEvent::Hotkey(e.id));
                    ctx.request_repaint();
                }
            }));
        }
        let mut hotkeys = Hotkeys {
            manager,
            registered: Vec::new(),
            status: Vec::new(),
            printscreen: Arc::new(Mutex::new(Vec::new())),
            override_snip: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            hook_ok: false,
        };
        {
            let prt = hotkeys.printscreen.clone();
            let snip = hotkeys.override_snip.clone();
            // Le hook clavier doit répondre en quelques millisecondes, sinon Windows le retire :
            // il ne fait que poster l'évènement, un thread relais réveille l'interface.
            let tx = relay(tx.clone(), ctx.clone());
            hotkeys.hook_ok = platform::install_printscreen_hook(Box::new(move |m| {
                let kind = if m & platform::KEY_SNIP != 0 {
                    snip.load(std::sync::atomic::Ordering::SeqCst).then_some(CaptureKind::Region)
                } else {
                    prt.lock().ok().and_then(|v| v.iter().find(|(mask, _)| *mask == m & 0x0F).map(|(_, k)| *k))
                };
                match kind {
                    Some(k) => {
                        if m & platform::NO_FIRE == 0 {
                            crate::debug_log(&format!("raccourci (hook) -> {k:?}"));
                            let _ = tx.send(AppEvent::Shortcut(k));
                        }
                        true
                    }
                    None => false,
                }
            }));
        }
        hotkeys.apply(&settings.hotkeys);
        crate::diag::log(&format!("hook Impr. écran : {}", if hotkeys.hook_ok { "installé" } else { "indisponible" }));
        crate::diag::start_watchdog(&ctx);
        {
            // Nouvelle instance lancée (menu Démarrer…) : afficher la fenêtre. Si l'interface
            // ne répond plus, quitter pour que la nouvelle instance prenne le relais.
            let tx = tx.clone();
            let ctx = ctx.clone();
            platform::activation_listener(move || {
                crate::diag::log("demande d'affichage par une nouvelle instance");
                let _ = tx.send(AppEvent::TrayOpen);
                if !crate::diag::ui_responds(&ctx, Duration::from_secs(4)) {
                    crate::diag::log(&format!("interface bloquée ({}) : arrêt pour relance", crate::diag::describe_block()));
                    std::process::exit(3);
                }
            });
        }

        // Icône de la zone de notification.
        let mut menu_ids = HashMap::new();
        let tray = build_tray(&mut menu_ids, &settings).ok();
        {
            let tx = tx.clone();
            let ctx = ctx.clone();
            // Ces gestionnaires tournent hors de la boucle egui : si la fenêtre est restée sur un
            // écran débranché, eframe peut ne plus rafraîchir l'interface ; on la ramène d'abord.
            let win = Window(hwnd);
            let quit_id = menu_ids.iter().find(|(_, v)| **v == "quit").map(|(k, _)| k.clone());
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                crate::diag::step("menu de notification");
                win.rescue();
                if Some(&e.id.0) == quit_id.as_ref() {
                    // Filet de sécurité : quitter même si l'interface ne traite plus les évènements.
                    std::thread::spawn(|| {
                        std::thread::sleep(Duration::from_secs(4));
                        std::process::exit(0);
                    });
                }
                let _ = tx.send(AppEvent::Menu(e.id.0.clone()));
                ctx.request_repaint();
            }));
        }
        {
            let tx = tx.clone();
            let ctx = ctx.clone();
            let win = Window(hwnd);
            TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
                crate::diag::step("clic sur l'icône de notification");
                win.rescue();
                let open = matches!(
                    e,
                    TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }
                        | TrayIconEvent::DoubleClick { button: MouseButton::Left, .. }
                );
                if open {
                    let _ = tx.send(AppEvent::TrayOpen);
                    ctx.request_repaint();
                }
            }));
        }

        let hotkey_edit = settings.hotkeys.clone();
        Self {
            win: Window(hwnd),
            mode: Mode::Workspace,
            saved: None,
            visible: true,
            visible_before_capture: true,
            page: Page::Home,
            workspace: Workspace::new(&ctx),
            settings,
            editor: None,
            overlay: None,
            recorder: None,
            popup: None,
            finishing_video: false,
            tx,
            rx,
            hotkeys,
            _tray: tray,
            menu_ids,
            snackbar: None,
            ocr_text: None,
            busy: false,
            busy_since: Instant::now(),
            quit: false,
            applied_theme: None,
            hotkey_edit,
            audio_devices: Vec::new(),
            mic_refresh: None,
            palette_sel: None,
            palette_hex: (usize::MAX, String::new()),
            screen_check: Instant::now(),
            park_on_start: start_hidden,
            title: String::new(),
            last_shortcut: None,
        }
    }

    fn toast(&mut self, msg: impl Into<String>) {
        self.snackbar = Some((msg.into(), Instant::now()));
    }

    // ------------------------------------------------------------------ fenêtre

    fn show_main(&mut self, ctx: &egui::Context) {
        crate::diag::step("affichage de la fenêtre");
        crate::diag::log(&format!("affichage de la fenêtre (mode {:?}, écrans {:?})", self.mode, capture::monitors()));
        match self.saved.take() {
            Some(s) => self.win.restore(&s),
            None => self.win.focus(),
        }
        self.visible = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }

    fn hide_main(&mut self) {
        crate::diag::step("masquage de la fenêtre");
        if self.saved.is_none() {
            self.saved = self.win.save();
        }
        self.win.park();
        self.visible = false;
    }

    fn enter_popup(&mut self, r: Rect, exclude: bool, activate: bool) {
        if self.saved.is_none() {
            self.saved = self.win.save();
        }
        crate::diag::step("fenêtre flottante");
        crate::diag::log(&format!("fenêtre flottante {r:?} (mode {:?})", self.mode));
        self.win.popup(r, exclude, activate);
        self.visible = true;
    }

    // ------------------------------------------------------------------ capture

    fn start_capture(&mut self, kind: CaptureKind, ctx: &egui::Context, from_hotkey: bool) {
        if kind == CaptureKind::Video && self.recorder.is_some() {
            self.stop_recording(ctx);
            return;
        }
        // Une capture qui ne revient pas (pilote, écran en veille…) ne doit pas tout bloquer.
        if self.busy && self.busy_since.elapsed() > Duration::from_secs(20) {
            crate::diag::log("capture précédente sans réponse depuis 20 s : abandonnée");
            self.busy = false;
        }
        if self.busy || self.overlay.is_some() || self.recorder.is_some() {
            crate::diag::log(&format!(
                "capture {kind:?} ignorée (occupé {}, sélection {}, vidéo {})",
                self.busy,
                self.overlay.is_some(),
                self.recorder.is_some()
            ));
            return;
        }
        crate::diag::log(&format!("capture {kind:?} (raccourci {from_hotkey}, mode {:?})", self.mode));
        // Une nouvelle capture remplace la miniature éventuellement affichée.
        if self.popup.take().is_some() {
            self.win.round_corners(false);
            self.mode = if self.editor.is_some() { Mode::Editor } else { Mode::Workspace };
        }
        // Capture de fenêtre par raccourci : fenêtre au premier plan (si ce n'est pas la nôtre).
        let target = if kind == CaptureKind::Window && from_hotkey && !(self.visible && platform::is_foreground(self.win)) {
            capture::foreground_window_rect()
        } else {
            None
        };
        if kind == CaptureKind::LastRegion && self.settings.last_region.is_none() {
            self.toast("Aucune zone précédente : sélectionnez une zone.");
        }
        let was_visible = self.visible;
        self.visible_before_capture = was_visible || self.mode == Mode::Editor;
        self.hide_main();
        self.busy = true;
        self.busy_since = Instant::now();
        let delay = match kind {
            CaptureKind::Delayed => Duration::from_secs(self.settings.delay_seconds as u64),
            _ if was_visible => Duration::from_millis(280),
            _ => Duration::from_millis(40),
        };
        let cursor = self.settings.capture_cursor;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            let origin = capture::virtual_screen();
            let started = Instant::now();
            let result = std::panic::catch_unwind(|| capture::capture_rect(origin, cursor));
            let ev = match result {
                Ok(Ok(image)) => AppEvent::Captured { kind, image, origin, target },
                Ok(Err(e)) => AppEvent::CaptureFailed(e.to_string()),
                Err(_) => AppEvent::CaptureFailed("erreur interne pendant la capture".into()),
            };
            crate::diag::log(&format!(
                "écran capturé {origin:?} en {} ms : {}",
                started.elapsed().as_millis(),
                match &ev {
                    AppEvent::CaptureFailed(e) => e.as_str(),
                    _ => "ok",
                }
            ));
            let _ = tx.send(ev);
            ctx.request_repaint();
        });
    }

    fn on_captured(&mut self, ctx: &egui::Context, kind: CaptureKind, image: RgbaImage, origin: Rect, target: Option<Rect>) {
        self.busy = false;
        let direct = match kind {
            CaptureKind::Fullscreen => Some(origin),
            CaptureKind::Window => target,
            CaptureKind::LastRegion => self.settings.last_region,
            _ => None,
        };
        if let Some(r) = direct {
            match capture::crop_virtual(&image, origin, r) {
                Some(img) => self.finish_capture(ctx, img),
                None => self.restore_after_cancel(ctx),
            }
            return;
        }
        let purpose = if kind == CaptureKind::Video { Purpose::Video } else { Purpose::Image };
        self.overlay = Some(Overlay::new(purpose, origin, image));
        self.mode = Mode::Overlay;
        self.enter_popup(origin, false, true);
        ctx.request_repaint();
    }

    fn show_capture_popup(&mut self, ctx: &egui::Context, image: RgbaImage, path: Option<PathBuf>) {
        let ppp = platform::units_per_point(ctx);
        let (cx, cy) = capture::cursor_pos();
        let wa = capture::work_area_at(cx, cy);
        let (w, h) = ((336.0 * ppp) as i32, (282.0 * ppp) as i32);
        let margin = (16.0 * ppp) as i32;
        let r = Rect::new(wa.x + wa.w - w - margin, wa.y + wa.h - h - margin, w, h);
        self.popup = Some(CapturePopup { image, path, texture: None, since: Instant::now() });
        self.mode = Mode::Popup;
        self.enter_popup(r, false, false);
        self.win.round_corners(true);
        ctx.request_repaint();
    }

    fn close_capture_popup(&mut self, ctx: &egui::Context) {
        self.popup = None;
        self.win.round_corners(false);
        self.restore_after_cancel(ctx);
    }

    fn capture_popup_ui(&mut self, ctx: &egui::Context) {
        let p = palette(ctx);
        let Some(pop) = self.popup.as_mut() else {
            self.restore_after_cancel(ctx);
            return;
        };
        if pop.texture.is_none() {
            let t = crate::workspace::fit_thumbnail(&pop.image, 640, 360);
            let ci = egui::ColorImage::from_rgba_unmultiplied([t.width() as usize, t.height() as usize], t.as_raw());
            pop.texture = Some(ctx.load_texture("capture-popup", ci, egui::TextureOptions::LINEAR));
        }
        // Le survol suspend la disparition automatique.
        if ctx.input(|i| i.pointer.has_pointer()) {
            pop.since = Instant::now();
        }
        let remaining = 1.0 - pop.since.elapsed().as_secs_f32() / POPUP_DELAY.as_secs_f32();
        let name = pop.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Capture".into());
        let size = (pop.image.width(), pop.image.height());
        let tex = pop.texture.clone();
        let title = if self.settings.copy_to_clipboard { "Capture enregistrée et copiée" } else { "Capture enregistrée" };
        let (mut edit, mut close, mut copy, mut reveal) = (false, false, false, false);

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.surface_container).inner_margin(Margin::same(12)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(ICON_CHECK_CIRCLE).size(20.0).color(p.primary));
                    ui.label(egui::RichText::new(title).font(theme::medium(14.0)).color(p.on_surface));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = theme::icon_button(ui, ICON_CLOSE, false, "Fermer").clicked();
                    });
                });
                // Miniature cliquable : ouvre l'éditeur.
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 160.0), egui::Sense::click());
                let painter = ui.painter();
                painter.rect_filled(rect, 12.0, p.surface_highest);
                if let Some(t) = &tex {
                    let ts = t.size_vec2();
                    let k = (rect.width() / ts.x).min(rect.height() / ts.y);
                    let img_rect = egui::Rect::from_center_size(rect.center(), ts * k);
                    painter.image(t.id(), img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                }
                if resp.hovered() {
                    painter.rect_filled(rect, 12.0, Color32::from_black_alpha(110));
                    let chip = egui::Rect::from_center_size(rect.center(), vec2(120.0, 40.0));
                    painter.rect_filled(chip, 20.0, p.primary);
                    painter.text(chip.left_center() + vec2(28.0, 0.0), Align2::CENTER_CENTER, ICON_EDIT, FontId::proportional(18.0), p.on_primary);
                    painter.text(chip.left_center() + vec2(46.0, 0.0), Align2::LEFT_CENTER, "Éditer", theme::medium(14.0), p.on_primary);
                }
                edit = resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Ouvrir dans l'éditeur").clicked();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.add_space(2.0);
                        let short = if name.chars().count() > 30 { format!("{}…", name.chars().take(29).collect::<String>()) } else { name.clone() };
                        ui.label(egui::RichText::new(short).size(12.5).color(p.on_surface));
                        ui.label(egui::RichText::new(format!("{} × {} px", size.0, size.1)).size(11.5).color(p.on_surface_variant));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        reveal = theme::icon_button(ui, ICON_FOLDER_OPEN, false, &format!("Afficher dans {}", platform::FILE_MANAGER)).clicked();
                        copy = theme::icon_button(ui, ICON_CONTENT_COPY, false, "Copier").clicked();
                    });
                });
            });
        // Barre de temps restant avant disparition, au bas de la fenêtre.
        let screen = ctx.screen_rect();
        let bar = egui::Rect::from_min_size(
            egui::pos2(screen.left(), screen.bottom() - 4.0),
            vec2(screen.width() * remaining.clamp(0.0, 1.0), 4.0),
        );
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("popup_timer"))).rect_filled(bar, 0.0, p.primary);
        ctx.request_repaint_after(Duration::from_millis(50));

        if copy {
            let r = self.popup.as_ref().map(|pop| copy_image(&pop.image));
            match r {
                Some(Ok(())) => self.toast("Copié"),
                Some(Err(e)) => self.toast(format!("Erreur : {e}")),
                None => {}
            }
        }
        if reveal {
            if let Some(path) = self.popup.as_ref().and_then(|pop| pop.path.clone()) {
                platform::reveal_in_explorer(&path);
            }
        }
        if edit {
            if let Some(pop) = self.popup.take() {
                self.win.round_corners(false);
                self.open_editor(pop.image, pop.path);
                self.show_main(ctx);
            }
        } else if close || remaining <= 0.0 {
            self.close_capture_popup(ctx);
        }
    }

    fn restore_after_cancel(&mut self, ctx: &egui::Context) {
        self.mode = if self.editor.is_some() { Mode::Editor } else { Mode::Workspace };
        if self.visible_before_capture {
            self.show_main(ctx);
        } else {
            self.hide_main();
        }
    }

    fn finish_capture(&mut self, ctx: &egui::Context, img: RgbaImage) {
        if self.settings.copy_to_clipboard {
            if let Err(e) = copy_image(&img) {
                self.toast(format!("Presse-papiers indisponible : {e}"));
            }
        }
        let mut path = None;
        if self.settings.auto_save {
            match self.save_new(&img) {
                Ok(p) => path = Some(p),
                Err(e) => self.toast(format!("Enregistrement impossible : {e}")),
            }
        }
        match self.settings.after_capture {
            AfterCapture::OpenEditor => {
                self.close_editor(true);
                self.editor = Some(Editor::new(img, path));
                self.mode = Mode::Editor;
                self.show_main(ctx);
            }
            AfterCapture::ShowWorkspace => {
                self.page = Page::Home;
                self.show_capture_popup(ctx, img, path);
            }
            AfterCapture::Nothing => self.restore_after_cancel(ctx),
        }
    }

    fn new_capture_path(&self, ext: &str) -> PathBuf {
        self.unique_path("Capture", ext)
    }

    /// `<préfixe>_<date>.<ext>` dans le dossier des captures, sans écraser un fichier existant.
    fn unique_path(&self, prefix: &str, ext: &str) -> PathBuf {
        let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let mut p = self.settings.save_dir.join(format!("{prefix}_{stamp}.{ext}"));
        let mut n = 2;
        while p.exists() {
            p = self.settings.save_dir.join(format!("{prefix}_{stamp}_{n}.{ext}"));
            n += 1;
        }
        p
    }

    fn save_new(&mut self, img: &RgbaImage) -> anyhow::Result<PathBuf> {
        std::fs::create_dir_all(&self.settings.save_dir)?;
        let path = self.new_capture_path(self.settings.image_format.ext());
        save_image(img, &path, self.settings.jpeg_quality)?;
        self.workspace.refresh();
        Ok(path)
    }

    // ------------------------------------------------------------------ éditeur

    fn open_editor(&mut self, img: RgbaImage, path: Option<PathBuf>) {
        self.close_editor(true);
        self.editor = Some(Editor::new(img, path));
        self.mode = Mode::Editor;
    }

    /// Ferme l'éditeur ; les modifications sont enregistrées automatiquement.
    fn close_editor(&mut self, autosave: bool) {
        let Some(mut ed) = self.editor.take() else { return };
        if autosave && ed.modified {
            let _ = self.save_editor(&mut ed, None);
        }
        if self.mode == Mode::Editor {
            self.mode = Mode::Workspace;
        }
    }

    fn save_editor(&mut self, ed: &mut Editor, target: Option<PathBuf>) -> anyhow::Result<PathBuf> {
        let img = ed.export_image(&self.settings);
        let path = match target.or_else(|| ed.path.clone()) {
            Some(p) => p,
            None => {
                std::fs::create_dir_all(&self.settings.save_dir)?;
                self.new_capture_path(self.settings.image_format.ext())
            }
        };
        save_image(&img, &path, self.settings.jpeg_quality)?;
        ed.path = Some(path.clone());
        ed.modified = false;
        self.workspace.forget(&path);
        Ok(path)
    }

    fn editor_actions(&mut self, ctx: &egui::Context, actions: Vec<EditorAction>) {
        for a in actions {
            let Some(mut ed) = self.editor.take() else { return };
            match a {
                EditorAction::Close => {
                    self.editor = Some(ed);
                    let modified = self.editor.as_ref().map(|e| e.modified).unwrap_or(false);
                    self.close_editor(true);
                    if modified {
                        self.toast("Modifications enregistrées");
                    }
                    continue;
                }
                EditorAction::Save => match self.save_editor(&mut ed, None) {
                    Ok(p) => self.toast(format!("Enregistré : {}", p.file_name().unwrap_or_default().to_string_lossy())),
                    Err(e) => self.toast(format!("Erreur : {e}")),
                },
                EditorAction::SaveAs => {
                    let name = ed.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Capture".into());
                    let file = rfd::FileDialog::new()
                        .set_directory(&self.settings.save_dir)
                        .set_file_name(format!("{name}.png"))
                        .add_filter("Image PNG", &["png"])
                        .add_filter("Image JPEG", &["jpg", "jpeg"])
                        .add_filter("Image BMP", &["bmp"])
                        .save_file();
                    if let Some(f) = file {
                        match self.save_editor(&mut ed, Some(f)) {
                            Ok(p) => self.toast(format!("Enregistré : {}", p.display())),
                            Err(e) => self.toast(format!("Erreur : {e}")),
                        }
                    }
                }
                EditorAction::Copy => {
                    let img = ed.export_image(&self.settings);
                    match copy_image(&img) {
                        Ok(()) => self.toast("Image copiée dans le presse-papiers"),
                        Err(e) => self.toast(format!("Erreur : {e}")),
                    }
                }
                EditorAction::Ocr => {
                    let img = ed.export_image(&Settings { effect_border: false, effect_shadow: false, ..self.settings.clone() });
                    let tx = self.tx.clone();
                    let ctx = ctx.clone();
                    self.toast("Reconnaissance du texte…");
                    std::thread::spawn(move || {
                        let r = platform::ocr(&img).map_err(|e| e.to_string());
                        let _ = tx.send(AppEvent::Ocr(r));
                        ctx.request_repaint();
                    });
                }
            }
            self.editor = Some(ed);
        }
    }

    // ------------------------------------------------------------------ vidéo

    fn start_recording(&mut self, ctx: &egui::Context, region: Rect) {
        let out = self.new_capture_path(self.settings.video_format.ext()).with_file_name(format!(
            "Video_{}.{}",
            chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"),
            self.settings.video_format.ext()
        ));
        match Recorder::start(&self.settings, region, out) {
            Ok(rec) => {
                if let Some(w) = &rec.warning {
                    self.toast(w.clone());
                }
                let ppp = platform::units_per_point(ctx);
                let (w, h) = ((360.0 * ppp) as i32, (60.0 * ppp) as i32);
                let mon = capture::monitor_at(region.x + region.w / 2, region.y + region.h / 2);
                let x = (region.x + region.w / 2 - w / 2).clamp(mon.x, mon.x + mon.w - w);
                let y = if region.y - h - 12 >= mon.y {
                    region.y - h - 12
                } else if region.y + region.h + 12 + h <= mon.y + mon.h {
                    region.y + region.h + 12
                } else {
                    mon.y + 12
                };
                self.recorder = Some(rec);
                self.mode = Mode::Recording;
                self.enter_popup(Rect::new(x, y, w, h), true, false);
            }
            Err(e) => {
                self.restore_after_cancel(ctx);
                self.toast(e.to_string());
            }
        }
    }

    fn stop_recording(&mut self, ctx: &egui::Context) {
        let Some(rec) = self.recorder.take() else { return };
        self.finishing_video = true;
        let tx = self.tx.clone();
        let c = ctx.clone();
        std::thread::spawn(move || {
            let r = rec.finish().map_err(|e| e.to_string());
            let _ = tx.send(AppEvent::VideoDone(r));
            c.request_repaint();
        });
        self.mode = if self.editor.is_some() { Mode::Editor } else { Mode::Workspace };
        self.page = Page::Home;
        self.show_main(ctx);
        self.toast(if self.settings.video_format == VideoFormat::Gif { "Conversion en GIF…" } else { "Finalisation de la vidéo…" });
    }

    fn cancel_recording(&mut self, ctx: &egui::Context) {
        if let Some(rec) = self.recorder.take() {
            rec.abort();
        }
        self.restore_after_cancel(ctx);
        self.toast("Enregistrement annulé");
    }

    // ------------------------------------------------------------------ événements

    fn handle_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.rx.try_recv() {
            crate::diag::step("traitement d'un évènement");
            match &ev {
                AppEvent::Hotkey(id) => crate::diag::log(&format!("raccourci global {id}")),
                AppEvent::Shortcut(k) => crate::diag::log(&format!("raccourci Impr. écran {k:?}")),
                AppEvent::Menu(id) => crate::diag::log(&format!("menu {:?}", self.menu_ids.get(id))),
                AppEvent::TrayOpen => crate::diag::log(&format!("ouverture demandée (mode {:?})", self.mode)),
                _ => {}
            }
            match ev {
                AppEvent::Hotkey(id) => {
                    if let Some(k) = self.hotkeys.kind_of(id) {
                        self.shortcut(k, ctx);
                    }
                }
                AppEvent::Shortcut(k) => self.shortcut(k, ctx),
                AppEvent::Menu(id) => match self.menu_ids.get(&id).copied() {
                    Some("region") => self.start_capture(CaptureKind::Region, ctx, false),
                    Some("full") => self.start_capture(CaptureKind::Fullscreen, ctx, false),
                    Some("window") => self.start_capture(CaptureKind::Window, ctx, false),
                    Some("last") => self.start_capture(CaptureKind::LastRegion, ctx, false),
                    Some("delayed") => self.start_capture(CaptureKind::Delayed, ctx, false),
                    Some("video") => self.start_capture(CaptureKind::Video, ctx, false),
                    Some("open") => {
                        if self.mode != Mode::Editor {
                            self.page = Page::Home;
                        }
                        self.show_main(ctx);
                    }
                    Some("folder") => platform::open_folder(&self.settings.save_dir),
                    Some("settings") => {
                        self.close_editor(true);
                        self.mode = Mode::Workspace;
                        self.page = Page::Settings;
                        self.show_main(ctx);
                    }
                    Some("quit") => self.request_quit(ctx),
                    _ => {}
                },
                AppEvent::TrayOpen => {
                    // Sélection ou miniature restée affichée (écran débranché…) : revenir à l'espace de travail.
                    match self.mode {
                        Mode::Overlay => {
                            self.overlay = None;
                            self.mode = if self.editor.is_some() { Mode::Editor } else { Mode::Workspace };
                        }
                        Mode::Popup => {
                            self.popup = None;
                            self.win.round_corners(false);
                            self.mode = if self.editor.is_some() { Mode::Editor } else { Mode::Workspace };
                        }
                        _ => {}
                    }
                    if self.mode == Mode::Workspace || self.mode == Mode::Editor {
                        self.show_main(ctx);
                    }
                }
                AppEvent::Captured { kind, image, origin, target } => self.on_captured(ctx, kind, image, origin, target),
                AppEvent::CaptureFailed(e) => {
                    self.busy = false;
                    self.restore_after_cancel(ctx);
                    self.toast(format!("Échec de la capture : {e}"));
                }
                AppEvent::Ocr(r) => match r {
                    Ok(t) if t.trim().is_empty() => self.toast("Aucun texte détecté"),
                    Ok(t) => {
                        let _ = arboard::Clipboard::new().and_then(|mut c| c.set_text(t.clone()));
                        self.ocr_text = Some(t);
                    }
                    Err(e) => self.toast(format!("OCR indisponible : {e}")),
                },
                AppEvent::VideoDone(r) => {
                    self.finishing_video = false;
                    match r {
                        Ok(p) => {
                            self.workspace.refresh();
                            self.toast(format!("Vidéo enregistrée : {}", p.file_name().unwrap_or_default().to_string_lossy()));
                        }
                        Err(e) => self.toast(format!("Erreur vidéo : {e}")),
                    }
                }
                AppEvent::AudioDevices(d) => self.audio_devices = d,
            }
        }
    }

    /// Un même appui peut arriver par le hook et par RegisterHotKey : on l'ignore s'il est trop rapproché.
    fn shortcut(&mut self, k: CaptureKind, ctx: &egui::Context) {
        if let Some((last, at)) = self.last_shortcut {
            if last == k && at.elapsed() < Duration::from_millis(400) {
                return;
            }
        }
        self.last_shortcut = Some((k, Instant::now()));
        self.start_capture(k, ctx, true);
    }

    fn request_quit(&mut self, ctx: &egui::Context) {
        crate::diag::log("arrêt demandé");
        self.close_editor(true);
        if let Some(rec) = self.recorder.take() {
            let _ = rec.finish();
        }
        self.settings.save();
        self.quit = true;
        if let Some(s) = self.saved.take() {
            self.win.restore(&s);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn set_title(&mut self, ctx: &egui::Context, title: String) {
        if self.title != title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn apply_theme_if_needed(&mut self, ctx: &egui::Context) {
        let sys_dark = ctx.system_theme().map(|t| t == egui::Theme::Dark).unwrap_or(false);
        let key = (self.settings.theme, self.settings.accent, sys_dark);
        if self.applied_theme != Some(key) {
            theme::apply(ctx, self.settings.theme, self.settings.accent);
            self.applied_theme = Some(key);
        }
    }

    // ------------------------------------------------------------------ pages

    fn nav_rail(&mut self, ctx: &egui::Context) {
        let p = palette(ctx);
        egui::SidePanel::left("rail")
            .exact_width(96.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(p.surface_container).inner_margin(Margin::symmetric(8, 16)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // FAB
                    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(56.0), egui::Sense::click());
                    let fill = p.state_layer(p.primary_container, p.on_primary_container, &resp);
                    ui.painter().rect_filled(rect.translate(vec2(0.0, 2.0)), 16.0, Color32::from_black_alpha(40));
                    ui.painter().rect_filled(rect, 16.0, fill);
                    ui.painter().text(rect.center(), Align2::CENTER_CENTER, ICON_SCREENSHOT_REGION, FontId::proportional(26.0), p.on_primary_container);
                    if resp.on_hover_text("Capturer une zone").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.start_capture(CaptureKind::Region, ctx, false);
                    }
                    ui.add_space(20.0);
                    for (page, icon, label) in [(Page::Home, ICON_HOME, "Accueil"), (Page::Settings, ICON_SETTINGS, "Paramètres"), (Page::About, ICON_INFO, "À propos")] {
                        if theme::nav_item(ui, icon, label, self.page == page).clicked() {
                            self.page = page;
                        }
                        ui.add_space(4.0);
                    }
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        if theme::icon_button(ui, ICON_POWER_SETTINGS_NEW, false, "Quitter ScreenCapture").clicked() {
                            self.request_quit(ctx);
                        }
                        let dark = p.dark;
                        if theme::icon_button(ui, if dark { ICON_LIGHT_MODE } else { ICON_DARK_MODE }, false, "Basculer le thème").clicked() {
                            self.settings.theme = if dark { ThemeChoice::Light } else { ThemeChoice::Dark };
                            self.settings.save();
                        }
                    });
                });
            });
    }

    fn settings_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = palette(ctx);
        let mut changed = false;
        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            ui.set_max_width(820.0);
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Paramètres").font(theme::medium(28.0)).color(p.on_surface));
            ui.add_space(16.0);

            theme::card(ui, "Enregistrement", ICON_FOLDER, |ui| {
                theme::setting_row(ui, "Dossier des captures", &self.settings.save_dir.display().to_string(), |ui| {
                    if theme::button(ui, ButtonKind::Tonal, Some(ICON_FOLDER_OPEN), "Parcourir").clicked() {
                        if let Some(d) = rfd::FileDialog::new().set_directory(&self.settings.save_dir).pick_folder() {
                            self.settings.save_dir = d;
                            self.workspace.refresh();
                            changed = true;
                        }
                    }
                });
                theme::setting_row(ui, "Format d'image", "PNG sans perte ou JPEG compressé", |ui| {
                    if theme::filter_chip(ui, "JPEG", self.settings.image_format == ImageFormat::Jpg).clicked() {
                        self.settings.image_format = ImageFormat::Jpg;
                        changed = true;
                    }
                    if theme::filter_chip(ui, "PNG", self.settings.image_format == ImageFormat::Png).clicked() {
                        self.settings.image_format = ImageFormat::Png;
                        changed = true;
                    }
                });
                if self.settings.image_format == ImageFormat::Jpg {
                    theme::setting_row(ui, "Qualité JPEG", "", |ui| {
                        changed |= ui.add(egui::Slider::new(&mut self.settings.jpeg_quality, 40..=100)).changed();
                    });
                }
                theme::setting_row(ui, "Enregistrer automatiquement", "Chaque capture est enregistrée dans le dossier", |ui| {
                    changed |= theme::switch(ui, &mut self.settings.auto_save).changed();
                });
                theme::setting_row(ui, "Copier dans le presse-papiers", "Chaque capture est copiée immédiatement", |ui| {
                    changed |= theme::switch(ui, &mut self.settings.copy_to_clipboard).changed();
                });
            });
            ui.add_space(12.0);

            theme::card(ui, "Capture", ICON_SCREENSHOT_REGION, |ui| {
                theme::setting_row(ui, "Après la capture", "", |ui| {
                    for (v, label) in [(AfterCapture::Nothing, "Rien"), (AfterCapture::ShowWorkspace, "Miniature"), (AfterCapture::OpenEditor, "Éditeur")] {
                        if theme::filter_chip(ui, label, self.settings.after_capture == v).clicked() {
                            self.settings.after_capture = v;
                            changed = true;
                        }
                    }
                });
                theme::setting_row(ui, "Inclure le curseur", "Dessine le pointeur de la souris dans la capture", |ui| {
                    changed |= theme::switch(ui, &mut self.settings.capture_cursor).changed();
                });
                theme::setting_row(ui, "Délai de la capture différée", "", |ui| {
                    changed |= ui.add(egui::Slider::new(&mut self.settings.delay_seconds, 1..=30).suffix(" s")).changed();
                });
            });
            ui.add_space(12.0);

            theme::card(ui, "Raccourcis clavier globaux", ICON_KEYBOARD, |ui| {
                let examples = if cfg!(target_os = "macos") {
                    "Exemples : super+shift+Digit2 (⌘⇧2), ctrl+alt+KeyS, F13. Laisser vide pour désactiver."
                } else {
                    "Exemples : PrintScreen, ctrl+PrintScreen, ctrl+shift+KeyS, alt+F9. Laisser vide pour désactiver."
                };
                ui.label(egui::RichText::new(examples).size(12.5).color(p.on_surface_variant));
                ui.add_space(6.0);
                let status = self.hotkeys.status.clone();
                let ok = |k: CaptureKind| status.iter().find(|(c, _)| *c == k).map(|(_, o)| *o).unwrap_or(false);
                let rows: [(&str, CaptureKind); 5] = [
                    ("Capturer une zone", CaptureKind::Region),
                    ("Plein écran", CaptureKind::Fullscreen),
                    ("Fenêtre active", CaptureKind::Window),
                    ("Dernière zone", CaptureKind::LastRegion),
                    ("Vidéo (démarrer / arrêter)", CaptureKind::Video),
                ];
                for (label, kind) in rows {
                    let field = match kind {
                        CaptureKind::Region => &mut self.hotkey_edit.region,
                        CaptureKind::Fullscreen => &mut self.hotkey_edit.fullscreen,
                        CaptureKind::Window => &mut self.hotkey_edit.window,
                        CaptureKind::LastRegion => &mut self.hotkey_edit.last_region,
                        _ => &mut self.hotkey_edit.video,
                    };
                    let desc = if field.trim().is_empty() {
                        "Désactivé".to_string()
                    } else if ok(kind) {
                        format!("Actif : {}", pretty_hotkey(field))
                    } else {
                        "Non enregistré (déjà utilisé par une autre application, ou syntaxe invalide)".to_string()
                    };
                    theme::setting_row(ui, label, &desc, |ui| {
                        ui.label(egui::RichText::new(if ok(kind) { ICON_CHECK_CIRCLE } else { ICON_ERROR }).size(20.0).color(if ok(kind) { p.primary } else { p.error }));
                        theme::text_field(ui, field, 190.0, "");
                    });
                }
                #[cfg(windows)]
                theme::setting_row(
                    ui,
                    "Remplacer Win + Maj + S",
                    "Capture de zone au lieu de l'Outil Capture de Windows (utile si la touche capture du clavier envoie ce raccourci)",
                    |ui| {
                        if theme::switch(ui, &mut self.hotkey_edit.override_snip).changed() {
                            self.settings.hotkeys.override_snip = self.hotkey_edit.override_snip;
                            self.hotkeys.apply(&self.settings.hotkeys);
                            changed = true;
                        }
                    },
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if theme::button(ui, ButtonKind::Filled, Some(ICON_CHECK), "Appliquer les raccourcis").clicked() {
                        self.settings.hotkeys = self.hotkey_edit.clone();
                        self.hotkeys.apply(&self.settings.hotkeys);
                        changed = true;
                    }
                    if theme::button(ui, ButtonKind::Text, Some(ICON_RESTART_ALT), "Valeurs par défaut").clicked() {
                        self.hotkey_edit = crate::settings::Hotkeys::default();
                    }
                });
                let tip = if cfg!(target_os = "macos") {
                    "Astuce : ⌘⇧3, ⌘⇧4 et ⌘⇧5 sont réservés par macOS (Réglages Système > Clavier > Raccourcis clavier > Captures d'écran)."
                } else {
                    "Astuce : si Impr. écran ouvre l'Outil Capture de Windows, désactivez « Utiliser la touche Impr. écran pour ouvrir la capture d'écran » dans Paramètres Windows > Accessibilité > Clavier."
                };
                ui.label(egui::RichText::new(tip).size(12.0).color(p.on_surface_variant));
            });
            ui.add_space(12.0);

            theme::card(ui, "Vidéo", ICON_VIDEOCAM, |ui| {
                let found = recorder::find_ffmpeg(&self.settings);
                let desc = match &found {
                    Some(f) => format!("Utilisé : {}", f.display()),
                    None => format!("Introuvable — installez FFmpeg ({}) ou indiquez son emplacement", recorder::FFMPEG_HINT),
                };
                theme::setting_row(ui, "FFmpeg", &desc, |ui| {
                    if theme::button(ui, ButtonKind::Tonal, Some(ICON_FOLDER_OPEN), "Choisir").clicked() {
                        if let Some(f) = {
                            let d = rfd::FileDialog::new();
                            if cfg!(windows) { d.add_filter("ffmpeg", &["exe"]) } else { d }
                        }
                        .pick_file() {
                            self.settings.ffmpeg_path = f.display().to_string();
                            changed = true;
                        }
                    }
                });
                theme::setting_row(ui, "Format", "", |ui| {
                    for (v, label) in [(VideoFormat::Gif, "GIF"), (VideoFormat::Webm, "WebM"), (VideoFormat::Mp4, "MP4")] {
                        if theme::filter_chip(ui, label, self.settings.video_format == v).clicked() {
                            self.settings.video_format = v;
                            changed = true;
                        }
                    }
                });
                theme::setting_row(ui, "Images par seconde", "", |ui| {
                    changed |= ui.add(egui::Slider::new(&mut self.settings.video_fps, 5..=60)).changed();
                });
                theme::setting_row(ui, "Afficher le cadre d'enregistrement", "", |ui| {
                    changed |= theme::switch(ui, &mut self.settings.video_show_region).changed();
                });
                theme::setting_row(ui, "Enregistrer le curseur", "", |ui| {
                    changed |= theme::switch(ui, &mut self.settings.video_cursor).changed();
                });
                let mic = self.settings.audio_device.clone().unwrap_or_else(|| "Aucun".into());
                // Liste rafraîchie automatiquement tant que la page est affichée (micro branché à chaud).
                let stale = self.mic_refresh.is_none_or(|t| t.elapsed() > Duration::from_secs(5));
                let desc = if self.mic_refresh.is_some() && self.audio_devices.is_empty() {
                    "Aucun micro branché ou activé dans Windows • non disponible en GIF"
                } else {
                    "Non disponible pour le format GIF"
                };
                theme::setting_row(ui, "Microphone", desc, |ui| {
                    let force = theme::icon_button(ui, ICON_REFRESH, false, "Actualiser la liste").clicked();
                    if force || stale {
                        self.mic_refresh = Some(Instant::now());
                        let tx = self.tx.clone();
                        let c = ctx.clone();
                        std::thread::spawn(move || {
                            let _ = tx.send(AppEvent::AudioDevices(recorder::list_microphones()));
                            c.request_repaint();
                        });
                        ctx.request_repaint_after(Duration::from_secs(5));
                    }
                    egui::ComboBox::from_id_salt("mic").selected_text(mic).width(220.0).show_ui(ui, |ui| {
                        if ui.selectable_label(self.settings.audio_device.is_none(), "Aucun").clicked() {
                            self.settings.audio_device = None;
                            changed = true;
                        }
                        for d in &self.audio_devices {
                            if ui.selectable_label(self.settings.audio_device.as_deref() == Some(d), d).clicked() {
                                self.settings.audio_device = Some(d.clone());
                                changed = true;
                            }
                        }
                    });
                });
            });
            ui.add_space(12.0);

            theme::card(ui, "Diagnostic", ICON_BUG_REPORT, |ui| {
                let path = crate::diag::log_path();
                theme::setting_row(ui, "Journal de diagnostic", &path.display().to_string(), |ui| {
                    if theme::button(ui, ButtonKind::Tonal, Some(ICON_FOLDER_OPEN), "Ouvrir le dossier").clicked() {
                        platform::open_folder(&crate::diag::log_dir());
                    }
                });
                ui.label(
                    egui::RichText::new("En cas de blocage, relancer ScreenCapture depuis le menu Démarrer remplace l'instance bloquée ; le journal indique alors l'étape en cause.")
                        .size(12.0)
                        .color(p.on_surface_variant),
                );
            });
            ui.add_space(12.0);

            theme::card(ui, "Couleurs de l'éditeur", ICON_COLOR_LENS, |ui| {
                changed |= self.palette_editor(ui);
            });
            ui.add_space(12.0);

            theme::card(ui, "Apparence et démarrage", ICON_PALETTE, |ui| {
                theme::setting_row(ui, "Thème", "", |ui| {
                    for (v, label) in [(ThemeChoice::Dark, "Sombre"), (ThemeChoice::Light, "Clair"), (ThemeChoice::System, "Système")] {
                        if theme::filter_chip(ui, label, self.settings.theme == v).clicked() {
                            self.settings.theme = v;
                            changed = true;
                        }
                    }
                });
                theme::setting_row(ui, "Couleur d'accent", "", |ui| {
                    for a in [Accent::Red, Accent::Orange, Accent::Green, Accent::Blue, Accent::Violet] {
                        let c = theme::Palette::new(false, a).primary;
                        if theme::swatch(ui, c, self.settings.accent == a).clicked() {
                            self.settings.accent = a;
                            changed = true;
                        }
                    }
                });
                theme::setting_row(ui, "Fermer dans la zone de notification", "La croix masque la fenêtre sans quitter", |ui| {
                    changed |= theme::switch(ui, &mut self.settings.close_to_tray).changed();
                });
                theme::setting_row(ui, "Démarrer réduit", "Lancer directement dans la zone de notification", |ui| {
                    changed |= theme::switch(ui, &mut self.settings.start_in_tray).changed();
                });
            });
            ui.add_space(24.0);
        });
        if changed {
            self.settings.save();
        }
    }

    /// Nuancier des styles : modifier, ajouter, retirer ou réordonner les couleurs.
    fn palette_editor(&mut self, ui: &mut egui::Ui) -> bool {
        const MAX: usize = 32;
        let mut changed = false;
        let pal = &mut self.settings.palette;
        if pal.is_empty() {
            *pal = crate::editor::model::COLORS.to_vec();
        }
        if self.palette_sel.is_some_and(|i| i >= pal.len()) {
            self.palette_sel = None;
        }
        theme::setting_row(
            ui,
            "Nuancier des styles",
            "Clic : modifier  •  clic droit : retirer  •  + : ajouter",
            |ui| {
                if theme::button(ui, theme::ButtonKind::Text, Some(ICON_RESTART_ALT), "Couleurs d'origine").clicked() {
                    *pal = crate::editor::model::COLORS.to_vec();
                    self.palette_sel = None;
                    changed = true;
                }
            },
        );
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            let mut remove = None;
            for (i, c) in pal.iter().enumerate() {
                let c32 = egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
                let hint = if c[3] == 0 { "Transparent".to_string() } else { format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]) };
                let r = theme::swatch(ui, c32, self.palette_sel == Some(i)).on_hover_text(hint);
                if r.clicked() {
                    self.palette_sel = if self.palette_sel == Some(i) { None } else { Some(i) };
                }
                if r.secondary_clicked() && pal.len() > 1 {
                    remove = Some(i);
                }
            }
            if let Some(i) = remove {
                pal.remove(i);
                self.palette_sel = None;
                changed = true;
            }
            if pal.len() < MAX && theme::icon_button(ui, ICON_ADD, false, "Ajouter une couleur").clicked() {
                let base = self.palette_sel.and_then(|i| pal.get(i).copied()).unwrap_or([128, 128, 128, 255]);
                pal.push(if base[3] == 0 { [128, 128, 128, 255] } else { base });
                self.palette_sel = Some(pal.len() - 1);
                changed = true;
            }
        });

        if let Some(i) = self.palette_sel {
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                let c = pal[i];
                let mut c32 = egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
                if egui::widgets::color_picker::color_picker_color32(ui, &mut c32, egui::widgets::color_picker::Alpha::OnlyBlend) {
                    pal[i] = c32.to_srgba_unmultiplied();
                    changed = true;
                }
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.label(format!("Couleur {} sur {}", i + 1, pal.len()));
                    let hex_of = |c: [u8; 4]| format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]);
                    let editing = ui.memory(|m| m.focused()).is_some_and(|f| f == egui::Id::new("palette_hex"));
                    if self.palette_hex.0 != i || !editing {
                        self.palette_hex = (i, hex_of(c));
                    }
                    let hex = &mut self.palette_hex.1;
                    let r = ui.add(egui::TextEdit::singleline(hex).id(egui::Id::new("palette_hex")).desired_width(90.0).hint_text("#RRVVBB"));
                    if r.changed() {
                        let h = hex.trim().trim_start_matches('#');
                        if h.len() == 6 {
                            if let Ok(v) = u32::from_str_radix(h, 16) {
                                pal[i] = [(v >> 16) as u8, (v >> 8) as u8, v as u8, 255];
                                changed = true;
                            }
                        }
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if i > 0 && theme::icon_button(ui, ICON_CHEVRON_LEFT, false, "Déplacer à gauche").clicked() {
                            pal.swap(i, i - 1);
                            self.palette_sel = Some(i - 1);
                            changed = true;
                        }
                        if i + 1 < pal.len() && theme::icon_button(ui, ICON_CHEVRON_RIGHT, false, "Déplacer à droite").clicked() {
                            pal.swap(i, i + 1);
                            self.palette_sel = Some(i + 1);
                            changed = true;
                        }
                        if pal.len() > 1 && theme::icon_button(ui, ICON_DELETE, false, "Retirer cette couleur").clicked() {
                            pal.remove(i);
                            self.palette_sel = None;
                            changed = true;
                        }
                    });
                });
            });
        }
        changed
    }

    fn about_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = palette(ctx);
        ui.add_space(8.0);
        ui.label(egui::RichText::new("ScreenCapture").font(theme::medium(28.0)).color(p.on_surface));
        ui.label(egui::RichText::new(format!("Version {}  •  Rust + egui  •  Material Design 3", env!("CARGO_PKG_VERSION"))).color(p.on_surface_variant));
        ui.add_space(16.0);
        theme::card(ui, "Fonctionnalités", ICON_STARS, |ui| {
            for (icon, text) in [
                (ICON_SCREENSHOT_REGION, "Capture de zone avec loupe, réticule et pipette, détection automatique des fenêtres"),
                (ICON_SCREENSHOT_MONITOR, "Plein écran multi-moniteurs, fenêtre active, dernière zone, capture différée"),
                (ICON_VIDEOCAM, "Enregistrement vidéo MP4 / WebM / GIF avec micro (via FFmpeg)"),
                (ICON_EDIT, "Éditeur : flèches, formes, surligneur, flou, texte, bulles, numéros, loupe, tampons, rognage, rotation"),
                (ICON_DOCUMENT_SCANNER, "Extraction de texte (OCR Windows)"),
                (ICON_PHOTO_LIBRARY, "Espace de travail avec historique, recherche et filtres"),
                (ICON_KEYBOARD, "Raccourcis clavier globaux configurables, icône de notification"),
            ] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(icon).size(20.0).color(p.primary));
                    ui.label(text);
                });
            }
        });
        ui.add_space(12.0);
        ui.label(egui::RichText::new("Inspiré de l'ergonomie de Screenpresso. Aucune partie de son code n'est réutilisée.").size(12.5).color(p.on_surface_variant));
    }

    fn workspace_actions(&mut self, ctx: &egui::Context, actions: Vec<WsAction>) {
        for a in actions {
            match a {
                WsAction::Capture(k) => self.start_capture(k, ctx, false),
                WsAction::Edit(path) => match image::open(&path) {
                    Ok(img) => self.open_editor(img.to_rgba8(), Some(path)),
                    Err(e) => self.toast(format!("Impossible d'ouvrir l'image : {e}")),
                },
                WsAction::Copy(path) => {
                    let r = image::open(&path).map_err(anyhow::Error::from).and_then(|i| copy_image(&i.to_rgba8()));
                    match r {
                        Ok(()) => self.toast("Copié dans le presse-papiers"),
                        Err(e) => self.toast(format!("Erreur : {e}")),
                    }
                }
                WsAction::Delete(path) => match platform::move_to_recycle_bin(&path) {
                    Ok(()) => {
                        self.workspace.forget(&path);
                        self.toast("Envoyé dans la Corbeille");
                    }
                    Err(e) => self.toast(format!("Erreur : {e}")),
                },
                WsAction::Open(path) => platform::open_with_default(&path),
                WsAction::Reveal(path) => platform::reveal_in_explorer(&path),
                WsAction::OpenFolder => platform::open_folder(&self.settings.save_dir),
                WsAction::Paste => self.paste_clipboard(),
                WsAction::OpenFile => {
                    if let Some(f) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "bmp", "gif"]).pick_file() {
                        match image::open(&f) {
                            Ok(img) => {
                                let inside = f.parent() == Some(self.settings.save_dir.as_path());
                                self.open_editor(img.to_rgba8(), inside.then_some(f));
                            }
                            Err(e) => self.toast(format!("Impossible d'ouvrir l'image : {e}")),
                        }
                    }
                }
            }
        }
    }

    /// Colle l'image du presse-papiers (ou les fichiers image copiés) dans le dossier des captures.
    fn paste_clipboard(&mut self) {
        crate::diag::step("collage du presse-papiers");
        let Ok(mut cb) = arboard::Clipboard::new() else {
            self.toast("Presse-papiers indisponible");
            return;
        };
        if let Ok(img) = cb.get_image() {
            let Some(rgba) = RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.into_owned()) else { return };
            let r = std::fs::create_dir_all(&self.settings.save_dir).map_err(anyhow::Error::from).and_then(|_| {
                let path = self.unique_path("Collage", self.settings.image_format.ext());
                save_image(&rgba, &path, self.settings.jpeg_quality).map(|_| path)
            });
            match r {
                Ok(path) => {
                    self.workspace.refresh();
                    self.page = Page::Home;
                    self.toast(format!("Image collée : {}", path.file_name().unwrap_or_default().to_string_lossy()));
                }
                Err(e) => self.toast(format!("Enregistrement impossible : {e}")),
            }
            return;
        }
        let files = cb.get().file_list().unwrap_or_default();
        if files.is_empty() {
            self.toast("Le presse-papiers ne contient pas d'image");
        } else {
            self.import_files(&files);
        }
    }

    /// Copie des fichiers image (collés ou déposés) dans le dossier des captures.
    fn import_files(&mut self, files: &[PathBuf]) {
        let images: Vec<&PathBuf> = files
            .iter()
            .filter(|f| {
                let ext = f.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
                f.is_file() && crate::workspace::IMAGE_EXT.contains(&ext.as_str())
            })
            .collect();
        if images.is_empty() {
            self.toast("Aucune image parmi les fichiers");
            return;
        }
        let _ = std::fs::create_dir_all(&self.settings.save_dir);
        let mut copied = 0;
        for f in &images {
            if f.parent() == Some(self.settings.save_dir.as_path()) {
                continue;
            }
            let mut dest = self.settings.save_dir.join(f.file_name().unwrap_or_default());
            let stem = f.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let ext = f.extension().unwrap_or_default().to_string_lossy().to_string();
            let mut n = 2;
            while dest.exists() {
                dest = self.settings.save_dir.join(format!("{stem}_{n}.{ext}"));
                n += 1;
            }
            match std::fs::copy(f, &dest) {
                Ok(_) => copied += 1,
                Err(e) => crate::diag::log(&format!("copie impossible de {}: {e}", f.display())),
            }
        }
        self.workspace.refresh();
        self.page = Page::Home;
        self.toast(match copied {
            0 => "Ces images sont déjà dans le dossier des captures".to_string(),
            1 => "1 image ajoutée au dossier des captures".to_string(),
            n => format!("{n} images ajoutées au dossier des captures"),
        });
    }

    fn recording_bar(&mut self, ctx: &egui::Context) {
        let p = palette(ctx);
        let elapsed = self.recorder.as_ref().map(|r| r.started.elapsed()).unwrap_or_default();
        let mut stop = false;
        let mut cancel = false;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.surface_high).corner_radius(CornerRadius::same(0)).inner_margin(Margin::symmetric(14, 10)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let t = ctx.input(|i| i.time);
                    let a = (0.55 + 0.45 * (t * 3.0).sin()) as f32;
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 7.0, Color32::from_rgb(229, 57, 53).gamma_multiply(a));
                    let s = elapsed.as_secs();
                    ui.label(egui::RichText::new(format!("{:02}:{:02}", s / 60, s % 60)).font(theme::medium(18.0)).color(p.on_surface));
                    if let Some(r) = &self.recorder {
                        ui.label(egui::RichText::new(format!("{}×{}", r.region.w, r.region.h)).size(12.0).color(p.on_surface_variant));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        stop = theme::button(ui, ButtonKind::Danger, Some(ICON_STOP_CIRCLE), "Arrêter").clicked();
                        cancel = theme::icon_button(ui, ICON_DELETE, false, "Annuler l'enregistrement").clicked();
                    });
                });
            });
        ctx.request_repaint_after(Duration::from_millis(100));
        if stop {
            self.stop_recording(ctx);
        } else if cancel {
            self.cancel_recording(ctx);
        }
    }

    fn snackbar_ui(&mut self, ctx: &egui::Context) {
        let Some((msg, at)) = self.snackbar.clone() else { return };
        if at.elapsed() > Duration::from_secs(4) {
            self.snackbar = None;
            return;
        }
        let p = palette(ctx);
        egui::Area::new(egui::Id::new("snackbar"))
            .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -24.0))
            .order(egui::Order::Tooltip)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.inverse_surface)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(16, 12))
                    .shadow(egui::epaint::Shadow { offset: [0, 3], blur: 10, spread: 0, color: Color32::from_black_alpha(60) })
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(msg).color(p.inverse_on_surface));
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(250));
    }

    fn ocr_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut text) = self.ocr_text.clone() else { return };
        let mut open = true;
        let mut close = false;
        egui::Window::new("Texte extrait")
            .collapsible(false)
            .default_width(520.0)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Le texte a été copié dans le presse-papiers.").color(palette(ctx).on_surface_variant));
                ui.add_space(8.0);
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut text).desired_width(f32::INFINITY).desired_rows(10));
                });
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::button(ui, ButtonKind::Filled, Some(ICON_CONTENT_COPY), "Copier").clicked() {
                        let _ = arboard::Clipboard::new().and_then(|mut c| c.set_text(text.clone()));
                        close = true;
                    }
                    if theme::button(ui, ButtonKind::Text, None, "Fermer").clicked() {
                        close = true;
                    }
                });
            });
        self.ocr_text = if !open || close { None } else { Some(text) };
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::diag::heartbeat();
        self.apply_theme_if_needed(ctx);
        if std::mem::take(&mut self.park_on_start) {
            self.hide_main();
        }
        self.handle_events(ctx);
        // Écran débranché alors que la fenêtre y était affichée : la ramener (vérifié toutes les 2 s).
        if self.visible && self.saved.is_none() && self.screen_check.elapsed() > Duration::from_secs(2) {
            self.screen_check = Instant::now();
            crate::diag::step("vérification des écrans");
            self.win.rescue();
            ctx.request_repaint_after(Duration::from_secs(2));
        }

        // Fermeture : masquer dans la zone de notification.
        if ctx.input(|i| i.viewport().close_requested()) && !self.quit {
            if self.settings.close_to_tray {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                if self.mode == Mode::Editor {
                    self.close_editor(true);
                }
                self.hide_main();
            } else {
                self.request_quit(ctx);
            }
        }

        crate::diag::step(match self.mode {
            Mode::Overlay => "interface : sélection",
            Mode::Recording => "interface : enregistrement",
            Mode::Popup => "interface : miniature",
            Mode::Editor => "interface : éditeur",
            Mode::Workspace => "interface : espace de travail",
        });
        match self.mode {
            Mode::Overlay => {
                if let Some(ov) = &self.overlay {
                    self.win.ensure_rect(ov.origin);
                }
                let result = self.overlay.as_mut().map(|o| o.ui(ctx)).unwrap_or(OverlayResult::Cancel);
                match result {
                    OverlayResult::Pending => {}
                    OverlayResult::CopyColor(hex) => {
                        let _ = arboard::Clipboard::new().and_then(|mut c| c.set_text(hex.clone()));
                        self.overlay = None;
                        self.restore_after_cancel(ctx);
                        self.toast(format!("Couleur {hex} copiée"));
                    }
                    OverlayResult::Cancel => {
                        self.overlay = None;
                        self.restore_after_cancel(ctx);
                    }
                    OverlayResult::Selected(r) => {
                        let ov = self.overlay.take().expect("overlay");
                        match ov.purpose {
                            Purpose::Image => {
                                self.settings.last_region = Some(r);
                                self.settings.save();
                                self.hide_main();
                                match capture::crop_virtual(&ov.image, ov.origin, r) {
                                    Some(img) => self.finish_capture(ctx, img),
                                    None => self.restore_after_cancel(ctx),
                                }
                            }
                            Purpose::Video => {
                                self.hide_main();
                                self.start_recording(ctx, r);
                            }
                        }
                    }
                }
            }
            Mode::Recording => self.recording_bar(ctx),
            Mode::Popup => self.capture_popup_ui(ctx),
            Mode::Editor => {
                let mut settings = std::mem::take(&mut self.settings);
                let actions = self.editor.as_mut().map(|e| e.ui(ctx, &mut settings)).unwrap_or_default();
                self.settings = settings;
                if let Some(e) = &self.editor {
                    self.set_title(ctx, format!("{} — ScreenCapture", e.title()));
                }
                self.editor_actions(ctx, actions);
            }
            Mode::Workspace => {
                self.set_title(ctx, "ScreenCapture".into());
                if !ctx.wants_keyboard_input() && ctx.input(|i| i.modifiers.command && i.key_pressed(Key::V)) {
                    self.paste_clipboard();
                }
                // Fichiers glissés depuis l'Explorateur / le Finder.
                let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
                if !dropped.is_empty() {
                    self.import_files(&dropped);
                }
                self.nav_rail(ctx);
                let p = palette(ctx);
                let mut ws_actions = Vec::new();
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(p.surface).inner_margin(Margin { left: 28, right: 20, top: 20, bottom: 0 }))
                    .show(ctx, |ui| match self.page {
                        Page::Home => {
                            let status = self.hotkeys.status.clone();
                            ws_actions = self.workspace.ui(ui, &self.settings, &status);
                        }
                        Page::Settings => self.settings_page(ui, ctx),
                        Page::About => self.about_page(ui, ctx),
                    });
                self.workspace_actions(ctx, ws_actions);
            }
        }

        self.ocr_dialog(ctx);
        self.snackbar_ui(ctx);
        if self.finishing_video {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
        // Après `update`, eframe dessine et présente l'image (pilote graphique), puis attend les évènements.
        crate::diag::step("après update : rendu graphique ou attente d'évènements");
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }
}

fn build_tray(ids: &mut HashMap<String, &'static str>, settings: &Settings) -> anyhow::Result<TrayIcon> {
    let menu = Menu::new();
    let hk = &settings.hotkeys;
    let entries: [(&'static str, String); 6] = [
        ("region", format!("Capturer une zone\t{}", pretty_hotkey(&hk.region))),
        ("full", format!("Plein écran\t{}", pretty_hotkey(&hk.fullscreen))),
        ("window", format!("Fenêtre\t{}", pretty_hotkey(&hk.window))),
        ("last", format!("Dernière zone\t{}", pretty_hotkey(&hk.last_region))),
        ("delayed", format!("Capture différée ({} s)", settings.delay_seconds)),
        ("video", format!("Enregistrer une vidéo\t{}", pretty_hotkey(&hk.video))),
    ];
    for (key, label) in entries {
        let item = MenuItem::new(label, true, None);
        ids.insert(item.id().0.clone(), key);
        menu.append(&item)?;
    }
    menu.append(&PredefinedMenuItem::separator())?;
    for (key, label) in [("open", "Ouvrir l'espace de travail"), ("folder", "Ouvrir le dossier des captures"), ("settings", "Paramètres")] {
        let item = MenuItem::new(label, true, None);
        ids.insert(item.id().0.clone(), key);
        menu.append(&item)?;
    }
    menu.append(&PredefinedMenuItem::separator())?;
    let quit = MenuItem::new("Quitter", true, None);
    ids.insert(quit.id().0.clone(), "quit");
    menu.append(&quit)?;

    let rgba = crate::icon::app_icon(32);
    let icon = tray_icon::Icon::from_rgba(rgba, 32, 32)?;
    Ok(TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        // Sur Mac, un clic sur l'icône de la barre des menus ouvre le menu, comme partout ailleurs.
        .with_menu_on_left_click(cfg!(target_os = "macos"))
        .with_tooltip(format!("ScreenCapture — {} pour capturer", pretty_hotkey(&settings.hotkeys.region)))
        .with_icon(icon)
        .build()?)
}

pub fn copy_image(img: &RgbaImage) -> anyhow::Result<()> {
    crate::diag::step("copie dans le presse-papiers");
    let mut cb = arboard::Clipboard::new()?;
    cb.set_image(arboard::ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: std::borrow::Cow::Borrowed(img.as_raw()),
    })?;
    Ok(())
}

pub fn save_image(img: &RgbaImage, path: &Path, jpeg_quality: u8) -> anyhow::Result<()> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if ext == "jpg" || ext == "jpeg" {
        // Le JPEG n'a pas de transparence : composition sur fond blanc.
        let mut rgb = image::RgbImage::new(img.width(), img.height());
        for (x, y, p) in img.enumerate_pixels() {
            let a = p[3] as u32;
            let mix = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
            rgb.put_pixel(x, y, image::Rgb([mix(p[0]), mix(p[1]), mix(p[2])]));
        }
        let file = std::io::BufWriter::new(std::fs::File::create(path)?);
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(file, jpeg_quality);
        enc.encode_image(&rgb)?;
    } else {
        img.save(path)?;
    }
    Ok(())
}

/// Thread relais : transmet les évènements et réveille l'interface, pour les appelants qui
/// ne doivent jamais attendre le verrou du contexte egui (hook clavier).
fn relay(tx: Sender<AppEvent>, ctx: egui::Context) -> Sender<AppEvent> {
    let (relay_tx, relay_rx) = channel::<AppEvent>();
    let _ = std::thread::Builder::new().name("relais".into()).spawn(move || {
        for ev in relay_rx {
            if tx.send(ev).is_err() {
                break;
            }
            ctx.request_repaint();
        }
    });
    relay_tx
}
