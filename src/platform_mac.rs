//! Intégration macOS : géométrie de la fenêtre principale (AppKit), instance unique, corbeille, OCR (Vision).

use std::path::Path;

use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplication, NSNormalWindowLevel, NSStatusWindowLevel, NSView, NSWindow, NSWindowCollectionBehavior,
    NSWindowSharingType, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};

use super::PrintScreenHandler;
use crate::capture::imp::{rect_from_cg, rect_to_cg, scale};
use crate::capture::Rect;

/// Style et position sauvegardés pour revenir à la fenêtre « normale ».
#[derive(Clone, Copy, Debug)]
pub struct SavedWindow {
    style: NSWindowStyleMask,
    rect: Rect,
}

/// Fenêtre principale, identifiée par le pointeur de sa NSView (fourni par winit).
#[derive(Clone, Copy, Debug)]
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

    pub fn save(self) -> Option<SavedWindow> {
        let (w, _) = self.ns_window()?;
        Some(SavedWindow { style: w.styleMask(), rect: from_cocoa(w.frame()) })
    }

    fn set_visible(w: &NSWindow, visible: bool) {
        unsafe { w.setAlphaValue(if visible { 1.0 } else { 0.0 }) };
        w.setIgnoresMouseEvents(!visible);
    }

    pub fn restore(self, s: &SavedWindow) {
        self.unminimize();
        let Some((w, mtm)) = self.ns_window() else { return };
        w.setStyleMask(s.style);
        w.setLevel(NSNormalWindowLevel);
        w.setHasShadow(true);
        w.setSharingType(NSWindowSharingType::NSWindowSharingReadOnly);
        unsafe { w.setCollectionBehavior(NSWindowCollectionBehavior::Default) };
        Self::set_visible(&w, true);
        // L'écran d'origine a pu être débranché pendant que la fenêtre était cachée.
        w.setFrame_display(to_cocoa(crate::capture::fit_on_screen(s.rect)), true);
        w.makeKeyAndOrderFront(None);
        bring_app_to_front(mtm);
    }

    /// Fenêtre sans bordure, au-dessus de tout (barre des menus comprise), placée exactement sur `r`.
    pub fn popup(self, r: Rect, exclude_from_capture: bool, activate: bool) {
        self.unminimize();
        let Some((w, mtm)) = self.ns_window() else { return };
        w.setStyleMask(NSWindowStyleMask::Borderless);
        w.setLevel(NSStatusWindowLevel);
        w.setHasShadow(false);
        w.setSharingType(if exclude_from_capture {
            NSWindowSharingType::NSWindowSharingNone
        } else {
            NSWindowSharingType::NSWindowSharingReadOnly
        });
        unsafe {
            w.setCollectionBehavior(NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::FullScreenAuxiliary)
        };
        Self::set_visible(&w, true);
        w.setFrame_display(to_cocoa(r), true);
        if activate {
            bring_app_to_front(mtm);
            w.makeKeyAndOrderFront(None);
        } else {
            unsafe { w.orderFrontRegardless() };
        }
    }

    /// « Masque » la fenêtre en la rendant transparente et inerte : une fenêtre réellement
    /// masquée ne serait plus redessinée par eframe, qui ne traiterait plus les raccourcis.
    pub fn park(self) {
        let Some((w, mtm)) = self.ns_window() else { return };
        Self::set_visible(&w, false);
        w.setSharingType(NSWindowSharingType::NSWindowSharingNone);
        w.setLevel(NSNormalWindowLevel);
        unsafe {
            w.orderBack(None);
            // Rendre le clavier à l'application précédente.
            NSApplication::sharedApplication(mtm).deactivate();
        }
    }

    /// Les fenêtres macOS ont déjà des coins arrondis.
    pub fn round_corners(self, _round: bool) {}

    pub fn ensure_rect(self, r: Rect) {
        if self.rect() != Some(r) {
            if let Some((w, _)) = self.ns_window() {
                w.setFrame_display(to_cocoa(r), true);
            }
        }
    }

    /// Sort la fenêtre du Dock si elle a été réduite. Renvoie `true` si elle l'était.
    /// AppKit n'est utilisable que depuis le thread principal : ailleurs (surveillance), sans effet.
    pub fn unminimize(self) -> bool {
        let Some((w, _)) = self.ns_window() else { return false };
        if !w.isMiniaturized() {
            return false;
        }
        unsafe { w.deminiaturize(None) };
        crate::diag::log("fenêtre réduite : restaurée");
        true
    }

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

    pub fn focus(self) {
        self.unminimize();
        self.rescue();
        let Some((w, mtm)) = self.ns_window() else { return };
        Self::set_visible(&w, true);
        w.makeKeyAndOrderFront(None);
        bring_app_to_front(mtm);
    }
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
pub fn activation_listener(_f: impl Fn() + Send + 'static) {}

/// Envoie un fichier dans la Corbeille (récupérable).
pub fn move_to_recycle_bin(path: &Path) -> anyhow::Result<()> {
    use objc2_foundation::{NSFileManager, NSString, NSURL};
    let url = unsafe { NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy())) };
    unsafe { NSFileManager::defaultManager().trashItemAtURL_resultingItemURL_error(&url, None) }
        .map_err(|e| anyhow::anyhow!("suppression impossible : {}", e.localizedDescription()))
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
            .map_err(|e| anyhow::anyhow!("OCR impossible : {}", e.localizedDescription()))?;
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
