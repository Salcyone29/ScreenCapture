//! Intégration Windows : géométrie de la fenêtre principale, instance unique, corbeille, OCR.

use std::os::windows::process::CommandExt;
use std::path::Path;

use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND, RECT};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Shell::{ShellExecuteW, SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_SILENT, FO_DELETE, SHFILEOPSTRUCTW};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, GetWindowRect, SetForegroundWindow, SetWindowDisplayAffinity, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, GWL_EXSTYLE, GWL_STYLE, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOSIZE,
    SWP_SHOWWINDOW, SW_HIDE, SW_SHOW, WDA_EXCLUDEFROMCAPTURE, WDA_NONE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    WS_POPUP, WS_VISIBLE,
};

use super::{PrintScreenHandler, KEY_SNIP, MOD_ALT, MOD_CTRL, MOD_SHIFT, MOD_WIN, NO_FIRE};
use crate::capture::Rect;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Style et position sauvegardés pour revenir à la fenêtre « normale ».
#[derive(Clone, Copy, Debug)]
pub struct SavedWindow {
    style: isize,
    ex_style: isize,
    rect: Rect,
}

#[derive(Clone, Copy, Debug)]
pub struct Window(pub isize);

impl Window {
    fn hwnd(self) -> HWND {
        HWND(self.0 as *mut _)
    }

    pub fn is_valid(self) -> bool {
        self.0 != 0
    }

    pub fn rect(self) -> Option<Rect> {
        let mut r = RECT::default();
        unsafe { GetWindowRect(self.hwnd(), &mut r).ok()? };
        Some(Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top))
    }

    pub fn save(self) -> Option<SavedWindow> {
        if !self.is_valid() {
            return None;
        }
        unsafe {
            Some(SavedWindow {
                style: GetWindowLongPtrW(self.hwnd(), GWL_STYLE),
                ex_style: GetWindowLongPtrW(self.hwnd(), GWL_EXSTYLE),
                rect: self.rect()?,
            })
        }
    }

    pub fn restore(self, s: &SavedWindow) {
        if !self.is_valid() {
            return;
        }
        self.unminimize();
        unsafe {
            // Masquer puis réafficher pour que la barre des tâches prenne en compte le style.
            let _ = ShowWindow(self.hwnd(), SW_HIDE);
            SetWindowLongPtrW(self.hwnd(), GWL_STYLE, s.style);
            SetWindowLongPtrW(self.hwnd(), GWL_EXSTYLE, s.ex_style);
            let _ = SetWindowDisplayAffinity(self.hwnd(), WDA_NONE);
            // L'écran d'origine a pu être débranché pendant que la fenêtre était garée.
            let r = crate::capture::fit_on_screen(s.rect);
            let _ = SetWindowPos(self.hwnd(), HWND_NOTOPMOST, r.x, r.y, r.w, r.h, SWP_FRAMECHANGED | SWP_SHOWWINDOW);
            let _ = SetForegroundWindow(self.hwnd());
        }
    }

    fn set_tool_style(self, popup: bool) {
        unsafe {
            let _ = ShowWindow(self.hwnd(), SW_HIDE);
            if popup {
                SetWindowLongPtrW(self.hwnd(), GWL_STYLE, (WS_POPUP.0 | WS_VISIBLE.0) as isize);
            }
            let ex = GetWindowLongPtrW(self.hwnd(), GWL_EXSTYLE) as u32;
            let ex = (ex & !WS_EX_APPWINDOW.0) | WS_EX_TOOLWINDOW.0;
            SetWindowLongPtrW(self.hwnd(), GWL_EXSTYLE, ex as isize);
        }
    }

    /// Fenêtre sans bordure, au premier plan, placée exactement sur `r` (pixels physiques).
    pub fn popup(self, r: Rect, exclude_from_capture: bool, activate: bool) {
        if !self.is_valid() {
            return;
        }
        self.unminimize();
        self.set_tool_style(true);
        unsafe {
            let _ = SetWindowDisplayAffinity(
                self.hwnd(),
                if exclude_from_capture { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE },
            );
            let mut flags = SWP_FRAMECHANGED | SWP_SHOWWINDOW;
            if !activate {
                flags |= SWP_NOACTIVATE;
            }
            let _ = SetWindowPos(self.hwnd(), HWND_TOPMOST, r.x, r.y, r.w, r.h, flags);
            if activate {
                let _ = SetForegroundWindow(self.hwnd());
            }
        }
    }

    /// « Masque » la fenêtre en la garant hors écran, sans bouton dans la barre des tâches.
    /// Une fenêtre réellement masquée ne serait plus rafraîchie par eframe, et l'application
    /// ne traiterait plus les raccourcis ni l'icône de notification.
    pub fn park(self) {
        if !self.is_valid() {
            return;
        }
        self.set_tool_style(false);
        unsafe {
            let _ = SetWindowDisplayAffinity(self.hwnd(), WDA_EXCLUDEFROMCAPTURE);
            let _ = SetWindowPos(
                self.hwnd(),
                HWND_BOTTOM,
                -32000,
                -32000,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
            );
        }
    }

    /// Coins arrondis DWM (Windows 11) pour les fenêtres flottantes ; sans effet ailleurs.
    pub fn round_corners(self, round: bool) {
        use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DEFAULT, DWMWCP_ROUND};
        if !self.is_valid() {
            return;
        }
        let pref = if round { DWMWCP_ROUND } else { DWMWCP_DEFAULT };
        unsafe {
            let _ = DwmSetWindowAttribute(
                self.hwnd(),
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &pref as *const _ as *const _,
                std::mem::size_of_val(&pref) as u32,
            );
        }
    }

    pub fn ensure_rect(self, r: Rect) {
        if self.rect() != Some(r) {
            unsafe {
                let _ = SetWindowPos(self.hwnd(), HWND_TOPMOST, r.x, r.y, r.w, r.h, SWP_NOACTIVATE);
            }
        }
    }

    /// Sort la fenêtre de l'état réduit (Win+D, « Afficher le bureau », clic dans la barre des
    /// tâches…) : eframe n'exécute plus l'application tant que sa fenêtre est réduite, ce qui
    /// bloquait captures, raccourcis et menu. Asynchrone, donc appelable depuis n'importe quel thread.
    /// Renvoie `true` si la fenêtre était réduite.
    pub fn unminimize(self) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::{IsIconic, ShowWindowAsync, SW_SHOWNOACTIVATE};
        if !self.is_valid() || !unsafe { IsIconic(self.hwnd()) }.as_bool() {
            return false;
        }
        unsafe {
            let _ = ShowWindowAsync(self.hwnd(), SW_SHOWNOACTIVATE);
        }
        super::WAS_MINIMIZED.store(true, std::sync::atomic::Ordering::SeqCst);
        crate::diag::log("fenêtre réduite : restaurée");
        true
    }

    /// Replace la fenêtre sur un écran connecté si elle est restée sur un écran débranché
    /// (sans effet si elle est garée hors écran volontairement). Appelable depuis un autre thread.
    pub fn rescue(self) {
        if self.unminimize() {
            return;
        }
        let Some(r) = self.rect().filter(|r| r.x > -30000 && r.y > -30000) else {
            return;
        };
        let fit = crate::capture::fit_on_screen(r);
        if fit != r {
            unsafe {
                let _ = SetWindowPos(self.hwnd(), HWND_NOTOPMOST, fit.x, fit.y, fit.w, fit.h, SWP_NOACTIVATE);
            }
        }
    }

    pub fn focus(self) {
        self.rescue();
        unsafe {
            let _ = ShowWindow(self.hwnd(), SW_SHOW);
            let _ = SetForegroundWindow(self.hwnd());
        }
    }
}

static PRINTSCREEN_HANDLER: std::sync::OnceLock<PrintScreenHandler> = std::sync::OnceLock::new();
static PRINTSCREEN_DOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SNIP_SWALLOWED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Intercepte Impr. écran (et Win+Maj+S) par un hook clavier bas niveau. `RegisterHotKey` ne suffit pas :
/// d'autres outils de capture (ou Windows) consomment souvent cette touche avant lui.
/// `handler(modificateurs)` renvoie `true` si la touche doit être consommée.
pub fn install_printscreen_hook(handler: PrintScreenHandler) -> bool {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowsHookExW, WH_KEYBOARD_LL};
    use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, TranslateMessage, MSG};
    if PRINTSCREEN_HANDLER.set(handler).is_err() {
        return true;
    }
    // Thread dédié avec sa propre boucle de messages : le thread de l'interface peut être
    // occupé par le rendu, et Windows ignore (puis retire) un hook qui répond trop tard.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("printscreen-hook".into())
        .spawn(move || unsafe {
            let module = GetModuleHandleW(None).unwrap_or_default();
            let r = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0);
            crate::debug_log(&format!("hook clavier installé : {r:?}"));
            let _ = tx.send(r.is_ok());
            if r.is_err() {
                return;
            }
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .ok();
    rx.recv().unwrap_or(false)
}

unsafe extern "system" fn keyboard_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use std::sync::atomic::Ordering;
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, GetAsyncKeyState, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT, VK_SNAPSHOT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_SYSKEYDOWN};

    if code >= 0 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let is_prt = kb.vkCode == VK_SNAPSHOT.0 as u32;
        let is_s = kb.vkCode == 0x53;
        if let (true, Some(h)) = (is_prt || is_s, PRINTSCREEN_HANDLER.get()) {
            let pressed = |vk: u16| GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0;
            let mut mods = 0;
            if pressed(VK_CONTROL.0) { mods |= MOD_CTRL; }
            if pressed(VK_SHIFT.0) { mods |= MOD_SHIFT; }
            if pressed(VK_MENU.0) { mods |= MOD_ALT; }
            if pressed(VK_LWIN.0) || pressed(VK_RWIN.0) { mods |= MOD_WIN; }
            let msg = wparam.0 as u32;
            let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            if is_prt {
                crate::debug_log(&format!("hook : Impr. écran msg={msg:#x}"));
                // Déclenche à l'appui ; au relâchement seulement si l'appui n'a pas été vu.
                let fire = if down { !PRINTSCREEN_DOWN.swap(true, Ordering::SeqCst) } else { !PRINTSCREEN_DOWN.swap(false, Ordering::SeqCst) };
                if h(if fire { mods } else { mods | NO_FIRE }) {
                    return LRESULT(1);
                }
            } else if down {
                // Win+Maj+S : raccourci de l'Outil Capture, envoyé tel quel par la touche
                // « capture d'écran » de nombreux claviers récents.
                if mods == MOD_WIN | MOD_SHIFT && h(mods | KEY_SNIP) {
                    crate::debug_log("hook : Win+Maj+S intercepté");
                    SNIP_SWALLOWED.store(true, Ordering::SeqCst);
                    // Touche neutre pour que le relâchement de Win n'ouvre pas le menu Démarrer.
                    keybd_event(0xE8, 0, KEYBD_EVENT_FLAGS(0), 0);
                    keybd_event(0xE8, 0, KEYEVENTF_KEYUP, 0);
                    return LRESULT(1);
                }
            } else if SNIP_SWALLOWED.swap(false, Ordering::SeqCst) {
                return LRESULT(1);
            }
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

/// Renvoie `false` si une autre instance tourne déjà. Avec `activate_existing`, celle-ci est priée
/// d'afficher sa fenêtre ; si elle est bloquée elle se termine, et cette instance prend le relais.
pub fn acquire_single_instance(activate_existing: bool) -> bool {
    use windows::Win32::Foundation::{WAIT_ABANDONED, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE};
    unsafe {
        // Le handle est volontairement conservé pendant toute la vie du processus.
        let Ok(handle) = CreateMutexW(None, true, w!("Local\\ScreenCapture.Rust.SingleInstance")) else {
            return true;
        };
        if GetLastError() != ERROR_ALREADY_EXISTS {
            return true;
        }
        if !activate_existing {
            return false;
        }
        crate::diag::log("une autre instance tourne : demande d'affichage");
        if let Ok(ev) = OpenEventW(EVENT_MODIFY_STATE, false, ACTIVATE_EVENT) {
            let _ = SetEvent(ev);
        }
        let w = WaitForSingleObject(handle, 9000);
        let take_over = w == WAIT_OBJECT_0 || w == WAIT_ABANDONED;
        if take_over {
            crate::diag::log("l'instance précédente s'est arrêtée : relance");
        }
        take_over
    }
}

const ACTIVATE_EVENT: PCWSTR = w!("Local\\ScreenCapture.Rust.Activate");

/// Appelle `f` (depuis un thread dédié) chaque fois qu'une nouvelle instance est lancée.
pub fn activation_listener(f: impl Fn() + Send + 'static) {
    use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject, INFINITE};
    let Ok(ev) = (unsafe { CreateEventW(None, false, false, ACTIVATE_EVENT) }) else { return };
    let ev = ev.0 as isize;
    let _ = std::thread::Builder::new().name("activation".into()).spawn(move || loop {
        let h = windows::Win32::Foundation::HANDLE(ev as *mut _);
        if unsafe { WaitForSingleObject(h, INFINITE) } != windows::Win32::Foundation::WAIT_OBJECT_0 {
            return;
        }
        f();
    });
}

/// Envoie un fichier dans la corbeille (récupérable).
pub fn move_to_recycle_bin(path: &Path) -> anyhow::Result<()> {
    let mut from: Vec<u16> = path.as_os_str().encode_wide_vec();
    from.push(0);
    from.push(0);
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT).0 as u16,
        ..Default::default()
    };
    let r = unsafe { SHFileOperationW(&mut op) };
    anyhow::ensure!(r == 0, "suppression impossible (code {r})");
    Ok(())
}

trait EncodeWideVec {
    fn encode_wide_vec(&self) -> Vec<u16>;
}

impl EncodeWideVec for std::ffi::OsStr {
    fn encode_wide_vec(&self) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        self.encode_wide().collect()
    }
}

/// Ouvre un fichier avec l'application par défaut.
pub fn open_with_default(path: &Path) {
    unsafe {
        ShellExecuteW(
            HWND::default(),
            w!("open"),
            &HSTRING::from(path.as_os_str()),
            PCWSTR::null(),
            PCWSTR::null(),
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        );
    }
}

/// Ouvre l'Explorateur en sélectionnant le fichier.
pub fn reveal_in_explorer(path: &Path) {
    let _ = std::process::Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .spawn();
}

pub fn open_folder(path: &Path) {
    let _ = std::fs::create_dir_all(path);
    let _ = std::process::Command::new("explorer.exe").arg(path).spawn();
}

pub fn hidden_command(program: &str) -> std::process::Command {
    let mut c = std::process::Command::new(program);
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

/// Reconnaissance de texte via Windows.Media.Ocr (langues du profil utilisateur).
pub fn ocr(img: &image::RgbaImage) -> anyhow::Result<String> {
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let max = OcrEngine::MaxImageDimension()? as u32;
    let img = if img.width() > max || img.height() > max {
        let s = max as f32 / img.width().max(img.height()) as f32;
        image::imageops::resize(
            img,
            (img.width() as f32 * s) as u32,
            (img.height() as f32 * s) as u32,
            image::imageops::FilterType::Triangle,
        )
    } else {
        img.clone()
    };
    let mut bgra = img.as_raw().clone();
    for px in bgra.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    let writer = DataWriter::new()?;
    writer.WriteBytes(&bgra)?;
    let buffer = writer.DetachBuffer()?;
    let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        img.width() as i32,
        img.height() as i32,
        BitmapAlphaMode::Premultiplied,
    )?;
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
    let result = engine.RecognizeAsync(&bitmap)?.get()?;
    let mut out = String::new();
    for line in result.Lines()? {
        out.push_str(&line.Text()?.to_string_lossy());
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

/// Vrai si notre fenêtre est au premier plan.
pub fn is_foreground(win: Window) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    unsafe { GetForegroundWindow().0 as isize == win.0 }
}

/// Pixels d'écran (unités de `Rect`) par point egui.
pub fn units_per_point(ctx: &eframe::egui::Context) -> f32 {
    ctx.pixels_per_point()
}

/// Identifiant natif de la fenêtre principale (HWND).
pub fn native_handle(h: raw_window_handle::RawWindowHandle) -> isize {
    match h {
        raw_window_handle::RawWindowHandle::Win32(h) => h.hwnd.get(),
        _ => 0,
    }
}

/// Ferme un menu ou un mode modal resté ouvert (menu de notification caché sous la sélection
/// plein écran…) : tant qu'il est ouvert, Windows suspend la boucle de l'application.
/// Renvoie `true` si un message a été envoyé.
pub fn cancel_modal_loops() -> bool {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CANCELMODE};
    unsafe extern "system" fn cb(hwnd: HWND, data: LPARAM) -> windows::Win32::Foundation::BOOL {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == std::process::id() {
            let _ = PostMessageW(hwnd, WM_CANCELMODE, WPARAM(0), LPARAM(0));
            *(data.0 as *mut u32) += 1;
        }
        windows::Win32::Foundation::BOOL(1)
    }
    let mut n = 0u32;
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut n as *mut u32 as isize));
    }
    n > 0
}
