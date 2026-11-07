//! Capture d'écran macOS (CoreGraphics) : écrans, fenêtres, curseur.
//!
//! macOS place les écrans en points, avec l'origine en haut à gauche de l'écran principal.
//! Les `Rect` de l'application sont ces points multipliés par la plus grande échelle Retina
//! des écrans branchés (`scale()`), pour que les captures gardent leur pleine résolution.
//!
//! Une fenêtre macOS ne s'étend pas sur plusieurs écrans : le « bureau virtuel » capturé est
//! donc l'écran sous le curseur.

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_graphics::display::{CGDisplay, CGPoint, CGRect, CGSize};
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::window::{
    copy_window_info, kCGNullWindowID, kCGWindowBounds, kCGWindowImageBestResolution, kCGWindowLayer,
    kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly, kCGWindowOwnerPID,
};
use image::RgbaImage;

use super::{Rect, WindowInfo};

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGRectMakeWithDictionaryRepresentation(dict: CFDictionaryRef, rect: *mut CGRect) -> bool;
}

/// Pixels par point macOS : la plus grande échelle Retina des écrans branchés. Mise en cache
/// une seconde : la sélection la demande à chaque image (curseur, conversion en points).
pub fn scale() -> f64 {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static CACHE: Mutex<Option<(Instant, f64)>> = Mutex::new(None);
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, s)) = *cache {
        if at.elapsed() < Duration::from_secs(1) {
            return s;
        }
    }
    let s = compute_scale();
    *cache = Some((Instant::now(), s));
    s
}

fn compute_scale() -> f64 {
    CGDisplay::active_displays()
        .unwrap_or_default()
        .into_iter()
        .map(|id| {
            let d = CGDisplay::new(id);
            let w = d.bounds().size.width;
            if w > 0.0 { d.pixels_wide() as f64 / w } else { 1.0 }
        })
        .fold(1.0, f64::max)
}

/// Échelle Retina propre à un écran (1 ou 2 en pratique).
pub fn display_scale(d: CGDisplay) -> f64 {
    let w = d.bounds().size.width;
    if w > 0.0 { d.pixels_wide() as f64 / w } else { 1.0 }
}

pub fn rect_from_cg(r: CGRect, s: f64) -> Rect {
    let x0 = (r.origin.x * s).round() as i32;
    let y0 = (r.origin.y * s).round() as i32;
    let x1 = ((r.origin.x + r.size.width) * s).round() as i32;
    let y1 = ((r.origin.y + r.size.height) * s).round() as i32;
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

pub fn rect_to_cg(r: Rect, s: f64) -> CGRect {
    CGRect::new(
        &CGPoint::new(r.x as f64 / s, r.y as f64 / s),
        &CGSize::new(r.w as f64 / s, r.h as f64 / s),
    )
}

/// Écrans branchés, avec leur rectangle en unités de l'application.
pub fn displays() -> Vec<(CGDisplay, Rect)> {
    let s = scale();
    CGDisplay::active_displays()
        .unwrap_or_default()
        .into_iter()
        .map(|id| {
            let d = CGDisplay::new(id);
            (d, rect_from_cg(d.bounds(), s))
        })
        .collect()
}

/// Écran sous le curseur.
pub fn virtual_screen() -> Rect {
    let (x, y) = cursor_pos();
    super::monitor_at(x, y)
}

pub fn cursor_pos() -> (i32, i32) {
    let p = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .ok()
        .and_then(|src| CGEvent::new(src).ok())
        .map(|e| e.location())
        .unwrap_or(CGPoint::new(0.0, 0.0));
    let s = scale();
    ((p.x * s).round() as i32, (p.y * s).round() as i32)
}

pub fn monitors() -> Vec<Rect> {
    let mut v: Vec<Rect> = displays().into_iter().map(|(_, r)| r).collect();
    if v.is_empty() {
        v.push(rect_from_cg(CGDisplay::main().bounds(), scale()));
    }
    v
}

/// Zone utile de l'écran contenant le point (sans la barre des menus ni le Dock).
pub fn work_area_at(x: i32, y: i32) -> Rect {
    use objc2_app_kit::NSScreen;
    use objc2_foundation::MainThreadMarker;
    let s = scale();
    if let Some(mtm) = MainThreadMarker::new() {
        let screens = NSScreen::screens(mtm);
        // Coordonnées Cocoa : origine en bas à gauche de l'écran principal (le premier).
        let primary_h = unsafe { screens.firstObject() }.map(|p| p.frame().size.height).unwrap_or(0.0);
        for scr in screens.iter() {
            let to_cg = |f: objc2_foundation::NSRect| {
                CGRect::new(
                    &CGPoint::new(f.origin.x, primary_h - f.origin.y - f.size.height),
                    &CGSize::new(f.size.width, f.size.height),
                )
            };
            if rect_from_cg(to_cg(scr.frame()), s).contains(x, y) {
                return rect_from_cg(to_cg(scr.visibleFrame()), s);
            }
        }
    }
    // Hors du thread principal : écran entier moins la barre des menus.
    let m = super::monitor_at(x, y);
    let bar = (25.0 * s) as i32;
    Rect::new(m.x, m.y + bar, m.w, m.h - bar)
}

/// Capture une zone (unités de l'application) ; l'image fait `r.w × r.h` pixels.
pub fn capture_rect(r: Rect, _with_cursor: bool) -> anyhow::Result<RgbaImage> {
    anyhow::ensure!(r.w > 0 && r.h > 0, "zone vide");
    let s = scale();
    let cg = CGDisplay::screenshot(
        rect_to_cg(r, s),
        kCGWindowListOptionOnScreenOnly,
        kCGNullWindowID,
        kCGWindowImageBestResolution,
    )
    .ok_or_else(|| {
        anyhow::anyhow!(
            "Capture refusée : autorisez ScreenCapture dans Réglages Système > Confidentialité et sécurité > Enregistrement de l'écran, puis relancez l'application."
        )
    })?;
    let (w, h, stride) = (cg.width(), cg.height(), cg.bytes_per_row());
    anyhow::ensure!(w > 0 && h > 0 && cg.bits_per_pixel() == 32, "format d'image inattendu");
    let data = cg.data();
    let bytes = data.bytes();
    let mut buf = Vec::with_capacity(w * h * 4);
    for row in 0..h {
        for px in bytes[row * stride..row * stride + w * 4].chunks_exact(4) {
            // BGRA (alpha prémultiplié, toujours opaque pour l'écran).
            buf.extend_from_slice(&[px[2], px[1], px[0], 255]);
        }
    }
    let img = RgbaImage::from_raw(w as u32, h as u32, buf).expect("taille du tampon");
    // Écran moins dense que l'échelle commune : agrandir pour rester en unités de l'application.
    if img.width() != r.w as u32 || img.height() != r.h as u32 {
        return Ok(image::imageops::resize(&img, r.w as u32, r.h as u32, image::imageops::FilterType::Triangle));
    }
    Ok(img)
}

/// Fenêtres normales visibles, de la plus haute à la plus basse (hors fenêtres de ce processus).
pub fn visible_windows() -> Vec<WindowInfo> {
    let s = scale();
    let me = std::process::id() as i64;
    let Some(list) = copy_window_info(kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, kCGNullWindowID) else {
        return Vec::new();
    };
    let list: CFArray<CFDictionary<CFString, CFType>> = unsafe { CFArray::wrap_under_get_rule(list.as_concrete_TypeRef()) };
    let key = |k| unsafe { CFString::wrap_under_get_rule(k) };
    let num = |d: &CFDictionary<CFString, CFType>, k| d.find(key(k)).and_then(|v| v.downcast::<CFNumber>()).and_then(|n| n.to_i64());
    list.iter()
        .filter_map(|d| {
            if num(&d, unsafe { kCGWindowLayer }) != Some(0) || num(&d, unsafe { kCGWindowOwnerPID }) == Some(me) {
                return None;
            }
            let b = d.find(key(unsafe { kCGWindowBounds }))?;
            let mut r = CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(0.0, 0.0));
            let ok = unsafe { CGRectMakeWithDictionaryRepresentation(b.as_CFTypeRef() as CFDictionaryRef, &mut r) };
            let rect = rect_from_cg(r, s);
            (ok && rect.w > 1 && rect.h > 1).then_some(WindowInfo { rect })
        })
        .collect()
}

/// Fenêtre au premier plan : la plus haute des fenêtres normales des autres applications.
pub fn foreground_window_rect() -> Option<Rect> {
    let screen = virtual_screen();
    visible_windows().into_iter().find_map(|w| w.rect.intersect(&screen))
}
