//! Intégration macOS : géométrie de la fenêtre principale (AppKit), instance unique, corbeille, OCR (Vision).

use std::path::Path;

use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSStatusWindowLevel, NSView, NSWindow, NSWindowCollectionBehavior, NSWindowSharingType};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};

use super::PrintScreenHandler;
use crate::capture::imp::{rect_from_cg, rect_to_cg, scale};
use crate::capture::Rect;

/// Fenêtre native identifiée par le pointeur de sa NSView : la fenêtre principale (fournie par
/// winit) ou une fenêtre flottante (sélection, miniature, barre d'enregistrement).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window(pub isize);

/// Hauteur de l'écran principal en points : les coordonnées Cocoa partent du bas de cet écran.
fn primary_height() -> f64 {
    core_graphics::display::CGDisplay::main().bounds().size.height
}

fn from_cocoa(f: NSRect) -> Rect {
    let h = primary_height();
    let cg = core_graphics::display::CGRect::new(
        &core_graphics::display::CGPoint::new(f.origin.x, h - f.origin.y - f.size.height),
        &core_graphics::display::CGSize::new(f.size.width, f.size.height),
    );
    rect_from_cg(cg, scale())
}

fn to_cocoa(r: Rect) -> NSRect {
    let cg = rect_to_cg(r, scale());
    let h = primary_height();
    NSRect::new(NSPoint::new(cg.origin.x, h - cg.origin.y - cg.size.height), NSSize::new(cg.size.width, cg.size.height))
}

impl Window {
    /// La fenêtre AppKit, uniquement depuis le thread principal (exigé par AppKit).
    fn ns_window(self) -> Option<(Retained<NSWindow>, MainThreadMarker)> {
        let mtm = MainThreadMarker::new()?;
        if self.0 == 0 {
            return None;
        }
        let view = unsafe { &*(self.0 as *const NSView) };
        Some((view.window()?, mtm))
    }

    pub fn rect(self) -> Option<Rect> {
        let (w, _) = self.ns_window()?;
        Some(from_cocoa(w.frame()))
    }

    /// Fenêtre de l'application portant exactement ce titre (fenêtre flottante egui).
    pub fn find(title: &str) -> Option<Window> {
        let mtm = MainThreadMarker::new()?;
        let app = NSApplication::sharedApplication(mtm);
        let windows = app.windows();
        windows.iter().find_map(|w| {
            (w.title().to_string() == title).then(|| w.contentView().map(|v| Window(Retained::as_ptr(&v) as isize)))?
        })
    }

    fn set_visible(w: &NSWindow, visible: bool) {
        unsafe { w.setAlphaValue(if visible { 1.0 } else { 0.0 }) };
        w.setIgnoresMouseEvents(!visible);
    }

    /// Masque la fenêtre (transparente et inerte) en la laissant dessiner.
    pub fn cloak(self, cloaked: bool) {
        if let Some((w, _)) = self.ns_window() {
            Self::set_visible(&w, !cloaked);
        }
    }

    pub fn is_minimized(self) -> bool {
        self.ns_window().is_some_and(|(w, _)| w.isMiniaturized())
    }

    /// Fenêtre principale : la rendre invisible sans la déplacer ni la redimensionner.
    pub fn hide(self) {
        let Some((w, mtm)) = self.ns_window() else { return };
        Self::set_visible(&w, false);
        w.setSharingType(NSWindowSharingType::NSWindowSharingNone);
        unsafe {
            w.orderBack(None);
            // Rendre le clavier à l'application précédente.
            NSApplication::sharedApplication(mtm).deactivate();
        }
    }

    /// Fenêtre principale : la réafficher telle qu'elle était, au premier plan.
    pub fn show(self) {
        let Some((w, mtm)) = self.ns_window() else { return };
        if w.isMiniaturized() {
            unsafe { w.deminiaturize(None) };
        }
        self.rescue();
        w.setSharingType(NSWindowSharingType::NSWindowSharingReadOnly);
        Self::set_visible(&w, true);
        w.makeKeyAndOrderFront(None);
        bring_app_to_front(mtm);
    }

    /// Fenêtre flottante : placée exactement sur `r`, au-dessus de tout (barre des menus comprise).
    pub fn place_floating(self, r: Rect, activate: bool, exclude_from_capture: bool) {
        let Some((w, mtm)) = self.ns_window() else { return };
        w.setLevel(NSStatusWindowLevel);
        w.setSharingType(if exclude_from_capture {
            NSWindowSharingType::NSWindowSharingNone
        } else {
            NSWindowSharingType::NSWindowSharingReadOnly
        });
        unsafe {
            w.setCollectionBehavior(NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::FullScreenAuxiliary)
        };
        w.setFrame_display(to_cocoa(r), true);
        if activate {
            bring_app_to_front(mtm);
            w.makeKeyAndOrderFront(None);
        } else {
            unsafe { w.orderFrontRegardless() };
        }
    }

    /// Les fenêtres macOS ont déjà des coins arrondis.
    pub fn round_corners(self, _round: bool) {}

    /// Replace la fenêtre sur un écran branché si elle est restée sur un écran débranché.
    pub fn rescue(self) {
        let Some((w, _)) = self.ns_window() else { return };
        if w.isMiniaturized() {
            return;
        }
        let r = from_cocoa(w.frame());
        let fit = crate::capture::fit_on_screen(r);
        if fit != r {
            w.setFrame_display(to_cocoa(fit), true);
        }
    }

    /// Déplacement en cours par l'utilisateur : AppKit gère lui-même les changements d'échelle.
    pub fn is_moving(self) -> bool {
        false
    }

    /// État lisible pour le journal.
    pub fn describe(self) -> String {
        format!("{:?} réduite={}", self.rect(), self.is_minimized())
    }
}

/// AppKit gère lui-même le passage d'un écran à l'autre.
pub fn fix_dpi_changes(_win: Window) {}

pub fn set_wanted_size(_points: (f32, f32)) {}

/// Unités de `Rect` par point pour une fenêtre placée en `(x, y)` : l'échelle commune sur Mac.
pub fn scale_at(_x: i32, _y: i32) -> f32 {
    scale() as f32
}

/// Vrai si notre fenêtre est au premier plan.
pub fn is_foreground(win: Window) -> bool {
    win.ns_window()
        .is_some_and(|(w, mtm)| unsafe { NSApplication::sharedApplication(mtm).isActive() } && w.isKeyWindow())
}

/// Unités de `Rect` par point egui.
pub fn units_per_point(ctx: &eframe::egui::Context) -> f32 {
    scale() as f32 * ctx.zoom_factor()
}

/// Identifiant natif de la fenêtre principale (pointeur NSView).
pub fn native_handle(h: raw_window_handle::RawWindowHandle) -> isize {
    match h {
        raw_window_handle::RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as isize,
        _ => 0,
    }
}

/// Pas de touche Impr. écran sur Mac : les raccourcis passent tous par `global-hotkey`.
pub fn install_printscreen_hook(_handler: PrintScreenHandler) -> bool {
    false
}

/// Renvoie `false` si une autre instance tourne déjà (verrou sur un fichier temporaire).
pub fn acquire_single_instance(_activate_existing: bool) -> bool {
    static LOCK: std::sync::OnceLock<std::fs::File> = std::sync::OnceLock::new();
    let path = std::env::temp_dir().join("ScreenCapture.Rust.SingleInstance.lock");
    let Ok(file) = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(path) else {
        return true;
    };
    if file.try_lock().is_err() {
        return false;
    }
    // Le verrou est conservé pendant toute la vie du processus.
    let _ = LOCK.set(file);
    true
}

/// Sur Mac, relancer l'application la réactive déjà (Dock / Finder).
pub fn activation_listener(_f: impl Fn(Option<String>) + Send + 'static) {}

/// Pilotage par `--cmd` : non pris en charge sur Mac.
pub fn send_command(_cmd: &str) -> bool {
    false
}

/// Envoie un fichier dans la Corbeille (récupérable).
pub fn move_to_recycle_bin(path: &Path) -> anyhow::Result<()> {
    use objc2_foundation::{NSFileManager, NSString, NSURL};
    let url = unsafe { NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy())) };
    unsafe { NSFileManager::defaultManager().trashItemAtURL_resultingItemURL_error(&url, None) }
        .map_err(|e| anyhow::anyhow!("{}", crate::i18n::trf("suppression impossible : {0}", &[&e.localizedDescription()])))
}

/// Ouvre un fichier avec l'application par défaut.
pub fn open_with_default(path: &Path) {
    let _ = std::process::Command::new("open").arg(path).spawn();
}

/// Affiche le fichier sélectionné dans le Finder.
pub fn reveal_in_explorer(path: &Path) {
    let _ = std::process::Command::new("open").arg("-R").arg(path).spawn();
}

pub fn open_folder(path: &Path) {
    let _ = std::fs::create_dir_all(path);
    let _ = std::process::Command::new("open").arg(path).spawn();
}

pub fn hidden_command(program: &str) -> std::process::Command {
    std::process::Command::new(program)
}

/// Reconnaissance de texte via le framework Vision (français et anglais).
pub fn ocr(img: &image::RgbaImage) -> anyhow::Result<String> {
    use objc2::ClassType;
    use objc2_foundation::{NSArray, NSData, NSDictionary, NSString};
    use objc2_vision::{VNImageRequestHandler, VNRecognizeTextRequest, VNRequest, VNRequestTextRecognitionLevel};

    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(img.clone()).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
    let data = NSData::from_vec(png);
    unsafe {
        let request = VNRecognizeTextRequest::new();
        request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
        request.setUsesLanguageCorrection(true);
        let langs = NSArray::from_vec(vec![NSString::from_str("fr-FR"), NSString::from_str("en-US")]);
        request.setRecognitionLanguages(&langs);
        let handler = VNImageRequestHandler::initWithData_options(VNImageRequestHandler::alloc(), &data, &NSDictionary::new());
        let requests: Retained<NSArray<VNRequest>> = NSArray::from_slice(&[request.as_super().as_super()]);
        handler
            .performRequests_error(&requests)
            .map_err(|e| anyhow::anyhow!("{}", crate::i18n::trf("OCR impossible : {0}", &[&e.localizedDescription()])))?;
        let mut out = String::new();
        if let Some(results) = request.results() {
            for obs in results.iter() {
                if let Some(text) = obs.topCandidates(1).firstObject() {
                    out.push_str(&text.string().to_string());
                    out.push('\n');
                }
            }
        }
        Ok(out.trim_end().to_string())
    }
}

/// Passe l'application au premier plan (`activate` seul n'existe qu'à partir de macOS 14).
#[allow(deprecated)]
fn bring_app_to_front(mtm: MainThreadMarker) {
    NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
}

/// Pas de boucle modale de menu à débloquer sur Mac.
pub fn cancel_modal_loops() -> bool {
    false
}

/// Touche V enfoncée (pour ⌘V : egui n'émet aucun évènement quand le presse-papiers
/// contient une image et pas de texte).
pub fn is_v_key_down() -> bool {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceKeyState(state_id: i32, key: u16) -> bool;
    }
    // kCGEventSourceStateCombinedSessionState = 0, kVK_ANSI_V = 9.
    unsafe { CGEventSourceKeyState(0, 9) }
}

pub use crate::capture::imp::{request_screen_capture, screen_capture_allowed};

/// Ouvre Réglages Système à la page Confidentialité et sécurité > Enregistrement de l'écran.
pub fn open_screen_capture_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        .spawn();
}

/// Relance l'application (paquet .app) deux secondes après sa fermeture : macOS ne prend en compte
/// une nouvelle autorisation d'enregistrement de l'écran qu'au lancement suivant.
pub fn relaunch() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let target = exe
        .ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| exe.clone());
    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("sleep 2; open -n \"$0\"")
        .arg(&target)
        .spawn()?;
    Ok(())
}

/// Lancement à l'ouverture de session : agent ~/Library/LaunchAgents/io.github.salcyone29.screencapture.plist.
fn launch_agent() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(std::path::Path::new(&home).join("Library/LaunchAgents/io.github.salcyone29.screencapture.plist"))
}

pub fn launch_at_startup() -> bool {
    launch_agent().is_some_and(|p| p.exists())
}

pub fn set_launch_at_startup(on: bool) -> anyhow::Result<()> {
    let path = launch_agent().ok_or_else(|| anyhow::anyhow!("HOME"))?;
    if !on {
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        return Ok(());
    }
    let exe = std::env::current_exe()?;
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let plist = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n\t<key>Label</key>\n\t<string>io.github.salcyone29.screencapture</string>\n\t<key>ProgramArguments</key>\n\t<array>\n\t\t<string>{}</string>\n\t\t<string>--tray</string>\n\t</array>\n\t<key>RunAtLoad</key>\n\t<true/>\n</dict>\n</plist>\n",
        esc(&exe.display().to_string())
    );
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, plist)?;
    Ok(())
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn CGEventCreateScrollWheelEvent2(source: *const std::ffi::c_void, units: u32, count: u32, w1: i32, w2: i32, w3: i32) -> *mut std::ffi::c_void;
    fn CGEventPost(tap: u32, event: *mut std::ffi::c_void);
    fn CFRelease(cf: *const std::ffi::c_void);
}

/// Capture défilante : simuler la molette demande l'autorisation « Accessibilité ». Sans elle,
/// l'utilisateur fait défiler lui-même la page.
pub fn can_simulate_scroll() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

pub fn move_cursor(x: i32, y: i32) {
    let s = scale();
    let _ = core_graphics::display::CGDisplay::warp_mouse_cursor_position(core_graphics::display::CGPoint::new(x as f64 / s, y as f64 / s));
}

/// Fait défiler vers le bas le contenu sous le point (x, y) de `notches` crans.
pub fn scroll_wheel(x: i32, y: i32, notches: i32) {
    move_cursor(x, y);
    for _ in 0..notches.max(1) {
        unsafe {
            // Unité « ligne », valeur négative : contenu vers le bas (indépendant du sens naturel).
            let ev = CGEventCreateScrollWheelEvent2(std::ptr::null(), 1, 1, -3, 0, 0);
            if !ev.is_null() {
                CGEventPost(0, ev);
                CFRelease(ev);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
}
