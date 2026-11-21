//! Capture d'écran Win32 (GDI) : bureau virtuel, moniteurs, fenêtres, curseur.

use image::RgbaImage;

use super::{monitor_at, Rect, WindowInfo};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, EnumDisplayMonitors, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, HDC, HMONITOR,
    SRCCOPY,
};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    DrawIconEx, EnumWindows, GetCursorInfo, GetCursorPos, GetForegroundWindow, GetIconInfo, GetSystemMetrics,
    GetWindowLongW, GetWindowRect, GetWindowTextLengthW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    CURSORINFO, CURSOR_SHOWING, DI_NORMAL, GWL_EXSTYLE, ICONINFO, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, WS_EX_TOOLWINDOW,
};

fn rect_from_win(r: &RECT) -> Rect {
    Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top)
}

pub fn virtual_screen() -> Rect {
    unsafe {
        Rect::new(
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    }
}

pub fn cursor_pos() -> (i32, i32) {
    let mut p = Default::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    (p.x, p.y)
}

pub fn monitors() -> Vec<Rect> {
    unsafe extern "system" fn cb(_m: HMONITOR, _dc: HDC, r: *mut RECT, data: LPARAM) -> BOOL {
        let v = &mut *(data.0 as *mut Vec<Rect>);
        v.push(rect_from_win(&*r));
        BOOL(1)
    }
    let mut v: Vec<Rect> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(HDC::default(), None, Some(cb), LPARAM(&mut v as *mut _ as isize));
    }
    if v.is_empty() {
        v.push(virtual_screen());
    }
    v
}

/// Zone de travail (sans la barre des tâches) du moniteur contenant le point.
pub fn work_area_at(x: i32, y: i32) -> Rect {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    unsafe {
        let mon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if GetMonitorInfoW(mon, &mut mi).as_bool() {
            return rect_from_win(&mi.rcWork);
        }
    }
    monitor_at(x, y)
}

/// Capture une zone du bureau virtuel.
pub fn capture_rect(r: Rect, with_cursor: bool) -> anyhow::Result<RgbaImage> {
    anyhow::ensure!(r.w > 0 && r.h > 0, crate::i18n::tr("zone vide"));
    unsafe {
        let screen = GetDC(HWND::default());
        let mem = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, r.w, r.h);
        let old = SelectObject(mem, bmp);
        let blit = BitBlt(mem, 0, 0, r.w, r.h, screen, r.x, r.y, SRCCOPY | CAPTUREBLT);
        if with_cursor {
            draw_cursor(mem, r);
        }
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: r.w,
                biHeight: -r.h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; (r.w * r.h * 4) as usize];
        SelectObject(mem, old);
        let lines = GetDIBits(mem, bmp, 0, r.h as u32, Some(buf.as_mut_ptr() as *mut _), &mut info, DIB_RGB_COLORS);
        let _ = DeleteObject(bmp);
        let _ = DeleteDC(mem);
        ReleaseDC(HWND::default(), screen);
        blit?;
        anyhow::ensure!(lines == r.h, crate::i18n::tr("GetDIBits a échoué"));
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2);
            px[3] = 255;
        }
        Ok(RgbaImage::from_raw(r.w as u32, r.h as u32, buf).expect("taille du tampon"))
    }
}

unsafe fn draw_cursor(dc: HDC, r: Rect) {
    let mut ci = CURSORINFO { cbSize: std::mem::size_of::<CURSORINFO>() as u32, ..Default::default() };
    if GetCursorInfo(&mut ci).is_err() || ci.flags != CURSOR_SHOWING {
        return;
    }
    let mut ii = ICONINFO::default();
    if GetIconInfo(ci.hCursor, &mut ii).is_err() {
        return;
    }
    let x = ci.ptScreenPos.x - r.x - ii.xHotspot as i32;
    let y = ci.ptScreenPos.y - r.y - ii.yHotspot as i32;
    let _ = DrawIconEx(dc, x, y, ci.hCursor, 0, 0, 0, None, DI_NORMAL);
    if !ii.hbmColor.is_invalid() {
        let _ = DeleteObject(ii.hbmColor);
    }
    if !ii.hbmMask.is_invalid() {
        let _ = DeleteObject(ii.hbmMask);
    }
}

unsafe fn window_bounds(hwnd: HWND) -> Option<Rect> {
    let mut r = RECT::default();
    let ok = DwmGetWindowAttribute(
        hwnd,
        DWMWA_EXTENDED_FRAME_BOUNDS,
        &mut r as *mut _ as *mut _,
        std::mem::size_of::<RECT>() as u32,
    )
    .is_ok();
    if !ok && GetWindowRect(hwnd, &mut r).is_err() {
        return None;
    }
    let rect = rect_from_win(&r);
    (rect.w > 0 && rect.h > 0).then_some(rect)
}

/// Fenêtres visibles de haut en bas dans l'ordre Z (hors fenêtres de ce processus).
pub fn visible_windows() -> Vec<WindowInfo> {
    unsafe extern "system" fn cb(hwnd: HWND, data: LPARAM) -> BOOL {
        let v = &mut *(data.0 as *mut Vec<WindowInfo>);
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return BOOL(1);
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == GetCurrentProcessId() {
            return BOOL(1);
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, 4);
        if cloaked != 0 {
            return BOOL(1);
        }
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if ex & WS_EX_TOOLWINDOW.0 != 0 && GetWindowTextLengthW(hwnd) == 0 {
            return BOOL(1);
        }
        if let Some(rect) = window_bounds(hwnd) {
            v.push(WindowInfo { rect });
        }
        BOOL(1)
    }
    let mut v: Vec<WindowInfo> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut v as *mut _ as isize));
    }
    v
}

/// Rectangle de la fenêtre au premier plan (pour la capture de fenêtre par raccourci).
pub fn foreground_window_rect() -> Option<Rect> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        window_bounds(hwnd).and_then(|r| r.intersect(&virtual_screen()))
    }
}

/// Fenêtre d'une autre application sous le point (x, y), la plus profonde (zone de contenu) :
/// destinataire de la molette pendant la sélection, qui recouvre tout l'écran.
pub fn wheel_target(x: i32, y: i32) -> Option<isize> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    use windows::Win32::UI::WindowsAndMessaging::{ChildWindowFromPointEx, CWP_SKIPINVISIBLE, CWP_SKIPTRANSPARENT};
    struct Find {
        x: i32,
        y: i32,
        found: Option<HWND>,
    }
    unsafe extern "system" fn cb(hwnd: HWND, data: LPARAM) -> BOOL {
        let f = &mut *(data.0 as *mut Find);
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return BOOL(1);
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == GetCurrentProcessId() {
            return BOOL(1);
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, 4);
        if cloaked != 0 {
            return BOOL(1);
        }
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        // Fenêtres transparentes aux clics (bandeaux, effets) : la molette les traverse.
        if ex & windows::Win32::UI::WindowsAndMessaging::WS_EX_TRANSPARENT.0 != 0 {
            return BOOL(1);
        }
        if window_bounds(hwnd).is_some_and(|r| r.contains(f.x, f.y)) {
            f.found = Some(hwnd);
            return BOOL(0);
        }
        BOOL(1)
    }
    let mut f = Find { x, y, found: None };
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut f as *mut _ as isize));
        let mut hwnd = f.found?;
        // Descendre jusqu'à la fenêtre enfant sous le point.
        for _ in 0..16 {
            let mut pt = POINT { x, y };
            let _ = ScreenToClient(hwnd, &mut pt);
            let child = ChildWindowFromPointEx(hwnd, pt, CWP_SKIPINVISIBLE | CWP_SKIPTRANSPARENT);
            if child.0.is_null() || child == hwnd {
                break;
            }
            hwnd = child;
        }
        let mut class = [0u16; 128];
        let n = windows::Win32::UI::WindowsAndMessaging::GetClassNameW(hwnd, &mut class);
        crate::diag::log(&format!("molette : fenêtre {:#x} « {} »", hwnd.0 as isize, String::from_utf16_lossy(&class[..n.max(0) as usize])));
        Some(hwnd.0 as isize)
    }
}

/// Fait défiler vers le bas la fenêtre `target` de `notches` crans, comme la molette au point (x, y).
pub fn post_wheel(target: isize, x: i32, y: i32, notches: i32) {
    use windows::Win32::Foundation::WPARAM;
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_MOUSEWHEEL};
    let delta = (-120 * notches.max(1)) as i16 as u16 as usize;
    let lparam = ((y as u16 as u32) << 16 | (x as u16 as u32)) as i32 as isize;
    unsafe {
        let _ = PostMessageW(HWND(target as *mut _), WM_MOUSEWHEEL, WPARAM(delta << 16), LPARAM(lparam));
    }
}
