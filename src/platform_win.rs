//! Intégration Windows : géométrie de la fenêtre principale, instance unique, corbeille, OCR.

use std::os::windows::process::CommandExt;
use std::path::Path;

use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND, POINT, RECT};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Shell::{ShellExecuteW, SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_SILENT, FO_DELETE, SHFILEOPSTRUCTW};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, GetWindowRect, SetForegroundWindow, SetWindowDisplayAffinity, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE,
    WDA_EXCLUDEFROMCAPTURE, WDA_NONE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

use super::{PrintScreenHandler, KEY_ESCAPE, KEY_SNIP, MOD_ALT, MOD_CTRL, MOD_SHIFT, MOD_WIN, NO_FIRE};
use crate::capture::Rect;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Fenêtre native : la fenêtre principale (espace de travail / éditeur) ou une fenêtre flottante
/// (sélection, miniature, barre d'enregistrement), chacune étant une fenêtre egui distincte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window(pub isize);

impl Window {
    fn hwnd(self) -> HWND {
        HWND(self.0 as *mut _)
    }

    pub fn is_valid(self) -> bool {
        self.0 != 0 && unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindow(self.hwnd()) }.as_bool()
    }

    pub fn rect(self) -> Option<Rect> {
        let mut r = RECT::default();
        unsafe { GetWindowRect(self.hwnd(), &mut r).ok()? };
        Some(Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top))
    }

    /// Fenêtre de ce processus portant exactement ce titre (fenêtre flottante egui).
    pub fn find(title: &str) -> Option<Window> {
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowThreadProcessId};
        let h = unsafe { FindWindowW(None, &HSTRING::from(title)) }.ok()?;
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        (pid == std::process::id()).then_some(Window(h.0 as isize))
    }

    /// Masque la fenêtre à l'écran (DWM) sans la cacher pour Windows : elle reste à sa place et
    /// à sa taille, et eframe continue de la dessiner, donc les raccourcis restent actifs.
    pub fn cloak(self, cloaked: bool) {
        use windows::Win32::Foundation::BOOL;
        use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CLOAK};
        if !self.is_valid() {
            return;
        }
        let value = BOOL::from(cloaked);
        unsafe {
            let _ = DwmSetWindowAttribute(self.hwnd(), DWMWA_CLOAK, &value as *const _ as *const _, std::mem::size_of::<BOOL>() as u32);
        }
    }

    /// Fenêtre « transparente » à la souris : clics et molette atteignent la fenêtre en dessous
    /// (elle reste affichée).
    pub fn set_click_through(self, on: bool) {
        use windows::Win32::UI::WindowsAndMessaging::{SetLayeredWindowAttributes, LWA_ALPHA, WS_EX_LAYERED, WS_EX_TRANSPARENT};
        if !self.is_valid() {
            return;
        }
        unsafe {
            let ex = GetWindowLongPtrW(self.hwnd(), GWL_EXSTYLE) as u32;
            let flags = WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0;
            let new = if on { ex | flags } else { ex & !flags };
            if new != ex {
                SetWindowLongPtrW(self.hwnd(), GWL_EXSTYLE, new as isize);
                if on {
                    let _ = SetLayeredWindowAttributes(self.hwnd(), windows::Win32::Foundation::COLORREF(0), 255, LWA_ALPHA);
                }
            }
        }
    }

    pub fn is_minimized(self) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::IsIconic;
        self.is_valid() && unsafe { IsIconic(self.hwnd()) }.as_bool()
    }

    /// Bouton dans la barre des tâches (fenêtre d'application) ou non (fenêtre outil).
    /// La barre des tâches ne relit ce style qu'au réaffichage : on masque/réaffiche donc la
    /// fenêtre, ce qui ne se voit pas quand elle est masquée par DWM.
    fn set_taskbar_button(self, visible: bool) {
        use windows::Win32::UI::WindowsAndMessaging::{SW_SHOWMINNOACTIVE, SW_SHOWNA};
        unsafe {
            let ex = GetWindowLongPtrW(self.hwnd(), GWL_EXSTYLE) as u32;
            let new = if visible { (ex | WS_EX_APPWINDOW.0) & !WS_EX_TOOLWINDOW.0 } else { (ex & !WS_EX_APPWINDOW.0) | WS_EX_TOOLWINDOW.0 };
            if new == ex {
                return;
            }
            let minimized = self.is_minimized();
            let _ = ShowWindow(self.hwnd(), SW_HIDE);
            SetWindowLongPtrW(self.hwnd(), GWL_EXSTYLE, new as isize);
            let _ = ShowWindow(self.hwnd(), if minimized { SW_SHOWMINNOACTIVE } else { SW_SHOWNA });
        }
    }

    /// Fenêtre principale : la rendre invisible (zone de notification). Elle n'est ni déplacée
    /// ni redimensionnée, et n'apparaît plus dans la barre des tâches ni dans Alt+Tab.
    pub fn hide(self) {
        if !self.is_valid() {
            return;
        }
        self.cloak(true);
        self.set_taskbar_button(false);
    }

    /// Fenêtre principale : la réafficher telle qu'elle était, restaurée si elle était réduite,
    /// ramenée sur un écran branché si le sien a été débranché, et au premier plan.
    pub fn show(self) {
        use windows::Win32::UI::WindowsAndMessaging::SW_RESTORE;
        if !self.is_valid() {
            return;
        }
        self.set_taskbar_button(true);
        unsafe {
            if self.is_minimized() {
                let _ = ShowWindow(self.hwnd(), SW_RESTORE);
            }
        }
        self.rescue();
        self.cloak(false);
        self.force_foreground();
    }

    /// Fenêtre flottante : placée exactement sur `r` (pixels physiques), au-dessus de tout, sans
    /// bouton dans la barre des tâches. `exclude_from_capture` : invisible pour les captures et
    /// les vidéos (barre d'enregistrement).
    pub fn place_floating(self, r: Rect, activate: bool, exclude_from_capture: bool) {
        if !self.is_valid() {
            return;
        }
        unsafe {
            let _ = SetWindowDisplayAffinity(self.hwnd(), if exclude_from_capture { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE });
            let ex = GetWindowLongPtrW(self.hwnd(), GWL_EXSTYLE) as u32;
            SetWindowLongPtrW(self.hwnd(), GWL_EXSTYLE, ((ex & !WS_EX_APPWINDOW.0) | WS_EX_TOOLWINDOW.0) as isize);
            let mut flags = SWP_SHOWWINDOW | SWP_FRAMECHANGED;
            if !activate {
                flags |= SWP_NOACTIVATE;
            }
            let _ = SetWindowPos(self.hwnd(), HWND_TOPMOST, r.x, r.y, r.w, r.h, flags);
        }
        if activate {
            self.force_foreground();
        }
    }

    /// Premier plan, même quand une autre application l'a (Windows le refuse alors) : la
    /// sélection doit recevoir le clavier (Échap, Entrée) dès qu'elle s'ouvre.
    fn force_foreground(self) {
        use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
        use windows::Win32::UI::WindowsAndMessaging::{BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId};
        unsafe {
            if SetForegroundWindow(self.hwnd()).as_bool() && GetForegroundWindow() == self.hwnd() {
                return;
            }
            let fg = GetForegroundWindow();
            let fg_thread = GetWindowThreadProcessId(fg, None);
            let me = GetCurrentThreadId();
            if fg_thread != 0 && fg_thread != me {
                let _ = AttachThreadInput(me, fg_thread, true);
                let _ = BringWindowToTop(self.hwnd());
                let _ = SetForegroundWindow(self.hwnd());
                let _ = AttachThreadInput(me, fg_thread, false);
            }
        }
    }

    /// Coins arrondis DWM (Windows 11) ; sans effet ailleurs.
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

    /// Replace la fenêtre sur un écran connecté si elle est restée sur un écran débranché
    /// (sans effet si elle est réduite).
    pub fn rescue(self) {
        if self.is_minimized() {
            return;
        }
        let Some(r) = self.rect() else { return };
        let fit = crate::capture::fit_on_screen(r);
        if fit != r {
            crate::diag::log(&format!("fenêtre hors écran {r:?} : replacée en {fit:?}"));
            unsafe {
                let _ = SetWindowPos(self.hwnd(), HWND_NOTOPMOST, fit.x, fit.y, fit.w, fit.h, SWP_NOACTIVATE);
            }
        }
    }

    /// Vrai pendant que l'utilisateur déplace ou redimensionne la fenêtre (boucle modale de Windows).
    pub fn is_moving(self) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::{GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO, GUI_INMOVESIZE};
        if !self.is_valid() {
            return false;
        }
        unsafe {
            let thread = GetWindowThreadProcessId(self.hwnd(), None);
            let mut info = GUITHREADINFO { cbSize: std::mem::size_of::<GUITHREADINFO>() as u32, ..Default::default() };
            GetGUIThreadInfo(thread, &mut info).is_ok() && info.flags.contains(GUI_INMOVESIZE)
        }
    }

    /// État lisible pour le journal : position, réduite, masquée.
    pub fn describe(self) -> String {
        use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
        let mut cloaked = 0u32;
        unsafe {
            let _ = DwmGetWindowAttribute(self.hwnd(), DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, 4);
        }
        format!("{:?} réduite={} masquée={}", self.rect(), self.is_minimized(), cloaked != 0)
    }
}

/// Taille voulue de la fenêtre principale, en points (zone cliente), tenue à jour par l'application.
static WANTED_POINTS: std::sync::Mutex<Option<(f32, f32)>> = std::sync::Mutex::new(None);

/// Mémorise la taille voulue de la fenêtre principale (points), appliquée à chaque changement d'écran.
pub fn set_wanted_size(points: (f32, f32)) {
    if let Ok(mut w) = WANTED_POINTS.lock() {
        *w = Some(points);
    }
}

/// Corrige le passage d'un écran à un autre d'échelle différente (100 % → 125 %…). winit recalcule
/// lui-même la taille puis pousse la fenêtre vers le nouvel écran ; selon la disposition des écrans,
/// la fenêtre bascule alors d'un écran à l'autre et rétrécit ou grossit en cascade pendant le
/// glisser. On laisse winit traiter le message (nouvelle échelle pour l'interface) puis on impose la
/// taille voulue (points × nouvelle échelle), en gardant la fenêtre sous le pointeur.
pub fn fix_dpi_changes(win: Window) {
    use windows::Win32::UI::Shell::SetWindowSubclass;
    if win.is_valid() {
        unsafe {
            let _ = SetWindowSubclass(win.hwnd(), Some(dpi_subclass), 1, 0);
        }
    }
}

unsafe extern "system" fn dpi_subclass(
    hwnd: HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    _data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::HiDpi::AdjustWindowRectExForDpi;
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, IsZoomed, GWL_STYLE, SWP_NOZORDER, WINDOW_EX_STYLE, WINDOW_STYLE, WM_DPICHANGED,
    };
    if msg != WM_DPICHANGED {
        return DefSubclassProc(hwnd, msg, wparam, lparam);
    }
    let suggested = *(lparam.0 as *const RECT);
    let result = DefSubclassProc(hwnd, msg, wparam, lparam);
    let wanted = WANTED_POINTS.lock().ok().and_then(|w| *w);
    let Some((pw, ph)) = wanted.filter(|_| !IsZoomed(hwnd).as_bool()) else { return result };
    let dpi = (wparam.0 & 0xFFFF) as u32;
    let scale = dpi as f32 / 96.0;
    // Zone cliente voulue à la nouvelle échelle, plus le cadre de la fenêtre à cette échelle.
    let mut r = RECT { left: 0, top: 0, right: (pw * scale).round() as i32, bottom: (ph * scale).round() as i32 };
    let style = WINDOW_STYLE(GetWindowLongPtrW(hwnd, GWL_STYLE) as u32);
    let ex = WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32);
    let _ = AdjustWindowRectExForDpi(&mut r, style, false, ex, dpi);
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    // Garder le pointeur à la même proportion horizontale de la barre de titre (glisser en cours).
    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);
    let sw = (suggested.right - suggested.left).max(1);
    let ratio = ((cursor.x - suggested.left) as f32 / sw as f32).clamp(0.0, 1.0);
    let x = if cursor.x >= suggested.left && cursor.x <= suggested.right { cursor.x - (ratio * w as f32) as i32 } else { suggested.left };
    let _ = SetWindowPos(hwnd, None, x, suggested.top, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
    result
}

/// Échelle (pixels par point) de l'écran contenant le point (pixels physiques).
pub fn scale_at(x: i32, y: i32) -> f32 {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    unsafe {
        let mon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let (mut dx, mut dy) = (96u32, 96u32);
        if GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy).is_ok() {
            dx as f32 / 96.0
        } else {
            1.0
        }
    }
}

static PRINTSCREEN_HANDLER: std::sync::OnceLock<PrintScreenHandler> = std::sync::OnceLock::new();
static PRINTSCREEN_DOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SNIP_SWALLOWED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static ESC_SWALLOWED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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
        // Échap pendant une capture : l'annuler (et ne pas transmettre la touche à l'application
        // au premier plan). Le reste du temps, Échap n'est pas touché.
        if kb.vkCode == 0x1B {
            let msg = wparam.0 as u32;
            let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            if down && super::CAPTURE_ACTIVE.load(Ordering::SeqCst) {
                if let Some(h) = PRINTSCREEN_HANDLER.get() {
                    if h(KEY_ESCAPE) {
                        ESC_SWALLOWED.store(true, Ordering::SeqCst);
                        return LRESULT(1);
                    }
                }
            } else if !down && ESC_SWALLOWED.swap(false, Ordering::SeqCst) {
                return LRESULT(1);
            }
        }
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
        // Instance lancée par l'utilisateur (menu Démarrer) : céder le premier plan à l'autre.
        let _ = windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(windows::Win32::UI::WindowsAndMessaging::ASFW_ANY);
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
/// `f` reçoit la commande transmise par `send_command`, ou `None` pour un simple affichage.
pub fn activation_listener(f: impl Fn(Option<String>) + Send + 'static) {
    use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject, INFINITE};
    let Ok(ev) = (unsafe { CreateEventW(None, false, false, ACTIVATE_EVENT) }) else { return };
    let ev = ev.0 as isize;
    let _ = std::thread::Builder::new().name("activation".into()).spawn(move || loop {
        let h = windows::Win32::Foundation::HANDLE(ev as *mut _);
        if unsafe { WaitForSingleObject(h, INFINITE) } != windows::Win32::Foundation::WAIT_OBJECT_0 {
            return;
        }
        let cmd = std::fs::read_to_string(command_file()).ok().map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
        let _ = std::fs::remove_file(command_file());
        f(cmd);
    });
}

fn command_file() -> std::path::PathBuf {
    std::env::temp_dir().join("ScreenCapture.Rust.command")
}

/// Transmet une commande à l'instance en cours (`ScreenCapture.exe --cmd region`…).
/// Renvoie `false` si aucune instance ne tourne.
pub fn send_command(cmd: &str) -> bool {
    use windows::Win32::System::Threading::{OpenEventW, SetEvent, EVENT_MODIFY_STATE};
    use windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY};
    let Ok(ev) = (unsafe { OpenEventW(EVENT_MODIFY_STATE, false, ACTIVATE_EVENT) }) else { return false };
    if std::fs::write(command_file(), cmd).is_err() {
        return false;
    }
    unsafe {
        // Cette instance vient d'être lancée par l'utilisateur : lui céder le premier plan.
        let _ = AllowSetForegroundWindow(ASFW_ANY);
        SetEvent(ev).is_ok()
    }
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
    anyhow::ensure!(r == 0, "{}", crate::i18n::trf("suppression impossible (code {0})", &[&r]));
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

/// Touche V enfoncée (pour Ctrl+V : egui n'émet aucun évènement quand le presse-papiers
/// contient une image et pas de texte).
pub fn is_v_key_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    unsafe { GetAsyncKeyState(0x56) as u16 & 0x8000 != 0 }
}

/// Lancement au démarrage de Windows : valeur « ScreenCapture » de la clé Run de l'utilisateur,
/// ou raccourci posé par l'installeur dans le dossier Démarrage (utilisateur ou tous les utilisateurs).
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "ScreenCapture";

fn startup_shortcuts() -> Vec<std::path::PathBuf> {
    let mut v = Vec::new();
    for (var, sub) in [("APPDATA", r"Microsoft\Windows\Start Menu\Programs\Startup"), ("ProgramData", r"Microsoft\Windows\Start Menu\Programs\StartUp")] {
        if let Some(dir) = std::env::var_os(var) {
            v.push(std::path::Path::new(&dir).join(sub).join("ScreenCapture.lnk"));
        }
    }
    v
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn run_value_exists(name: &str) -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
    let (key, name) = (wide(RUN_KEY), wide(name));
    unsafe { RegGetValueW(HKEY_CURRENT_USER, PCWSTR(key.as_ptr()), PCWSTR(name.as_ptr()), RRF_RT_REG_SZ, None, None, None) }.is_ok()
}

fn set_run_value(name: &str, command: &str) -> anyhow::Result<()> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ};
    let (key, name, cmd) = (wide(RUN_KEY), wide(name), wide(command));
    unsafe {
        RegSetKeyValueW(HKEY_CURRENT_USER, PCWSTR(key.as_ptr()), PCWSTR(name.as_ptr()), REG_SZ.0, Some(cmd.as_ptr() as *const _), (cmd.len() * 2) as u32)
    }
    .ok()?;
    Ok(())
}

fn delete_run_value(name: &str) -> anyhow::Result<()> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{RegDeleteKeyValueW, HKEY_CURRENT_USER};
    let (key, name) = (wide(RUN_KEY), wide(name));
    unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, PCWSTR(key.as_ptr()), PCWSTR(name.as_ptr())) }.ok()?;
    Ok(())
}

pub fn launch_at_startup() -> bool {
    run_value_exists(RUN_VALUE) || startup_shortcuts().iter().any(|p| p.exists())
}

pub fn set_launch_at_startup(on: bool) -> anyhow::Result<()> {
    if on {
        if !launch_at_startup() {
            let exe = std::env::current_exe()?;
            set_run_value(RUN_VALUE, &format!("\"{}\" --tray", exe.display()))?;
        }
        return Ok(());
    }
    if run_value_exists(RUN_VALUE) {
        delete_run_value(RUN_VALUE)?;
    }
    for p in startup_shortcuts() {
        if p.exists() {
            std::fs::remove_file(&p).map_err(|_| {
                anyhow::anyhow!(crate::i18n::tr("Le lancement au démarrage a été installé pour tous les utilisateurs : seul un administrateur peut le retirer."))
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod startup_tests {
    use super::*;

    /// Écriture, lecture et suppression d'une valeur de la clé Run (nom de test, sans toucher
    /// au réglage réel de l'utilisateur).
    #[test]
    fn valeur_run() {
        let name = "ScreenCapture-test-unitaire";
        let _ = delete_run_value(name);
        assert!(!run_value_exists(name));
        set_run_value(name, "\"C:\\x.exe\" --tray").unwrap();
        assert!(run_value_exists(name));
        delete_run_value(name).unwrap();
        assert!(!run_value_exists(name));
    }
}

/// Capture défilante : Windows accepte toujours la molette simulée.
pub fn can_simulate_scroll() -> bool {
    true
}

pub fn move_cursor(x: i32, y: i32) {
    let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::SetCursorPos(x, y) };
}

/// Bouton gauche de la souris enfoncé (état physique, quelle que soit la fenêtre).
pub fn left_button_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000 != 0 }
}

/// Crans de molette vers le bas, là où se trouve le pointeur.
pub fn send_wheel(notches: i32) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_WHEEL, MOUSEINPUT};
    for _ in 0..notches.clamp(0, 10) {
        let input = INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: (-120i32) as u32, dwFlags: MOUSEEVENTF_WHEEL, time: 0, dwExtraInfo: 0 } } };
        unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
}

/// Fait défiler vers le bas la fenêtre sous le point (x, y) de `notches` crans de molette.
pub fn scroll_wheel(x: i32, y: i32, notches: i32) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEEVENTF_WHEEL, MOUSEINPUT, MOUSE_EVENT_FLAGS};
    let send = |dx: i32, data: i32, flags: MOUSE_EVENT_FLAGS| {
        let input = INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dx, dy: 0, mouseData: data as u32, dwFlags: flags, time: 0, dwExtraInfo: 0 } } };
        unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    };
    move_cursor(x, y);
    // Petit mouvement réel : certaines applications ne défilent que sous un pointeur « survolant ».
    send(1, 0, MOUSEEVENTF_MOVE);
    send(-1, 0, MOUSEEVENTF_MOVE);
    std::thread::sleep(std::time::Duration::from_millis(30));
    for _ in 0..notches.max(1) {
        send(0, -120, MOUSEEVENTF_WHEEL);
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
}
