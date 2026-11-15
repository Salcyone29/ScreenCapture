//! Campagne de validation de la gestion des fenêtres (Windows).
//!
//! Lance ScreenCapture (version compilée), le pilote par `--cmd`, le clavier et la souris, puis
//! vérifie l'état réel des fenêtres (position, taille, réduite, masquée, bouton dans la barre des
//! tâches, réactivité) et le journal. Les réglages sont sauvegardés puis restaurés ; les captures
//! de test vont dans un dossier temporaire. Un film (Desktop Duplication, 60 i/s) sert à vérifier
//! qu'aucune fenêtre n'apparaît furtivement.
//!
//! Usage : `cargo run --release --example validate -- [dossier de sortie] [filtre]`
//! Les touches ne sont envoyées que si une fenêtre de ScreenCapture est au premier plan.

#[cfg(not(windows))]
fn main() {
    eprintln!("La campagne de validation des fenêtres ne fonctionne que sous Windows.");
}

#[cfg(windows)]
fn main() {
    win::main();
}

#[cfg(windows)]
mod win {
    use std::fmt::Write as _;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{Duration, Instant};

    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, WPARAM};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, EnumDisplayMonitors, GetDC, GetDIBits,
        GetMonitorInfoW, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, HMONITOR,
        MONITORINFO, SRCCOPY,
    };
    use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
    use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, mouse_event, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, MOUSE_EVENT_FLAGS};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, GetWindowTextW,
        GetWindowThreadProcessId, IsHungAppWindow, IsIconic, IsWindowVisible, IsZoomed, PostMessageW, SetCursorPos,
        SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOP, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SWP_NOSIZE, SW_MAXIMIZE, SW_MINIMIZE, WM_CLOSE, WS_EX_TOOLWINDOW,
    };

    // ------------------------------------------------------------------ fenêtres

    #[derive(Clone, Debug)]
    pub struct Win {
        pub hwnd: isize,
        pub title: String,
        pub r: (i32, i32, i32, i32),
        pub minimized: bool,
        pub maximized: bool,
        pub cloaked: bool,
        pub taskbar: bool,
        pub hung: bool,
    }

    impl Win {
        fn on_screen(&self) -> bool {
            !self.minimized && !self.cloaked
        }
    }

    fn title_of(h: HWND) -> String {
        let mut buf = [0u16; 256];
        let n = unsafe { GetWindowTextW(h, &mut buf) };
        String::from_utf16_lossy(&buf[..n as usize])
    }

    fn app_windows(pid: u32) -> Vec<Win> {
        unsafe extern "system" fn cb(h: HWND, l: LPARAM) -> BOOL {
            let v = &mut *(l.0 as *mut Vec<HWND>);
            v.push(h);
            BOOL(1)
        }
        let mut all: Vec<HWND> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(cb), LPARAM(&mut all as *mut _ as isize));
        }
        all.into_iter()
            .filter_map(|h| unsafe {
                let mut p = 0u32;
                GetWindowThreadProcessId(h, Some(&mut p));
                if p != pid || !IsWindowVisible(h).as_bool() {
                    return None;
                }
                let title = title_of(h);
                if !title.contains("ScreenCapture") {
                    return None;
                }
                let mut r = RECT::default();
                let _ = GetWindowRect(h, &mut r);
                let mut cloaked = 0u32;
                let _ = DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, 4);
                let ex = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
                Some(Win {
                    hwnd: h.0 as isize,
                    title,
                    r: (r.left, r.top, r.right - r.left, r.bottom - r.top),
                    minimized: IsIconic(h).as_bool(),
                    maximized: IsZoomed(h).as_bool(),
                    cloaked: cloaked != 0,
                    taskbar: ex & WS_EX_TOOLWINDOW.0 == 0,
                    hung: IsHungAppWindow(h).as_bool(),
                })
            })
            .collect()
    }

    fn foreground_title() -> String {
        unsafe { title_of(GetForegroundWindow()) }
    }

    pub fn virtual_screen() -> (i32, i32, i32, i32) {
        unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        }
    }

    /// Écrans : (rectangle, zone de travail).
    pub fn monitors() -> Vec<((i32, i32, i32, i32), (i32, i32, i32, i32))> {
        unsafe extern "system" fn cb(m: HMONITOR, _dc: HDC, _r: *mut RECT, l: LPARAM) -> BOOL {
            let v = &mut *(l.0 as *mut Vec<((i32, i32, i32, i32), (i32, i32, i32, i32))>);
            let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            if GetMonitorInfoW(m, &mut mi).as_bool() {
                let (a, b) = (mi.rcMonitor, mi.rcWork);
                v.push(((a.left, a.top, a.right - a.left, a.bottom - a.top), (b.left, b.top, b.right - b.left, b.bottom - b.top)));
            }
            BOOL(1)
        }
        let mut v = Vec::new();
        unsafe {
            let _ = EnumDisplayMonitors(HDC::default(), None, Some(cb), LPARAM(&mut v as *mut _ as isize));
        }
        v
    }

    fn intersects(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
        a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
    }

    // ------------------------------------------------------------------ clavier, souris

    fn key_raw(vk: u8, mods: &[u8]) {
        unsafe {
            for &m in mods {
                keybd_event(m, 0, KEYBD_EVENT_FLAGS(0), 0);
            }
            keybd_event(vk, 0, KEYBD_EVENT_FLAGS(0), 0);
            std::thread::sleep(Duration::from_millis(60));
            keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
            for &m in mods.iter().rev() {
                keybd_event(m, 0, KEYEVENTF_KEYUP, 0);
            }
        }
        std::thread::sleep(Duration::from_millis(150));
    }

    pub const VK_ESCAPE: u8 = 0x1B;
    pub const VK_SNAPSHOT: u8 = 0x2C;

    fn mouse_to(x: i32, y: i32) {
        unsafe {
            let _ = SetCursorPos(x, y);
            // Vrai mouvement (aller-retour d'un pixel) : SetCursorPos seul n'envoie pas toujours de
            // déplacement à la fenêtre, qui recevrait alors le clic à l'ancienne position.
            mouse_event(MOUSE_EVENT_FLAGS(1), 1, 0, 0, 0);
            mouse_event(MOUSE_EVENT_FLAGS(1), -1, 0, 0, 0);
        }
    }

    fn mouse_button(down: bool) {
        unsafe { mouse_event(MOUSE_EVENT_FLAGS(if down { 2 } else { 4 }), 0, 0, 0, 0) };
    }

    // ------------------------------------------------------------------ capture d'écran (rapport)

    fn screenshot(path: &Path, scale: f32) {
        let (x, y, w, h) = virtual_screen();
        unsafe {
            let screen = GetDC(HWND::default());
            let mem = CreateCompatibleDC(screen);
            let bmp = CreateCompatibleBitmap(screen, w, h);
            let old = SelectObject(mem, bmp);
            let _ = BitBlt(mem, 0, 0, w, h, screen, x, y, SRCCOPY);
            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut buf = vec![0u8; (w * h * 4) as usize];
            SelectObject(mem, old);
            GetDIBits(mem, bmp, 0, h as u32, Some(buf.as_mut_ptr() as *mut _), &mut info, DIB_RGB_COLORS);
            let _ = DeleteObject(bmp);
            let _ = DeleteDC(mem);
            ReleaseDC(HWND::default(), screen);
            for px in buf.chunks_exact_mut(4) {
                px.swap(0, 2);
                px[3] = 255;
            }
            if let Some(img) = image::RgbaImage::from_raw(w as u32, h as u32, buf) {
                let small = image::imageops::resize(&img, (w as f32 * scale) as u32, (h as f32 * scale) as u32, image::imageops::FilterType::Triangle);
                let _ = small.save(path);
            }
        }
    }

    // ------------------------------------------------------------------ banc de test

    pub struct Bench {
        exe: PathBuf,
        out: PathBuf,
        log: PathBuf,
        settings: PathBuf,
        settings_backup: Option<String>,
        captures: PathBuf,
        pid: u32,
        scenario: String,
        shots: Vec<(String, String)>,
        ffmpeg: Option<PathBuf>,
        /// Réglages supplémentaires du prochain démarrage.
        extra: Vec<(String, serde_json::Value)>,
    }

    impl Bench {
        fn new(out: PathBuf) -> Self {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let settings = dirs::config_dir().unwrap().join("ScreenCapture").join("settings.json");
            let captures = std::env::temp_dir().join("ScreenCapture-validation-captures");
            let _ = std::fs::remove_dir_all(&captures);
            std::fs::create_dir_all(&captures).unwrap();
            std::fs::create_dir_all(&out).unwrap();
            let ffmpeg = std::env::var("LOCALAPPDATA")
                .ok()
                .and_then(|l| find_file(&PathBuf::from(l).join("Microsoft").join("WinGet").join("Packages"), "ffmpeg.exe", 6));
            Bench {
                exe: std::env::var_os("SC_VALIDATE_EXE").map(PathBuf::from).unwrap_or_else(|| root.join("target").join("release").join("ScreenCapture.exe")),
                out,
                log: dirs::data_local_dir().unwrap().join("ScreenCapture").join("logs").join("screencapture.log"),
                settings_backup: std::fs::read_to_string(&settings).ok(),
                settings,
                captures,
                pid: 0,
                scenario: String::new(),
                shots: Vec::new(),
                ffmpeg,
                extra: Vec::new(),
            }
        }

        // -------------------------------------------------- réglages et lancement

        /// Écrit les réglages de test (application arrêtée).
        pub fn configure(&self, after_capture: &str, start_in_tray: bool) {
            let mut v: serde_json::Value = self
                .settings_backup
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_else(|| serde_json::json!({}));
            let o = v.as_object_mut().unwrap();
            o.insert("after_capture".into(), after_capture.into());
            o.insert("start_in_tray".into(), start_in_tray.into());
            o.insert("close_to_tray".into(), true.into());
            o.insert("copy_to_clipboard".into(), false.into());
            o.insert("auto_save".into(), true.into());
            o.insert("diagnostic".into(), true.into());
            o.insert("show_magnifier".into(), true.into());
            o.insert("delay_seconds".into(), 3.into());
            o.insert("audio_device".into(), serde_json::Value::Null);
            o.insert("record_audio".into(), true.into());
            o.insert("video_format".into(), "Mp4".into());
            o.insert("retention_images_days".into(), 0.into());
            o.insert("retention_videos_days".into(), 0.into());
            o.insert("retention_max_mb".into(), 0.into());
            o.insert("retention_to_trash".into(), true.into());
            o.insert("pinned".into(), serde_json::json!([]));
            o.insert("check_updates".into(), false.into());
            for (k, v) in &self.extra {
                o.insert(k.clone(), v.clone());
            }
            o.insert("save_dir".into(), self.captures.to_string_lossy().to_string().into());
            std::fs::create_dir_all(self.settings.parent().unwrap()).unwrap();
            std::fs::write(&self.settings, serde_json::to_string_pretty(&v).unwrap()).unwrap();
        }

        pub fn restore_settings(&self) {
            match &self.settings_backup {
                Some(s) => {
                    let _ = std::fs::write(&self.settings, s);
                }
                None => {
                    let _ = std::fs::remove_file(&self.settings);
                }
            }
        }

        pub fn kill(&mut self) {
            let _ = Command::new("taskkill").args(["/F", "/IM", "ScreenCapture.exe"]).output();
            let _ = Command::new("taskkill").args(["/F", "/IM", "dpi_probe.exe"]).output();
            std::thread::sleep(Duration::from_millis(700));
            self.pid = 0;
        }

        /// Démarre ScreenCapture avec les réglages donnés et attend sa fenêtre principale.
        pub fn start(&mut self, after_capture: &str, tray: bool) {
            self.kill();
            self.configure(after_capture, tray);
            let child = Command::new(&self.exe).spawn().expect("lancement de ScreenCapture");
            self.pid = child.id();
            let t = Instant::now();
            while t.elapsed() < Duration::from_secs(8) && self.main().is_none() {
                std::thread::sleep(Duration::from_millis(100));
            }
            std::thread::sleep(Duration::from_millis(800));
        }

        pub fn cmd(&self, c: &str, wait_ms: u64) {
            let _ = Command::new(&self.exe).args(["--cmd", c]).status();
            std::thread::sleep(Duration::from_millis(wait_ms));
        }

        // -------------------------------------------------- observation

        pub fn windows(&self) -> Vec<Win> {
            app_windows(self.pid)
        }

        pub fn main(&self) -> Option<Win> {
            self.windows().into_iter().find(|w| w.title == "ScreenCapture" || w.title.ends_with(" — ScreenCapture"))
        }

        pub fn floating(&self, name: &str) -> Option<Win> {
            let t = format!("ScreenCapture — {name}");
            self.windows().into_iter().find(|w| w.title == t)
        }

        /// Attend qu'une fenêtre flottante soit visible à l'écran.
        pub fn wait_floating(&self, name: &str, ms: u64) -> Option<Win> {
            let t = Instant::now();
            while t.elapsed() < Duration::from_millis(ms) {
                if let Some(w) = self.floating(name).filter(|w| w.on_screen()) {
                    return Some(w);
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            self.floating(name)
        }

        pub fn wait_gone(&self, name: &str, ms: u64) -> bool {
            let t = Instant::now();
            while t.elapsed() < Duration::from_millis(ms) {
                if self.floating(name).is_none() {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            false
        }

        pub fn foreground_is_app(&self) -> bool {
            foreground_title().contains("ScreenCapture")
        }

        pub fn log_mark(&self) -> u64 {
            std::fs::metadata(&self.log).map(|m| m.len()).unwrap_or(0)
        }

        pub fn log_since(&self, mark: u64) -> String {
            let data = std::fs::read(&self.log).unwrap_or_default();
            String::from_utf8_lossy(&data[(mark as usize).min(data.len())..]).to_string()
        }

        /// Interface réactive : une commande `state` est traitée en moins de `ms`.
        pub fn responsive(&self, ms: u64) -> bool {
            let mark = self.log_mark();
            self.cmd("state", 0);
            let t = Instant::now();
            while t.elapsed() < Duration::from_millis(ms) {
                if self.log_since(mark).contains("état :") {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            false
        }

        pub fn captures_count(&self) -> usize {
            std::fs::read_dir(&self.captures).map(|r| r.count()).unwrap_or(0)
        }

        // -------------------------------------------------- actions

        /// Touche, uniquement si une fenêtre de ScreenCapture est au premier plan.
        pub fn key(&self, vk: u8, mods: &[u8]) -> bool {
            if !self.foreground_is_app() {
                self.check("touche envoyée à ScreenCapture", false, &format!("premier plan : « {} »", foreground_title()));
                return false;
            }
            key_raw(vk, mods);
            true
        }

        /// Impr. écran : touche globale, interceptée par ScreenCapture quel que soit le premier plan.
        pub fn print_screen(&self) {
            key_raw(VK_SNAPSHOT, &[]);
        }

        /// Glisser pour sélectionner une zone (sur la sélection plein écran).
        pub fn drag(&self, x1: i32, y1: i32, x2: i32, y2: i32) {
            mouse_to(x1, y1);
            std::thread::sleep(Duration::from_millis(150));
            mouse_button(true);
            std::thread::sleep(Duration::from_millis(80));
            for i in 1..=10 {
                mouse_to(x1 + (x2 - x1) * i / 10, y1 + (y2 - y1) * i / 10);
                std::thread::sleep(Duration::from_millis(25));
            }
            std::thread::sleep(Duration::from_millis(80));
            mouse_button(false);
            std::thread::sleep(Duration::from_millis(300));
        }

        pub fn minimize_main(&self) {
            if let Some(m) = self.main() {
                unsafe {
                    let _ = ShowWindow(HWND(m.hwnd as *mut _), SW_MINIMIZE);
                }
                std::thread::sleep(Duration::from_millis(600));
            }
        }

        pub fn close_main(&self) {
            if let Some(m) = self.main() {
                unsafe {
                    let _ = PostMessageW(HWND(m.hwnd as *mut _), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
                std::thread::sleep(Duration::from_millis(700));
            }
        }

        /// Déplace l'espace de travail comme l'utilisateur : glisser la barre de titre à la souris
        /// jusqu'au point `(x, y)` (le point saisi de la barre de titre y arrive).
        pub fn drag_title_to(&self, x: i32, y: i32) {
            let Some(m) = self.main() else { return };
            let (gx, gy) = (m.r.0 + m.r.2 / 3, m.r.1 + 14);
            mouse_to(gx, gy);
            std::thread::sleep(Duration::from_millis(200));
            mouse_button(true);
            std::thread::sleep(Duration::from_millis(150));
            for i in 1..=30 {
                mouse_to(gx + (x - gx) * i / 30, gy + (y - gy) * i / 30);
                std::thread::sleep(Duration::from_millis(30));
            }
            std::thread::sleep(Duration::from_millis(250));
            mouse_button(false);
            std::thread::sleep(Duration::from_millis(800));
        }

        pub fn maximize_main(&self) {
            if let Some(m) = self.main() {
                unsafe {
                    let _ = ShowWindow(HWND(m.hwnd as *mut _), SW_MAXIMIZE);
                }
                std::thread::sleep(Duration::from_millis(800));
            }
        }

        /// Dernier fichier enregistré dans le dossier des captures de test.
        pub fn newest_capture(&self) -> Option<PathBuf> {
            std::fs::read_dir(&self.captures)
                .ok()?
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "png"))
                .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
                .map(|e| e.path())
        }

        /// Capture de zone pilotée : sélection de `sel` (pixels physiques). Vérifie la taille du
        /// fichier enregistré et que son contenu correspond à l'écran (`reference`, pris avant).
        pub fn capture_zone_and_check(&self, label: &str, sel: (i32, i32, i32, i32), reference: &image::RgbaImage) {
            let n = self.captures_count();
            self.cmd("region", 300);
            if self.wait_floating("Sélection", 3000).is_none() {
                self.check(&format!("{label} : sélection affichée"), false, "");
                return;
            }
            self.drag(sel.0, sel.1, sel.0 + sel.2, sel.1 + sel.3);
            let t = Instant::now();
            while self.captures_count() == n && t.elapsed() < Duration::from_secs(4) {
                std::thread::sleep(Duration::from_millis(100));
            }
            std::thread::sleep(Duration::from_millis(300));
            match self.newest_capture().and_then(|p| image::open(&p).ok()).map(|i| i.to_rgba8()) {
                Some(img) => {
                    let (w, h) = (img.width() as i32, img.height() as i32);
                    self.check(&format!("{label} : taille de l'image = zone sélectionnée"), (w - sel.2).abs() <= 1 && (h - sel.3).abs() <= 1, &format!("{w}×{h} pour {}×{}", sel.2, sel.3));
                    let sim = similarity(&image::imageops::resize(&img, 64, 48, image::imageops::FilterType::Triangle), &image::imageops::resize(reference, 64, 48, image::imageops::FilterType::Triangle));
                    self.check(&format!("{label} : contenu identique à l'écran"), sim > 0.95, &format!("ressemblance {:.1} %", sim * 100.0));
                }
                None => self.check(&format!("{label} : capture enregistrée"), false, ""),
            }
        }

        /// Clic gauche en (x, y), touches de modification maintenues (Ctrl, Maj).
        pub fn click(&self, x: i32, y: i32, mods: &[u8]) {
            mouse_to(x, y);
            std::thread::sleep(Duration::from_millis(150));
            unsafe {
                for &m in mods {
                    keybd_event(m, 0, KEYBD_EVENT_FLAGS(0), 0);
                }
            }
            std::thread::sleep(Duration::from_millis(60));
            mouse_button(true);
            std::thread::sleep(Duration::from_millis(60));
            mouse_button(false);
            std::thread::sleep(Duration::from_millis(60));
            unsafe {
                for &m in mods.iter().rev() {
                    keybd_event(m, 0, KEYEVENTF_KEYUP, 0);
                }
            }
            std::thread::sleep(Duration::from_millis(300));
        }

        /// Vignettes de l'espace de travail : (nom du fichier, centre en pixels d'écran).
        pub fn cards(&self) -> Vec<(String, i32, i32)> {
            let mark = self.log_mark();
            self.cmd("cards", 700);
            self.log_since(mark)
                .lines()
                .filter_map(|l| {
                    let rest = l.split("carte ").nth(1)?;
                    let mut it = rest.splitn(6, ' ');
                    let _i = it.next()?;
                    let (x, y, w, h): (f32, f32, f32, f32) = (it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?);
                    Some((it.next()?.trim().to_string(), (x + w / 2.0) as i32, (y + h / 3.0) as i32))
                })
                .collect()
        }

        /// Nombre de captures sélectionnées dans l'espace de travail.
        pub fn selected_count(&self) -> Option<usize> {
            let mark = self.log_mark();
            self.cmd("state", 600);
            let log = self.log_since(mark);
            let rest = log.split("sélectionnées=").nth(1)?;
            rest.split_whitespace().next()?.parse().ok()
        }

        pub fn capture_files(&self, ext: &str) -> Vec<String> {
            let mut v: Vec<String> = std::fs::read_dir(&self.captures)
                .map(|r| r.flatten().filter(|e| e.path().extension().is_some_and(|x| x == ext)).map(|e| e.file_name().to_string_lossy().to_string()).collect())
                .unwrap_or_default();
            v.sort();
            v
        }

        /// Déplace la fenêtre principale (simulation d'un écran débranché).
        pub fn move_main(&self, x: i32, y: i32) {
            if let Some(m) = self.main() {
                unsafe {
                    let _ = SetWindowPos(HWND(m.hwnd as *mut _), HWND_TOP, x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
                }
                std::thread::sleep(Duration::from_millis(300));
            }
        }

        // -------------------------------------------------- film (apparitions furtives)

        /// Filme l'écran pendant `action` et compte les images où la fenêtre principale apparaît
        /// dans `region` (comparaison avec son apparence de référence).
        pub fn film_main_appearances(
            &self,
            name: &str,
            region: (i32, i32, i32, i32),
            reference: &image::RgbaImage,
            background: &image::RgbaImage,
            secs: u32,
            action: impl FnOnce(),
        ) -> Option<usize> {
            let ffmpeg = self.ffmpeg.clone()?;
            let video = self.out.join(format!("{name}.mp4"));
            let mut rec = Command::new(&ffmpeg)
                .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "ddagrab=framerate=60:draw_mouse=0,hwdownload,format=bgra"])
                .args(["-t", &secs.to_string(), "-c:v", "libx264", "-preset", "ultrafast", "-crf", "20", "-pix_fmt", "yuv420p"])
                .arg(&video)
                .spawn()
                .ok()?;
            std::thread::sleep(Duration::from_millis(800));
            action();
            let _ = rec.wait();
            // Images de la zone de la fenêtre, réduites pour la comparaison.
            let frames = self.out.join(format!("{name}_frames"));
            let _ = std::fs::remove_dir_all(&frames);
            std::fs::create_dir_all(&frames).ok()?;
            let (vx, vy, _, _) = virtual_screen();
            let crop = format!("crop={}:{}:{}:{},scale=160:-2", region.2, region.3, region.0 - vx, region.1 - vy);
            Command::new(&ffmpeg)
                .args(["-hide_banner", "-loglevel", "error", "-i"])
                .arg(&video)
                .args(["-vf", &crop])
                .arg(frames.join("%05d.png"))
                .status()
                .ok()?;
            // Comparaison sur des images très réduites (48 px) : la mise en page compte, pas le
            // détail du contenu (nouvelles vignettes dans l'historique, horloge…).
            let h = (48.0 * region.3 as f32 / region.2 as f32) as u32;
            let small = |i: &image::RgbaImage| image::imageops::resize(i, 48, h, image::imageops::FilterType::Triangle);
            let (reference, background) = (small(reference), small(background));
            let mut hits = 0;
            let mut paths: Vec<_> = std::fs::read_dir(&frames).ok()?.flatten().map(|e| e.path()).collect();
            paths.sort();
            for p in &paths {
                let Ok(img) = image::open(p) else { continue };
                let img = small(&img.to_rgba8());
                // La fenêtre principale est visible si l'image lui ressemble nettement plus qu'au bureau.
                let (sm, sb) = (similarity(&img, &reference), similarity(&img, &background));
                if sm > 0.92 && sm > sb + 0.015 {
                    hits += 1;
                }
            }
            let _ = std::fs::remove_dir_all(&frames);
            Some(hits)
        }

        // -------------------------------------------------- résultats

        pub fn scenario(&mut self, id: &str, title: &str) {
            self.scenario = format!("{id} — {title}");
            println!("\n=== {} ===", self.scenario);
        }

        pub fn check(&self, what: &str, ok: bool, detail: &str) {
            println!("  [{}] {what} {detail}", if ok { "OK" } else { "ÉCHEC" });
            // Les résultats sont accumulés via une cellule intérieure : voir `record`.
            RESULTS.with(|r| r.borrow_mut().push((self.scenario.clone(), what.to_string(), ok, detail.to_string())));
        }

        pub fn shot(&mut self, name: &str) {
            let file = format!("{name}.png");
            screenshot(&self.out.join(&file), 0.25);
            self.shots.push((self.scenario.clone(), file));
        }

        pub fn report(&self, started: chrono::DateTime<chrono::Local>, env: &str) -> PathBuf {
            let results = RESULTS.with(|r| r.borrow().clone());
            let ok = results.iter().filter(|r| r.2).count();
            let mut md = String::new();
            let _ = writeln!(md, "# Campagne de validation — fenêtres et miniatures\n");
            let _ = writeln!(md, "- Date : {}", started.format("%d/%m/%Y %H:%M"));
            let _ = writeln!(md, "- Durée : {} s", (chrono::Local::now() - started).num_seconds());
            let _ = writeln!(md, "- Environnement : {env}");
            let _ = writeln!(md, "- Résultat : **{ok} / {} vérifications réussies**\n", results.len());
            let mut last = String::new();
            for (sc, what, pass, detail) in &results {
                if *sc != last {
                    let _ = writeln!(md, "\n## {sc}\n\n| Vérification | Résultat | Détail |\n|---|---|---|");
                    last = sc.clone();
                }
                let _ = writeln!(md, "| {what} | {} | {} |", if *pass { "✅" } else { "❌" }, detail.replace('|', "/"));
            }
            if !self.shots.is_empty() {
                let _ = writeln!(md, "\n## Captures d'écran\n");
                for (sc, f) in &self.shots {
                    let _ = writeln!(md, "- {sc} : `{f}`");
                }
            }
            let path = self.out.join("RAPPORT.md");
            std::fs::write(&path, md).unwrap();
            path
        }
    }

    thread_local! {
        static RESULTS: std::cell::RefCell<Vec<(String, String, bool, String)>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    fn similarity(a: &image::RgbaImage, b: &image::RgbaImage) -> f32 {
        let (w, h) = (a.width().min(b.width()), a.height().min(b.height()));
        if w == 0 || h == 0 {
            return 0.0;
        }
        let mut diff = 0u64;
        let mut n = 0u64;
        for y in 0..h {
            for x in 0..w {
                let (p, q) = (a.get_pixel(x, y), b.get_pixel(x, y));
                diff += (0..3).map(|i| (p[i] as i32 - q[i] as i32).unsigned_abs() as u64).sum::<u64>();
                n += 3;
            }
        }
        1.0 - diff as f32 / (n as f32 * 255.0)
    }

    fn find_file(dir: &Path, name: &str, depth: u32) -> Option<PathBuf> {
        if depth == 0 {
            return None;
        }
        for e in std::fs::read_dir(dir).ok()?.flatten() {
            let p = e.path();
            if p.is_file() && p.file_name().is_some_and(|n| n.eq_ignore_ascii_case(name)) {
                return Some(p);
            }
            if p.is_dir() {
                if let Some(f) = find_file(&p, name, depth - 1) {
                    return Some(f);
                }
            }
        }
        None
    }

    fn same_rect(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
        (a.0 - b.0).abs() <= 2 && (a.1 - b.1).abs() <= 2 && (a.2 - b.2).abs() <= 2 && (a.3 - b.3).abs() <= 2
    }

    fn fmt(r: (i32, i32, i32, i32)) -> String {
        format!("({}, {}) {}×{}", r.0, r.1, r.2, r.3)
    }

    /// Image de la fenêtre principale à l'écran (référence pour le film).
    fn grab_region(r: (i32, i32, i32, i32)) -> image::RgbaImage {
        let tmp = std::env::temp_dir().join("sc_validation_full.png");
        screenshot(&tmp, 1.0);
        let full = image::open(&tmp).unwrap().to_rgba8();
        let (vx, vy, _, _) = virtual_screen();
        image::imageops::crop_imm(&full, (r.0 - vx).max(0) as u32, (r.1 - vy).max(0) as u32, r.2 as u32, r.3 as u32).to_image()
    }

    // ------------------------------------------------------------------ scénarios

    pub fn main() {
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        let args: Vec<String> = std::env::args().collect();
        let out = args.get(1).map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("validation").join(chrono::Local::now().format("run-%Y-%m-%d_%H-%M").to_string())
        });
        let filter = args.get(2).cloned().unwrap_or_default();
        let started = chrono::Local::now();
        let mut b = Bench::new(out);
        let mons = monitors();
        let env = format!(
            "{} écran(s) : {} ; bureau virtuel {}",
            mons.len(),
            mons.iter().map(|(m, _)| fmt(*m)).collect::<Vec<_>>().join(", "),
            fmt(virtual_screen())
        );
        println!("{env}");
        // Les traces de diagnostic (D…) et les captures de l'aide (H…) ne tournent que si on les demande.
        let run = |id: &str| if filter.is_empty() { !id.starts_with('D') && !id.starts_with('H') } else { filter.split(',').any(|f| id.starts_with(f)) };

        let scenarios: Vec<(&str, fn(&mut Bench))> = vec![
            ("S01", s01_demarrage),
            ("S02", s02_demarrage_zone_notification),
            ("S03", s03_capture_miniature),
            ("S04", s04_annulation),
            ("S05", s05_depuis_zone_notification),
            ("S06", s06_fenetre_reduite),
            ("S07", s07_captures_rapprochees),
            ("S08", s08_editeur),
            ("S09", s09_copier_puis_capture),
            ("S10", s10_plein_ecran),
            ("S11", s11_ouverture_pendant_selection),
            ("S12", s12_fermeture_croix),
            ("S13", s13_impr_ecran_pendant_selection),
            ("S14", s14_ecran_debranche),
            ("S15", s15_endurance),
            ("S16", s16_film_fenetre_reduite),
            ("S17", s17_video),
            ("S18", s18_multi_ecran),
            ("S19", s19_deplacement_second_ecran),
            ("S20", s20_agrandie_second_ecran),
            ("S21", s21_plein_ecran_multi),
            ("S22", s22_echap),
            ("S23", s23_taille_minimale),
            ("S24", s24_selection_multiple),
            ("S25", s25_video_son),
            ("S26", s26_micro_pendant_enregistrement),
            ("S27", s27_bandeau_aide),
            ("S28", s28_conservation),
            ("S29", s29_langues),
            ("S30", s30_editeur_dupliquer_modifier),
            ("S31", s31_selection_par_case),
            ("H1", h1_aide),
            ("D1", d1_trace_deplacement),
            ("D2", d2_trace_retour),
        ];
        for (id, f) in scenarios {
            if run(id) {
                f(&mut b);
            }
        }
        b.kill();
        b.restore_settings();
        let _ = std::fs::remove_dir_all(&b.captures);
        let path = b.report(started, &env);
        println!("\nRapport : {}", path.display());
    }

    /// Point de sélection : centre de l'écran principal (ou du premier écran).
    fn sel_area() -> (i32, i32, i32, i32) {
        let mons = monitors();
        let (m, _) = mons.iter().find(|(m, _)| m.0 == 0 && m.1 == 0).or(mons.first()).copied().unwrap();
        let (cx, cy) = (m.0 + m.2 / 2, m.1 + m.3 / 2);
        (cx - 200, cy - 120, cx + 200, cy + 120)
    }

    fn check_overlay(b: &Bench) -> bool {
        let vs = virtual_screen();
        match b.wait_floating("Sélection", 3000) {
            Some(o) => {
                b.check("sélection visible", o.on_screen(), "");
                b.check("sélection couvre tous les écrans", same_rect(o.r, vs), &format!("{} (bureau {})", fmt(o.r), fmt(vs)));
                b.check("sélection sans bouton dans la barre des tâches", !o.taskbar, "");
                std::thread::sleep(Duration::from_millis(200));
                b.check("sélection au premier plan", b.foreground_is_app(), &foreground_title());
                true
            }
            None => {
                b.check("sélection visible", false, "aucune fenêtre de sélection");
                false
            }
        }
    }

    fn check_main_same(b: &Bench, before: &Win, label: &str) {
        match b.main() {
            Some(m) => {
                b.check(&format!("{label} : même position et taille"), same_rect(m.r, before.r), &format!("{} (avant {})", fmt(m.r), fmt(before.r)));
            }
            None => b.check(&format!("{label} : fenêtre principale trouvée"), false, ""),
        }
    }

    fn s01_demarrage(b: &mut Bench) {
        b.scenario("S01", "Démarrage normal");
        b.start("ShowWorkspace", false);
        let m = b.main();
        b.check("fenêtre principale présente", m.is_some(), "");
        if let Some(m) = m {
            b.check("affichée à l'écran", m.on_screen(), &fmt(m.r));
            b.check("bouton dans la barre des tâches", m.taskbar, "");
            b.check("sur un écran branché", monitors().iter().any(|(r, _)| intersects(*r, m.r)), "");
        }
        b.check("aucune fenêtre flottante", b.floating("Sélection").is_none() && b.floating("Miniature").is_none(), "");
        b.check("interface réactive", b.responsive(3000), "");
    }

    fn s02_demarrage_zone_notification(b: &mut Bench) {
        b.scenario("S02", "Démarrage dans la zone de notification puis ouverture");
        b.start("ShowWorkspace", true);
        let m = b.main();
        b.check("fenêtre principale masquée", m.as_ref().is_some_and(|m| m.cloaked), "");
        b.check("pas de bouton dans la barre des tâches", m.as_ref().is_some_and(|m| !m.taskbar), "");
        b.check("interface réactive (masquée)", b.responsive(3000), "");
        b.cmd("show", 800);
        let m2 = b.main();
        b.check("affichée après ouverture", m2.as_ref().is_some_and(|w| w.on_screen()), "");
        b.check("bouton dans la barre des tâches", m2.as_ref().is_some_and(|w| w.taskbar), "");
        if let (Some(a), Some(c)) = (m, m2) {
            b.check("même position et taille qu'au démarrage", same_rect(a.r, c.r), &fmt(c.r));
        }
    }

    fn s03_capture_miniature(b: &mut Bench) {
        b.scenario("S03", "Capture de zone depuis l'espace de travail affiché (miniature)");
        b.start("ShowWorkspace", false);
        let before = b.main().unwrap();
        b.cmd("region", 300);
        b.check("fenêtre principale masquée pendant la sélection", b.main().is_some_and(|m| m.cloaked), "");
        if check_overlay(b) {
            b.shot("S03_selection");
            let (x1, y1, x2, y2) = sel_area();
            b.drag(x1, y1, x2, y2);
            b.check("sélection fermée", b.wait_gone("Sélection", 2000), "");
            match b.wait_floating("Miniature", 3000) {
                Some(p) => {
                    let (cx, cy) = (x2, y2);
                    let mon = monitors().into_iter().find(|(r, _)| intersects(*r, (cx, cy, 1, 1))).unwrap();
                    let wa = mon.1;
                    b.check("miniature visible", p.on_screen(), &fmt(p.r));
                    b.check("miniature dans la zone de travail", p.r.0 >= wa.0 && p.r.1 >= wa.1 && p.r.0 + p.r.2 <= wa.0 + wa.2 && p.r.1 + p.r.3 <= wa.1 + wa.3, &format!("zone {}", fmt(wa)));
                    b.check("miniature en bas à droite", wa.0 + wa.2 - (p.r.0 + p.r.2) < 40 && wa.1 + wa.3 - (p.r.1 + p.r.3) < 40, "");
                    b.check("miniature de taille raisonnable", p.r.2 >= 300 && p.r.2 <= 900 && p.r.3 >= 250 && p.r.3 <= 760, &format!("{}×{}", p.r.2, p.r.3));
                    b.check("miniature sans bouton dans la barre des tâches", !p.taskbar, "");
                    b.shot("S03_miniature");
                }
                None => b.check("miniature visible", false, "absente"),
            }
            let m = b.main();
            b.check("fenêtre principale réaffichée", m.as_ref().is_some_and(|w| w.on_screen()), "");
            check_main_same(b, &before, "fenêtre principale");
            b.check("capture enregistrée", b.captures_count() >= 1, &format!("{} fichier(s)", b.captures_count()));
            b.check("miniature disparue après 6 s", b.wait_gone("Miniature", 8000), "");
            check_main_same(b, &before, "après la miniature");
        }
        b.check("interface réactive", b.responsive(3000), "");
    }

    fn s04_annulation(b: &mut Bench) {
        b.scenario("S04", "Sélection annulée avec Échap");
        b.start("ShowWorkspace", false);
        let before = b.main().unwrap();
        b.cmd("region", 300);
        if check_overlay(b) {
            b.key(VK_ESCAPE, &[]);
            b.check("sélection fermée", b.wait_gone("Sélection", 2000), "");
            std::thread::sleep(Duration::from_millis(300));
            b.check("fenêtre principale réaffichée", b.main().is_some_and(|w| w.on_screen()), "");
            check_main_same(b, &before, "fenêtre principale");
            b.check("aucune miniature", b.floating("Miniature").is_none(), "");
        }
    }

    fn s05_depuis_zone_notification(b: &mut Bench) {
        b.scenario("S05", "Capture alors que l'application est dans la zone de notification");
        b.start("ShowWorkspace", true);
        let before = b.main().unwrap();
        b.cmd("region", 300);
        if check_overlay(b) {
            let (x1, y1, x2, y2) = sel_area();
            b.drag(x1, y1, x2, y2);
            b.wait_floating("Miniature", 3000);
            b.check("fenêtre principale toujours masquée", b.main().is_some_and(|w| w.cloaked), "");
            b.check("miniature disparue", b.wait_gone("Miniature", 8000), "");
            std::thread::sleep(Duration::from_millis(500));
            let m = b.main();
            b.check("fenêtre principale toujours masquée après la miniature", m.as_ref().is_some_and(|w| w.cloaked), "");
            b.check("toujours sans bouton dans la barre des tâches", m.as_ref().is_some_and(|w| !w.taskbar), "");
            b.cmd("show", 800);
            check_main_same(b, &before, "à la réouverture");
        }
    }

    fn s06_fenetre_reduite(b: &mut Bench) {
        b.scenario("S06", "Capture alors que l'espace de travail est réduit dans la barre des tâches");
        b.start("ShowWorkspace", false);
        let before = b.main().unwrap();
        b.minimize_main();
        b.cmd("region", 300);
        b.check("fenêtre principale toujours réduite pendant la sélection", b.main().is_some_and(|w| w.minimized), "");
        if check_overlay(b) {
            let (x1, y1, x2, y2) = sel_area();
            b.drag(x1, y1, x2, y2);
            b.wait_floating("Miniature", 3000);
            b.check("toujours réduite pendant la miniature", b.main().is_some_and(|w| w.minimized), "");
            b.wait_gone("Miniature", 8000);
            std::thread::sleep(Duration::from_millis(500));
            let m = b.main();
            b.check("toujours réduite après la miniature", m.as_ref().is_some_and(|w| w.minimized), "");
            b.check("bouton toujours dans la barre des tâches", m.as_ref().is_some_and(|w| w.taskbar), "");
            b.cmd("show", 800);
            b.check("restaurée à l'ouverture", b.main().is_some_and(|w| w.on_screen()), "");
            check_main_same(b, &before, "à la restauration");
        }
    }

    fn s07_captures_rapprochees(b: &mut Bench) {
        b.scenario("S07", "Trois captures rapprochées pendant la miniature (fenêtre réduite)");
        b.start("ShowWorkspace", false);
        let before = b.main().unwrap();
        b.minimize_main();
        let (x1, y1, x2, y2) = sel_area();
        for i in 0..3 {
            b.cmd("region", 300);
            if b.wait_floating("Sélection", 3000).is_none() {
                b.check(&format!("sélection n°{}", i + 1), false, "absente");
                return;
            }
            b.drag(x1 + i * 20, y1, x2 + i * 20, y2);
            b.wait_floating("Miniature", 3000);
            b.check(&format!("capture n°{} : fenêtre principale toujours réduite", i + 1), b.main().is_some_and(|w| w.minimized), "");
            let popups = b.windows().into_iter().filter(|w| w.title == "ScreenCapture — Miniature").count();
            b.check(&format!("capture n°{} : une seule miniature", i + 1), popups == 1, &format!("{popups}"));
            std::thread::sleep(Duration::from_millis(400));
        }
        b.wait_gone("Miniature", 8000);
        std::thread::sleep(Duration::from_millis(500));
        b.check("fenêtre principale toujours réduite à la fin", b.main().is_some_and(|w| w.minimized), "");
        b.check("trois captures enregistrées", b.captures_count() >= 3, &format!("{}", b.captures_count()));
        b.cmd("show", 800);
        check_main_same(b, &before, "à la restauration");
    }

    fn s08_editeur(b: &mut Bench) {
        b.scenario("S08", "Capture avec ouverture dans l'éditeur");
        b.start("OpenEditor", false);
        let before = b.main().unwrap();
        b.cmd("region", 300);
        if check_overlay(b) {
            let (x1, y1, x2, y2) = sel_area();
            b.drag(x1, y1, x2, y2);
            std::thread::sleep(Duration::from_millis(800));
            let m = b.main();
            b.check("éditeur affiché", m.as_ref().is_some_and(|w| w.on_screen() && w.title.ends_with(" — ScreenCapture")), &m.as_ref().map(|w| w.title.clone()).unwrap_or_default());
            check_main_same(b, &before, "fenêtre de l'éditeur");
            b.check("au premier plan", b.foreground_is_app(), &foreground_title());
            b.shot("S08_editeur");
        }
    }

    fn s09_copier_puis_capture(b: &mut Bench) {
        b.scenario("S09", "Éditeur : Copier puis nouvelle capture avec Impr. écran (blocage signalé)");
        b.start("OpenEditor", false);
        b.cmd("full", 1500);
        let before = b.main();
        b.check("éditeur ouvert", before.as_ref().is_some_and(|w| w.title.ends_with(" — ScreenCapture")), "");
        let Some(before) = before else { return };
        for i in 1..=5 {
            b.cmd("editor-copy", 500);
            b.print_screen();
            let o = b.wait_floating("Sélection", 3000);
            b.check(&format!("essai {i} : sélection affichée après Copier + Impr. écran"), o.as_ref().is_some_and(|w| w.on_screen()), "");
            b.check(&format!("essai {i} : interface réactive"), b.responsive(3000), "");
            if o.is_none() {
                break;
            }
            b.key(VK_ESCAPE, &[]);
            b.wait_gone("Sélection", 2000);
            std::thread::sleep(Duration::from_millis(400));
            b.check(&format!("essai {i} : éditeur réaffiché"), b.main().is_some_and(|w| w.on_screen()), "");
        }
        check_main_same(b, &before, "fenêtre de l'éditeur");
    }

    fn s10_plein_ecran(b: &mut Bench) {
        b.scenario("S10", "Captures plein écran, fenêtre et dernière zone");
        b.start("ShowWorkspace", false);
        let before = b.main().unwrap();
        for (cmd, label) in [("full", "plein écran"), ("last", "dernière zone"), ("window", "fenêtre")] {
            let n = b.captures_count();
            b.cmd(cmd, 300);
            if b.wait_floating("Sélection", 1200).is_some() {
                // Fenêtre au premier plan = ScreenCapture, ou pas encore de zone précédente : la
                // sélection s'ouvre ; un clic choisit la fenêtre sous le pointeur.
                let (x1, y1, x2, y2) = sel_area();
                b.drag((x1 + x2) / 2, (y1 + y2) / 2, (x1 + x2) / 2, (y1 + y2) / 2);
            }
            let p = b.wait_floating("Miniature", 3000);
            b.check(&format!("{label} : capture enregistrée"), b.captures_count() > n || p.is_some(), &format!("{} fichier(s)", b.captures_count()));
            std::thread::sleep(Duration::from_millis(500));
            b.check(&format!("{label} : fenêtre principale réaffichée"), b.main().is_some_and(|w| w.on_screen()), "");
            check_main_same(b, &before, label);
            b.cmd("cancel", 300);
        }
    }

    fn s11_ouverture_pendant_selection(b: &mut Bench) {
        b.scenario("S11", "Ouverture depuis la zone de notification pendant la sélection");
        b.start("ShowWorkspace", true);
        let before = b.main().unwrap();
        b.cmd("region", 300);
        if check_overlay(b) {
            b.cmd("show", 800);
            b.check("sélection fermée", b.wait_gone("Sélection", 2000), "");
            b.check("fenêtre principale affichée", b.main().is_some_and(|w| w.on_screen()), "");
            check_main_same(b, &before, "fenêtre principale");
            b.cmd("region", 300);
            b.check("nouvelle capture possible ensuite", b.wait_floating("Sélection", 3000).is_some(), "");
            b.key(VK_ESCAPE, &[]);
        }
    }

    fn s12_fermeture_croix(b: &mut Bench) {
        b.scenario("S12", "Fermeture par la croix puis réouverture");
        b.start("ShowWorkspace", false);
        let before = b.main().unwrap();
        b.close_main();
        let m = b.main();
        b.check("masquée (zone de notification)", m.as_ref().is_some_and(|w| w.cloaked), "");
        b.check("plus de bouton dans la barre des tâches", m.as_ref().is_some_and(|w| !w.taskbar), "");
        b.check("processus toujours actif", b.responsive(3000), "");
        b.cmd("show", 800);
        b.check("réaffichée", b.main().is_some_and(|w| w.on_screen()), "");
        check_main_same(b, &before, "à la réouverture");
    }

    fn s13_impr_ecran_pendant_selection(b: &mut Bench) {
        b.scenario("S13", "Impr. écran pendant la sélection");
        b.start("ShowWorkspace", false);
        b.cmd("region", 300);
        if check_overlay(b) {
            b.print_screen();
            std::thread::sleep(Duration::from_millis(500));
            let n = b.windows().into_iter().filter(|w| w.title == "ScreenCapture — Sélection").count();
            b.check("toujours une seule sélection, visible", n == 1 && b.floating("Sélection").is_some_and(|w| w.on_screen()), &format!("{n}"));
            b.check("interface réactive", b.responsive(3000), "");
            b.key(VK_ESCAPE, &[]);
            b.check("sélection fermée", b.wait_gone("Sélection", 2000), "");
        }
    }

    fn s14_ecran_debranche(b: &mut Bench) {
        b.scenario("S14", "Fenêtre restée sur un écran débranché");
        b.start("ShowWorkspace", false);
        b.move_main(20000, 20000);
        b.close_main();
        b.cmd("show", 1000);
        let m = b.main();
        b.check("ramenée sur un écran branché", m.as_ref().is_some_and(|w| monitors().iter().any(|(r, _)| intersects(*r, w.r))), &m.as_ref().map(|w| fmt(w.r)).unwrap_or_default());
        b.check("affichée", m.as_ref().is_some_and(|w| w.on_screen()), "");
    }

    fn s15_endurance(b: &mut Bench) {
        b.scenario("S15", "Endurance : 20 captures et annulations enchaînées");
        b.start("ShowWorkspace", false);
        let before = b.main().unwrap();
        let (x1, y1, x2, y2) = sel_area();
        let mut failures = 0;
        for i in 0..20 {
            b.cmd(if i % 2 == 0 { "region" } else { "full" }, 200);
            if i % 2 == 0 {
                if b.wait_floating("Sélection", 3000).is_none() {
                    failures += 1;
                    continue;
                }
                if i % 4 == 0 {
                    b.drag(x1, y1, x2, y2);
                } else {
                    b.key(VK_ESCAPE, &[]);
                }
            }
            std::thread::sleep(Duration::from_millis(300));
            if b.windows().iter().any(|w| w.hung) {
                failures += 1;
            }
        }
        b.check("aucune sélection manquée ni fenêtre figée", failures == 0, &format!("{failures} échec(s)"));
        b.check("interface réactive", b.responsive(3000), "");
        b.cmd("cancel", 300);
        std::thread::sleep(Duration::from_millis(500));
        check_main_same(b, &before, "fenêtre principale");
        b.check("fenêtre principale affichée", b.main().is_some_and(|w| w.on_screen()), "");
    }

    fn s16_film_fenetre_reduite(b: &mut Bench) {
        b.scenario("S16", "Film : la fenêtre réduite n'apparaît jamais pendant deux captures");
        b.start("ShowWorkspace", false);
        center_main(b);
        let m = b.main().unwrap();
        std::thread::sleep(Duration::from_millis(500));
        let reference = grab_region(m.r);
        b.minimize_main();
        std::thread::sleep(Duration::from_millis(500));
        let background = grab_region(m.r);
        b.check("l'espace de travail se distingue du bureau", similarity(&reference, &background) < 0.97, "");
        let (x1, y1, x2, y2) = sel_area();
        let bench = &*b;
        let hits = bench.film_main_appearances("S16_film", m.r, &reference, &background, 16, || {
            for i in 0..2 {
                bench.cmd("region", 300);
                bench.wait_floating("Sélection", 3000);
                bench.drag(x1 + i * 30, y1, x2 + i * 30, y2);
                std::thread::sleep(Duration::from_millis(900));
            }
            bench.wait_gone("Miniature", 9000);
        });
        match hits {
            Some(h) => b.check("aucune image du film ne montre la fenêtre principale", h == 0, &format!("{h} image(s) sur ~960")),
            None => b.check("film réalisé", false, "FFmpeg indisponible"),
        }
        b.check("toujours réduite", b.main().is_some_and(|w| w.minimized), "");
        // Contrôle positif : fenêtre affichée pendant le film, le détecteur doit la voir.
        let bench = &*b;
        let control = bench.film_main_appearances("S16_controle", m.r, &reference, &background, 5, || {
            bench.cmd("show", 3500);
        });
        b.check("contrôle : le détecteur voit la fenêtre quand elle est affichée", control.is_some_and(|h| h > 60), &format!("{} image(s)", control.unwrap_or(0)));
    }

    fn s17_video(b: &mut Bench) {
        b.scenario("S17", "Enregistrement vidéo : barre flottante");
        b.start("ShowWorkspace", false);
        let before = b.main().unwrap();
        b.cmd("video", 300);
        if check_overlay(b) {
            let (x1, y1, x2, y2) = sel_area();
            b.drag(x1, y1, x2, y2);
            match b.wait_floating("Enregistrement", 4000) {
                Some(bar) => {
                    b.check("barre d'enregistrement visible", bar.on_screen(), &fmt(bar.r));
                    b.check("barre hors de la zone filmée", !intersects(bar.r, (x1, y1, x2 - x1, y2 - y1)), "");
                    b.check("fenêtre principale masquée pendant l'enregistrement", b.main().is_some_and(|w| w.cloaked), "");
                    std::thread::sleep(Duration::from_secs(2));
                    b.cmd("video", 2500);
                    b.check("barre fermée", b.wait_gone("Enregistrement", 3000), "");
                    b.check("fenêtre principale réaffichée", b.main().is_some_and(|w| w.on_screen()), "");
                    check_main_same(b, &before, "fenêtre principale");
                    std::thread::sleep(Duration::from_secs(3));
                    let videos = std::fs::read_dir(&b.captures).map(|r| r.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "mp4")).count()).unwrap_or(0);
                    b.check("vidéo enregistrée", videos >= 1, &format!("{videos}"));
                }
                None => b.check("barre d'enregistrement visible", false, "absente (FFmpeg ?)"),
            }
        }
    }

    /// Échelle (DPI) de l'écran contenant le point.
    fn scale_at(x: i32, y: i32) -> f32 {
        use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};
        use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
        unsafe {
            let m = MonitorFromPoint(windows::Win32::Foundation::POINT { x, y }, MONITOR_DEFAULTTONEAREST);
            let (mut dx, mut dy) = (96, 96);
            let _ = GetDpiForMonitor(m, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
            dx as f32 / 96.0
        }
    }

    fn s18_multi_ecran(b: &mut Bench) {
        b.scenario("S18", "Plusieurs écrans : fenêtre sur chaque écran, sélection et miniature");
        let mons = monitors();
        if mons.len() < 2 {
            b.check("au moins deux écrans branchés", false, "un seul écran : scénario non exécuté");
            return;
        }
        for (i, (mon, wa)) in mons.iter().enumerate() {
            b.start("ShowWorkspace", false);
            // Espace de travail placé sur cet écran.
            b.move_main(wa.0 + 40, wa.1 + 40);
            let before = b.main().unwrap();
            b.check(&format!("écran {} : fenêtre placée sur l'écran", i + 1), intersects(before.r, *mon), &fmt(before.r));
            b.cmd("region", 300);
            if !check_overlay(b) {
                continue;
            }
            // Sélection et pointeur sur cet écran : la miniature doit s'y afficher.
            let (cx, cy) = (mon.0 + mon.2 / 2, mon.1 + mon.3 / 2);
            b.drag(cx - 150, cy - 100, cx + 150, cy + 100);
            match b.wait_floating("Miniature", 3000) {
                Some(p) => {
                    let s = scale_at(cx, cy);
                    let expected = ((336.0 * s) as i32, (282.0 * s) as i32);
                    b.check(&format!("écran {} : miniature sur l'écran du pointeur", i + 1), p.r.0 >= wa.0 && p.r.0 + p.r.2 <= wa.0 + wa.2 && p.r.1 >= wa.1 && p.r.1 + p.r.3 <= wa.1 + wa.3, &format!("{} (zone {})", fmt(p.r), fmt(*wa)));
                    b.check(&format!("écran {} : taille de la miniature à l'échelle de l'écran ({} %)", i + 1, (s * 100.0) as i32), (p.r.2 - expected.0).abs() <= 4 && (p.r.3 - expected.1).abs() <= 4, &format!("{}×{} attendu {}×{}", p.r.2, p.r.3, expected.0, expected.1));
                }
                None => b.check(&format!("écran {} : miniature visible", i + 1), false, ""),
            }
            std::thread::sleep(Duration::from_millis(500));
            check_main_same(b, &before, &format!("écran {} : fenêtre principale", i + 1));
            b.check(&format!("écran {} : fenêtre principale toujours sur son écran", i + 1), b.main().is_some_and(|w| intersects(w.r, *mon)), "");
            b.cmd("cancel", 300);
        }
    }

    /// Second écran (le premier qui n'est pas en (0, 0)) : (rectangle, zone de travail).
    fn second_monitor() -> Option<((i32, i32, i32, i32), (i32, i32, i32, i32))> {
        monitors().into_iter().find(|(m, _)| !(m.0 == 0 && m.1 == 0))
    }

    fn s19_deplacement_second_ecran(b: &mut Bench) {
        b.scenario("S19", "Espace de travail déplacé à la souris sur le second écran, capture sur cet écran");
        let Some((mon, wa)) = second_monitor() else {
            b.check("second écran branché", false, "un seul écran");
            return;
        };
        let s2 = scale_at(mon.0 + 10, mon.1 + 10);
        b.check("échelle du second écran", true, &format!("{} %", (s2 * 100.0).round()));
        b.start("ShowWorkspace", false);
        let start = b.main().unwrap();
        // Glisser la barre de titre jusqu'en haut à gauche du second écran.
        b.drag_title_to(wa.0 + 300, wa.1 + 30);
        let moved = b.main().unwrap();
        let center = (moved.r.0 + moved.r.2 / 2, moved.r.1 + moved.r.3 / 2);
        b.check("déplacée sur le second écran (centre de la fenêtre)", intersects((center.0, center.1, 1, 1), mon), &format!("{} (avant {})", fmt(moved.r), fmt(start.r)));
        let expected_w = start.r.2 as f32 * s2;
        b.check(&format!("taille adaptée à l'échelle (×{s2:.2}), sans rétrécir"), (moved.r.2 as f32 - expected_w).abs() <= 8.0, &format!("largeur {} → {} (attendu ≈ {:.0})", start.r.2, moved.r.2, expected_w));
        b.shot("S19_deplacee");
        // Zone en bas à droite du second écran (hors de l'espace de travail).
        let sel = (mon.0 + mon.2 - 700, mon.1 + mon.3 - 520, 500, 320);
        let reference = grab_region(sel);
        b.capture_zone_and_check("capture sur le second écran", sel, &reference);
        if let Some(p) = b.wait_floating("Miniature", 3000) {
            let expected = ((336.0 * s2) as i32, (282.0 * s2) as i32);
            b.check("miniature sur le second écran", intersects(p.r, wa) && p.r.0 + p.r.2 <= wa.0 + wa.2 && p.r.1 + p.r.3 <= wa.1 + wa.3, &fmt(p.r));
            b.check(&format!("miniature à l'échelle {} %", (s2 * 100.0).round()), (p.r.2 - expected.0).abs() <= 4 && (p.r.3 - expected.1).abs() <= 4, &format!("{}×{} attendu {}×{}", p.r.2, p.r.3, expected.0, expected.1));
            b.shot("S19_miniature");
        } else {
            b.check("miniature affichée", false, "");
        }
        std::thread::sleep(Duration::from_millis(500));
        let after = b.main().unwrap();
        b.check("espace de travail resté sur le second écran", same_rect(after.r, moved.r), &format!("{} (avant {})", fmt(after.r), fmt(moved.r)));
        b.check("espace de travail affiché", after.on_screen(), "");
        // Capture sur le premier écran alors que l'espace de travail est sur le second.
        let (m1, _) = monitors().into_iter().find(|(m, _)| m.0 == 0 && m.1 == 0).unwrap();
        let sel1 = (m1.0 + m1.2 / 2 - 250, m1.1 + m1.3 / 2 - 160, 500, 320);
        b.cmd("cancel", 300);
        let reference1 = grab_region(sel1);
        b.capture_zone_and_check("capture sur le premier écran", sel1, &reference1);
        std::thread::sleep(Duration::from_millis(500));
        b.check("espace de travail toujours sur le second écran", b.main().is_some_and(|w| same_rect(w.r, moved.r)), "");
        // Réduction / zone de notification puis réouverture : même place.
        b.close_main();
        b.cmd("show", 800);
        b.check("rouvert au même endroit sur le second écran", b.main().is_some_and(|w| same_rect(w.r, moved.r) && w.on_screen()), "");
        // Retour sur le premier écran à la souris.
        b.drag_title_to(m1.0 + 400, m1.1 + 60);
        let back = b.main().unwrap();
        let center = (back.r.0 + back.r.2 / 2, back.r.1 + back.r.3 / 2);
        b.check("ramenée sur le premier écran (centre de la fenêtre)", intersects((center.0, center.1, 1, 1), m1), &fmt(back.r));
        b.check("taille d'origine retrouvée", (back.r.2 - start.r.2).abs() <= 4 && (back.r.3 - start.r.3).abs() <= 4, &format!("{}×{} (départ {}×{})", back.r.2, back.r.3, start.r.2, start.r.3));
    }

    fn s20_agrandie_second_ecran(b: &mut Bench) {
        b.scenario("S20", "Espace de travail agrandi (plein écran) sur le second écran");
        let Some((mon, wa)) = second_monitor() else {
            b.check("second écran branché", false, "un seul écran");
            return;
        };
        b.start("ShowWorkspace", false);
        b.drag_title_to(wa.0 + 300, wa.1 + 30);
        b.maximize_main();
        let max = b.main().unwrap();
        b.check("agrandie", max.maximized, "");
        b.check("occupe le second écran", intersects(max.r, mon) && (max.r.2 - wa.2).abs() <= 20 && (max.r.3 - wa.3).abs() <= 20, &format!("{} (zone {})", fmt(max.r), fmt(wa)));
        b.shot("S20_agrandie");
        // Capture sur le second écran (zone couverte par l'espace de travail, masqué pendant la capture).
        let sel = (mon.0 + 200, mon.1 + 200, 600, 400);
        b.cmd("region", 300);
        b.check("masquée pendant la sélection", b.main().is_some_and(|w| w.cloaked), "");
        if b.wait_floating("Sélection", 3000).is_some() {
            // L'image capturée ne doit pas contenir l'espace de travail : elle montre le bureau.
            b.drag(sel.0, sel.1, sel.0 + sel.2, sel.1 + sel.3);
            b.wait_floating("Miniature", 3000);
            std::thread::sleep(Duration::from_millis(500));
            let a = b.main().unwrap();
            b.check("toujours agrandie après la capture", a.maximized && a.on_screen(), "");
            b.check("même position et taille", same_rect(a.r, max.r), &format!("{} (avant {})", fmt(a.r), fmt(max.r)));
        }
        // Capture plein écran.
        b.cmd("full", 1500);
        let a = b.main().unwrap();
        b.check("plein écran : toujours agrandie", a.maximized && same_rect(a.r, max.r), &fmt(a.r));
        // Fermeture (zone de notification) puis réouverture.
        b.close_main();
        b.cmd("show", 900);
        let a = b.main().unwrap();
        b.check("rouverte agrandie au même endroit", a.maximized && a.on_screen() && same_rect(a.r, max.r), &fmt(a.r));
        // Réduite puis capture puis réouverture.
        b.minimize_main();
        b.cmd("full", 1500);
        b.check("reste réduite pendant une capture", b.main().is_some_and(|w| w.minimized), "");
        b.cmd("show", 900);
        let a = b.main().unwrap();
        b.check("restaurée agrandie au même endroit", a.maximized && a.on_screen() && same_rect(a.r, max.r), &fmt(a.r));
        b.check("interface réactive", b.responsive(3000), "");
    }

    fn s21_plein_ecran_multi(b: &mut Bench) {
        b.scenario("S21", "Capture plein écran sur deux écrans d'échelles différentes");
        let Some((mon, _)) = second_monitor() else {
            b.check("second écran branché", false, "un seul écran");
            return;
        };
        b.start("ShowWorkspace", true);
        let vs = virtual_screen();
        let zone2 = (mon.0 + mon.2 / 2 - 300, mon.1 + mon.3 / 2 - 200, 600, 400);
        let reference = grab_region(zone2);
        let n = b.captures_count();
        b.cmd("full", 2000);
        b.check("capture enregistrée", b.captures_count() > n, "");
        match b.newest_capture().and_then(|p| image::open(&p).ok()).map(|i| i.to_rgba8()) {
            Some(img) => {
                b.check("image à la taille du bureau virtuel (pixels physiques)", img.width() as i32 == vs.2 && img.height() as i32 == vs.3, &format!("{}×{} pour {}×{}", img.width(), img.height(), vs.2, vs.3));
                let part = image::imageops::crop_imm(&img, (zone2.0 - vs.0) as u32, (zone2.1 - vs.1) as u32, zone2.2 as u32, zone2.3 as u32).to_image();
                let sim = similarity(&image::imageops::resize(&part, 64, 48, image::imageops::FilterType::Triangle), &image::imageops::resize(&reference, 64, 48, image::imageops::FilterType::Triangle));
                b.check("second écran à sa place et à sa taille dans l'image", sim > 0.95, &format!("ressemblance {:.1} %", sim * 100.0));
            }
            None => b.check("image lisible", false, ""),
        }
    }

    /// Diagnostic : taille de la fenêtre à chaque étape d'un glisser vers le second écran.
    fn d1_trace_deplacement(b: &mut Bench) {
        b.scenario("D1", "Trace du déplacement à la souris");
        let Some((_, wa)) = second_monitor() else { return };
        b.start("ShowWorkspace", false);
        let m = b.main().unwrap();
        let (gx, gy) = (m.r.0 + m.r.2 / 3, m.r.1 + 14);
        let (x, y) = (wa.0 + 300, wa.1 + 30);
        mouse_to(gx, gy);
        std::thread::sleep(Duration::from_millis(200));
        mouse_button(true);
        std::thread::sleep(Duration::from_millis(150));
        for i in 1..=30 {
            mouse_to(gx + (x - gx) * i / 30, gy + (y - gy) * i / 30);
            std::thread::sleep(Duration::from_millis(60));
            if let Some(w) = b.main() {
                println!("    étape {i:2} souris ({}, {}) fenêtre {}", gx + (x - gx) * i / 30, gy + (y - gy) * i / 30, fmt(w.r));
            }
        }
        mouse_button(false);
        std::thread::sleep(Duration::from_millis(800));
        println!("    fin : {}", b.main().map(|w| fmt(w.r)).unwrap_or_default());
    }

    fn s22_echap(b: &mut Bench) {
        b.scenario("S22", "Échap annule une capture différée pendant le compte à rebours");
        b.start("ShowWorkspace", false);
        let before = b.main().unwrap();
        // Fenêtre de test du projet au premier plan : si l'interception d'Échap échouait, la touche
        // irait là (et non dans une application de l'utilisateur). Elle est fermée à la fin du test.
        let target_exe = b.exe.parent().unwrap().join("examples").join("dpi_probe.exe");
        let mut target = Command::new(&target_exe).arg("Cible de test ScreenCapture-validation").spawn().ok();
        std::thread::sleep(Duration::from_millis(2500));
        let fg = foreground_title();
        let safe = fg == "Cible de test ScreenCapture-validation";
        b.check("fenêtre cible de test au premier plan", safe, &fg);
        if safe {
            let n = b.captures_count();
            b.cmd("delayed", 1000);
            b.check("espace de travail masqué pendant le délai", b.main().is_some_and(|w| w.cloaked), "");
            key_raw(VK_ESCAPE, &[]);
            std::thread::sleep(Duration::from_secs(4));
            b.check("aucune sélection ouverte après le délai", b.floating("Sélection").is_none(), "");
            b.check("aucune capture enregistrée", b.captures_count() == n, &format!("{} → {}", n, b.captures_count()));
            b.check("aucune miniature", b.floating("Miniature").is_none(), "");
            b.check("espace de travail réaffiché", b.main().is_some_and(|w| w.on_screen()), "");
            check_main_same(b, &before, "espace de travail");
            // Échap dans la sélection, alors qu'une autre application avait le premier plan.
            b.cmd("region", 300);
            if b.wait_floating("Sélection", 3000).is_some() {
                key_raw(VK_ESCAPE, &[]);
                b.check("Échap ferme la sélection", b.wait_gone("Sélection", 2000), "");
            }
            b.check("interface réactive", b.responsive(3000), "");
        }
        if let Some(p) = target.as_mut() {
            let _ = p.kill();
        }
    }

    fn s23_taille_minimale(b: &mut Bench) {
        b.scenario("S23", "Taille minimale de l'espace de travail (900 × 600 points)");
        for (mon, wa) in monitors() {
            b.start("ShowWorkspace", false);
            b.move_main(wa.0 + 60, wa.1 + 60);
            std::thread::sleep(Duration::from_millis(500));
            let s = scale_at(wa.0 + 100, wa.1 + 100);
            if let Some(m) = b.main() {
                unsafe {
                    let _ = SetWindowPos(HWND(m.hwnd as *mut _), HWND_TOP, 0, 0, 300, 200, windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE | SWP_NOACTIVATE);
                }
            }
            std::thread::sleep(Duration::from_millis(1200));
            let m = b.main().unwrap();
            let (minw, minh) = ((900.0 * s) as i32, (600.0 * s) as i32);
            b.check(&format!("écran à {} % : taille minimale respectée", (s * 100.0).round()), m.r.2 >= minw && m.r.3 >= minh, &format!("{}×{} (minimum {}×{} + bordures)", m.r.2, m.r.3, minw, minh));
            b.check("toujours sur le même écran", intersects(m.r, mon), "");
        }
    }

    /// Diagnostic : glisser du second écran vers le premier, étape par étape.
    fn d2_trace_retour(b: &mut Bench) {
        b.scenario("D2", "Trace du retour vers le premier écran");
        let Some((_, wa)) = second_monitor() else { return };
        b.start("ShowWorkspace", false);
        b.drag_title_to(wa.0 + 300, wa.1 + 30);
        std::thread::sleep(Duration::from_millis(1500));
        let m = b.main().unwrap();
        println!("    départ {}", fmt(m.r));
        let (gx, gy) = (m.r.0 + m.r.2 / 3, m.r.1 + 14);
        let (x, y) = (400, 60);
        mouse_to(gx, gy);
        std::thread::sleep(Duration::from_millis(200));
        mouse_button(true);
        std::thread::sleep(Duration::from_millis(150));
        for i in 1..=30 {
            mouse_to(gx + (x - gx) * i / 30, gy + (y - gy) * i / 30);
            std::thread::sleep(Duration::from_millis(60));
            if let Some(w) = b.main() {
                println!("    étape {i:2} souris ({}, {}) fenêtre {}", gx + (x - gx) * i / 30, gy + (y - gy) * i / 30, fmt(w.r));
            }
        }
        mouse_button(false);
        std::thread::sleep(Duration::from_millis(1500));
        println!("    fin : {}", b.main().map(|w| fmt(w.r)).unwrap_or_default());
    }

    pub const VK_CONTROL: u8 = 0x11;
    pub const VK_SHIFT: u8 = 0x10;
    pub const VK_DELETE: u8 = 0x2E;
    pub const VK_RETURN: u8 = 0x0D;

    fn s24_selection_multiple(b: &mut Bench) {
        b.scenario("S24", "Sélection multiple et suppression au clavier dans l'espace de travail");
        // Six captures de test, les plus récentes en premier dans l'historique.
        b.kill();
        for e in std::fs::read_dir(&b.captures).into_iter().flatten().flatten() {
            let _ = std::fs::remove_file(e.path());
        }
        for i in 1..=6u8 {
            let img = image::RgbaImage::from_pixel(320, 200, image::Rgba([40 * i, 120, 200 - 25 * i, 255]));
            img.save(b.captures.join(format!("Test_selection_{i}.png"))).unwrap();
            std::thread::sleep(Duration::from_millis(60));
        }
        b.start("ShowWorkspace", false);
        center_main(b);
        std::thread::sleep(Duration::from_millis(1500));
        let cards = b.cards();
        b.check("six vignettes affichées", cards.len() == 6, &format!("{}", cards.len()));
        if cards.len() < 6 {
            return;
        }
        let name = |i: usize| cards[i].0.clone();
        let at = |i: usize| (cards[i].1, cards[i].2);

        // 1) Clic puis Ctrl+clic : deux captures, Suppr + Entrée.
        let (x, y) = at(0);
        b.click(x, y, &[]);
        let (x, y) = at(2);
        b.click(x, y, &[VK_CONTROL]);
        b.check("clic puis Ctrl+clic : 2 sélectionnées", b.selected_count() == Some(2), &format!("{:?}", b.selected_count()));
        b.shot("S24_selection");
        let removed = [name(0), name(2)];
        if b.key(VK_DELETE, &[]) {
            std::thread::sleep(Duration::from_millis(500));
            b.shot("S24_confirmation");
            b.key(VK_RETURN, &[]);
            std::thread::sleep(Duration::from_millis(1500));
        }
        let files = b.capture_files("png");
        b.check("Suppr + Entrée : les 2 captures sélectionnées supprimées", removed.iter().all(|r| !files.contains(r)), &format!("restantes : {}", files.len()));
        b.check("les autres captures conservées", files.len() == 4, &format!("{files:?}"));

        // 2) Maj+clic : plage de 3, Suppr puis Échap (annulation).
        std::thread::sleep(Duration::from_millis(800));
        let cards = b.cards();
        if cards.len() < 4 {
            b.check("vignettes restantes affichées", false, &format!("{}", cards.len()));
            return;
        }
        let (x, y) = (cards[0].1, cards[0].2);
        b.click(x, y, &[]);
        let (x, y) = (cards[2].1, cards[2].2);
        b.click(x, y, &[VK_SHIFT]);
        b.check("Maj+clic : plage de 3 sélectionnée", b.selected_count() == Some(3), &format!("{:?}", b.selected_count()));
        if b.key(VK_DELETE, &[]) {
            std::thread::sleep(Duration::from_millis(500));
            b.key(VK_ESCAPE, &[]);
            std::thread::sleep(Duration::from_millis(800));
        }
        b.check("Échap dans la confirmation : rien supprimé", b.capture_files("png").len() == 4, "");
        // Ctrl+clic pour retirer une capture de la sélection.
        let (x, y) = (cards[1].1, cards[1].2);
        b.click(x, y, &[VK_CONTROL]);
        b.check("Ctrl+clic sur une capture sélectionnée : retirée", b.selected_count() == Some(2), &format!("{:?}", b.selected_count()));

        // 3) Ctrl+A : tout, Suppr + Entrée.
        b.key(0x41, &[VK_CONTROL]);
        b.check("Ctrl+A : toutes sélectionnées", b.selected_count() == Some(4), &format!("{:?}", b.selected_count()));
        if b.key(VK_DELETE, &[]) {
            std::thread::sleep(Duration::from_millis(500));
            b.key(VK_RETURN, &[]);
            std::thread::sleep(Duration::from_millis(1500));
        }
        b.check("Ctrl+A puis Suppr + Entrée : tout supprimé", b.capture_files("png").is_empty(), &format!("{:?}", b.capture_files("png")));
        b.check("interface réactive", b.responsive(3000), "");
    }

    /// Premier micro vu par FFmpeg (DirectShow), s'il y en a un.
    fn first_microphone(ffmpeg: &Path) -> Option<String> {
        let out = Command::new(ffmpeg).args(["-hide_banner", "-list_devices", "true", "-f", "dshow", "-i", "dummy"]).output().ok()?;
        let text = String::from_utf8_lossy(&out.stderr).to_string();
        text.lines().filter(|l| l.contains("(audio)")).find_map(|l| {
            let s = l.find('"')?;
            let e = l[s + 1..].find('"')?;
            Some(l[s + 1..s + 1 + e].to_string())
        })
    }

    /// Nombre de pistes audio d'une vidéo (ffprobe).
    fn audio_streams(ffmpeg: &Path, video: &Path) -> Option<usize> {
        let ffprobe = ffmpeg.with_file_name("ffprobe.exe");
        let out = Command::new(ffprobe).args(["-v", "error", "-select_streams", "a", "-show_entries", "stream=codec_type", "-of", "csv=p=0"]).arg(video).output().ok()?;
        Some(String::from_utf8_lossy(&out.stdout).lines().filter(|l| l.contains("audio")).count())
    }

    fn s25_video_son(b: &mut Bench) {
        b.scenario("S25", "Vidéo avec ou sans le son (bouton / touche M dans la sélection)");
        let Some(ffmpeg) = b.ffmpeg.clone() else {
            b.check("FFmpeg disponible", false, "");
            return;
        };
        let mic = first_microphone(&ffmpeg);
        b.check("micro disponible pour le test", mic.is_some(), mic.as_deref().unwrap_or("aucun : seule la logique est vérifiée"));
        let device = mic.clone().unwrap_or_else(|| "Micro de test".into());
        b.extra = vec![("audio_device".into(), device.clone().into()), ("record_audio".into(), true.into())];
        b.start("ShowWorkspace", false);
        b.extra.clear();
        let (x1, y1, x2, y2) = sel_area();
        let record = |b: &mut Bench, label: &str, press_m: bool| -> Option<(String, Option<PathBuf>)> {
            let before = b.capture_files("mp4");
            let mark = b.log_mark();
            b.cmd("video", 300);
            if b.wait_floating("Sélection", 3000).is_none() {
                b.check(&format!("{label} : sélection affichée"), false, "");
                return None;
            }
            std::thread::sleep(Duration::from_millis(400));
            if press_m {
                b.key(0x4D, &[]);
                std::thread::sleep(Duration::from_millis(300));
            }
            b.shot(&format!("S25_{}", label.replace(' ', "_")));
            b.drag(x1, y1, x2, y2);
            b.wait_floating("Enregistrement", 4000);
            std::thread::sleep(Duration::from_secs(3));
            b.cmd("video", 300);
            let t = Instant::now();
            let mut file = None;
            while t.elapsed() < Duration::from_secs(10) && file.is_none() {
                std::thread::sleep(Duration::from_millis(300));
                file = b.capture_files("mp4").into_iter().find(|f| !before.contains(f));
            }
            std::thread::sleep(Duration::from_millis(800));
            let log = b.log_since(mark);
            let line = log.lines().find(|l| l.contains("enregistrement vidéo")).unwrap_or("").to_string();
            Some((line, file.map(|f| b.captures.join(f))))
        };
        // 1) Réglage par défaut « avec le son ».
        if let Some((line, file)) = record(b, "avec le son", false) {
            b.check("par défaut : avec le son", line.contains("avec le son") || line.contains("introuvable"), &line.split("] ").nth(1).unwrap_or("").to_string());
            if let (Some(f), true) = (file.as_ref(), mic.is_some()) {
                let n = audio_streams(&ffmpeg, f);
                b.check("la vidéo contient une piste audio", n == Some(1), &format!("{n:?}"));
                let v = mean_volume(&ffmpeg, f, None);
                b.check("le son du micro est enregistré", v.is_some_and(|v| v > -85.0), &format!("{v:?} dB"));
            }
            b.check("vidéo enregistrée", file.is_some(), "");
        }
        // 2) M dans la sélection : sans le son.
        if let Some((line, file)) = record(b, "M sans le son", true) {
            b.check("M : sans le son", line.contains("sans le son"), &line.split("] ").nth(1).unwrap_or("").to_string());
            if let (Some(f), true) = (file.as_ref(), mic.is_some()) {
                let v = mean_volume(&ffmpeg, f, None);
                b.check("piste audio silencieuse (micro coupé)", v.is_some_and(|v| v <= -90.0), &format!("{v:?} dB"));
            }
        }
        // 3) Le choix est mémorisé pour la vidéo suivante.
        if let Some((line, _)) = record(b, "choix mémorisé", false) {
            b.check("choix « sans le son » mémorisé", line.contains("sans le son"), "");
        }
        // 4) M de nouveau : le son revient.
        if let Some((line, _)) = record(b, "M avec le son", true) {
            b.check("M de nouveau : avec le son", line.contains("avec le son") || line.contains("introuvable"), "");
        }
    }

    /// Volume moyen (dB) de la piste audio, sur `[from, to]` secondes ou tout le fichier.
    fn mean_volume(ffmpeg: &Path, video: &Path, range: Option<(f32, f32)>) -> Option<f32> {
        let mut cmd = Command::new(ffmpeg);
        cmd.arg("-hide_banner");
        if let Some((a, z)) = range {
            cmd.args(["-ss", &format!("{a}"), "-to", &format!("{z}")]);
        }
        let out = cmd.arg("-i").arg(video).args(["-vn", "-af", "volumedetect", "-f", "null", "-"]).output().ok()?;
        let text = String::from_utf8_lossy(&out.stderr).to_string();
        let line = text.lines().find(|l| l.contains("mean_volume"))?;
        line.split("mean_volume:").nth(1)?.trim().trim_end_matches("dB").trim().parse().ok()
    }

    fn s26_micro_pendant_enregistrement(b: &mut Bench) {
        b.scenario("S26", "Micro activé puis coupé pendant l'enregistrement (bouton de la barre)");
        let Some(ffmpeg) = b.ffmpeg.clone() else { return };
        let Some(mic) = first_microphone(&ffmpeg) else {
            b.check("micro branché", false, "aucun micro : scénario non exécuté");
            return;
        };
        b.extra = vec![("audio_device".into(), mic.clone().into()), ("record_audio".into(), false.into())];
        b.start("ShowWorkspace", false);
        b.extra.clear();
        let before = b.capture_files("mp4");
        let mark = b.log_mark();
        b.cmd("video", 300);
        if b.wait_floating("Sélection", 3000).is_none() {
            b.check("sélection affichée", false, "");
            return;
        }
        let (x1, y1, x2, y2) = sel_area();
        b.drag(x1, y1, x2, y2);
        let bar = b.wait_floating("Enregistrement", 4000);
        b.check("enregistrement démarré sans le son", b.log_since(mark).contains("sans le son (micro"), "");
        if let Some(bar) = &bar {
            b.shot("S26_barre_micro_coupe");
            let _ = bar;
        }
        // Silence 3 s, puis micro activé 3 s, puis coupé 3 s.
        std::thread::sleep(Duration::from_secs(3));
        b.cmd("mic", 200);
        std::thread::sleep(Duration::from_secs(3));
        b.shot("S26_barre_micro_actif");
        b.cmd("mic", 200);
        std::thread::sleep(Duration::from_secs(3));
        b.cmd("video", 300);
        let t = Instant::now();
        let mut file = None;
        while t.elapsed() < Duration::from_secs(10) && file.is_none() {
            std::thread::sleep(Duration::from_millis(300));
            file = b.capture_files("mp4").into_iter().find(|f| !before.contains(f));
        }
        std::thread::sleep(Duration::from_millis(800));
        let log = b.log_since(mark);
        b.check("micro activé pendant l'enregistrement", log.contains("micro activé pendant l'enregistrement : ok"), "");
        b.check("micro coupé pendant l'enregistrement", log.contains("micro coupé pendant l'enregistrement : ok"), "");
        let Some(f) = file.map(|f| b.captures.join(f)) else {
            b.check("vidéo enregistrée", false, "");
            return;
        };
        // Volume par seconde : le profil attendu est silence / son / silence.
        let levels: Vec<f32> = (0..10).map(|k| mean_volume(&ffmpeg, &f, Some((k as f32, k as f32 + 1.0))).unwrap_or(-91.0)).collect();
        let profile = levels.iter().map(|v| format!("{v:.0}")).collect::<Vec<_>>().join(" ");
        let loud: Vec<usize> = levels.iter().enumerate().filter(|(_, v)| **v > -85.0).map(|(i, _)| i).collect();
        let silent_before = levels.iter().take_while(|v| **v <= -90.0).count();
        b.check("silence au début (micro coupé)", silent_before >= 2, &format!("dB par seconde : {profile}"));
        b.check("son pendant environ 3 s (micro activé)", (3..=5).contains(&loud.len()) && loud.windows(2).all(|w| w[1] == w[0] + 1), &format!("secondes avec du son : {loud:?}"));
        b.check("silence à la fin (micro recoupé)", levels.iter().rev().take(2).all(|v| *v <= -90.0), "");
        b.check("choix mémorisé (dernier état : coupé)", std::fs::read_to_string(&b.settings).unwrap_or_default().contains("\"record_audio\": false"), "");
    }

    fn s27_bandeau_aide(b: &mut Bench) {
        b.scenario("S27", "Le bandeau d'aide s'écarte de ce qu'on vise");
        b.start("ShowWorkspace", false);
        let position = |b: &Bench| -> String {
            let mark = b.log_mark();
            b.cmd("state", 500);
            b.log_since(mark).split("bandeau=").nth(1).and_then(|r| r.split(|c: char| !c.is_alphanumeric() && c != 'é').next()).unwrap_or("?").to_string()
        };
        let (m1, _) = monitors().into_iter().find(|(m, _)| m.0 == 0 && m.1 == 0).unwrap();
        b.cmd("region", 300);
        if b.wait_floating("Sélection", 3000).is_none() {
            b.check("sélection affichée", false, "");
            return;
        }
        mouse_to(m1.0 + m1.2 / 4, m1.1 + m1.3 / 2);
        std::thread::sleep(Duration::from_millis(400));
        let p = position(b);
        b.check("pointeur au milieu : bandeau en haut", p == "haut", &p);
        // Pointeur sur le bandeau (en haut au centre, là où se trouve le titre d'une fenêtre).
        mouse_to(m1.0 + m1.2 / 2, m1.1 + 30);
        std::thread::sleep(Duration::from_millis(400));
        let p = position(b);
        b.check("pointeur en haut au centre : bandeau déplacé en bas", p == "bas", &p);
        b.shot("S27_bandeau_en_bas");
        // Sélection en cours couvrant toute la hauteur de l'écran : bandeau masqué.
        mouse_to(m1.0 + m1.2 / 2 - 300, m1.1 + 5);
        std::thread::sleep(Duration::from_millis(150));
        mouse_button(true);
        for i in 1..=10 {
            mouse_to(m1.0 + m1.2 / 2 - 300 + 60 * i, m1.1 + 5 + (m1.3 - 10) * i / 10);
            std::thread::sleep(Duration::from_millis(30));
        }
        std::thread::sleep(Duration::from_millis(300));
        let p = position(b);
        b.check("sélection sur toute la hauteur : bandeau masqué", p == "masqué", &p);
        let mark = b.log_mark();
        b.cmd("state", 500);
        let zone = b.log_since(mark).split("zone=").nth(1).map(|z| z.chars().take(60).collect::<String>()).unwrap_or_default();
        b.check("la sélection commence exactement au point cliqué (y = 5)", zone.contains("y: 5,"), &zone);
        b.shot("S27_bandeau_masque");
        mouse_button(false);
        std::thread::sleep(Duration::from_millis(600));
        b.cmd("cancel", 300);
    }

    /// Fichier de test daté de `days` jours (création et modification), de `kb` Ko.
    fn dated_file(dir: &Path, name: &str, days: u64, kb: usize) {
        use std::os::windows::fs::FileTimesExt;
        let p = dir.join(name);
        std::fs::write(&p, vec![7u8; kb * 1024]).unwrap();
        let t = std::time::SystemTime::now() - Duration::from_secs(days * 24 * 3600);
        let f = std::fs::File::options().write(true).open(&p).unwrap();
        f.set_times(std::fs::FileTimes::new().set_created(t).set_modified(t)).unwrap();
    }

    /// Interface dans chaque langue : langue appliquée au démarrage, pages enregistrées en
    /// pleine résolution pour la relecture (textes coupés ou qui débordent).
    fn s29_langues(b: &mut Bench) {
        b.scenario("S29", "Interface en français, anglais, espagnol et allemand");
        for lang in ["En", "Es", "De", "Fr"] {
            b.extra = vec![("language".into(), lang.into())];
            let mark = b.log_mark();
            b.start("ShowWorkspace", false);
            b.extra.clear();
            let log = b.log_since(mark);
            b.check(&format!("{lang} : langue appliquée au démarrage"), log.contains(&format!("langue : {lang} ")), "");
            for page in ["home", "settings", "about"] {
                b.cmd(page, 900);
                let Some(m) = b.main() else {
                    b.check(&format!("{lang} : page {page} affichée"), false, "fenêtre introuvable");
                    continue;
                };
                let img = grab_region(m.r);
                let file = format!("S29_{lang}_{page}.png");
                let _ = img.save(b.out.join(&file));
                b.shots.push((format!("S29 — {lang} {page}"), file));
                b.check(&format!("{lang} : page {page} affichée"), m.r.2 > 0 && m.r.3 > 0, &fmt(m.r));
            }
        }
    }

    // ------------------------------------------------------------------ aide (captures génériques)

    /// Image de démonstration générique : maquette de tableau de bord (aucun contenu réel).
    fn demo_image(seed: u32) -> image::RgbaImage {
        let (w, h) = (1280u32, 800u32);
        let accents = [[103u8, 80, 164], [0, 121, 107], [211, 84, 0], [25, 118, 210], [194, 24, 91], [56, 142, 60]];
        let a = accents[seed as usize % accents.len()];
        let mut img = image::RgbaImage::from_pixel(w, h, image::Rgba([246, 247, 251, 255]));
        let rect = |img: &mut image::RgbaImage, x: u32, y: u32, rw: u32, rh: u32, c: [u8; 3]| {
            for yy in y..(y + rh).min(h) {
                for xx in x..(x + rw).min(w) {
                    img.put_pixel(xx, yy, image::Rgba([c[0], c[1], c[2], 255]));
                }
            }
        };
        let light = |c: [u8; 3], k: f32| [(c[0] as f32 + (255.0 - c[0] as f32) * k) as u8, (c[1] as f32 + (255.0 - c[1] as f32) * k) as u8, (c[2] as f32 + (255.0 - c[2] as f32) * k) as u8];
        rect(&mut img, 0, 0, w, 64, a);
        rect(&mut img, 0, 64, 220, h - 64, [234, 236, 243]);
        for i in 0..6 {
            rect(&mut img, 24, 96 + i * 52, 150 - (i * 13) % 50, 14, [190, 194, 206]);
        }
        rect(&mut img, 24, 22, 180, 20, light(a, 0.6));
        // Trois cartes à barres.
        for k in 0..3u32 {
            let (cx, cy) = (252 + k * 336, 96);
            rect(&mut img, cx, cy, 312, 300, [255, 255, 255]);
            rect(&mut img, cx + 20, cy + 20, 120, 12, [200, 204, 214]);
            for b in 0..7u32 {
                let v = 40 + ((b * 37 + k * 53 + seed * 29) % 160);
                rect(&mut img, cx + 24 + b * 40, cy + 270 - v, 26, v, light(a, 0.15 * k as f32));
            }
        }
        // Courbe.
        rect(&mut img, 252, 420, 1000, 340, [255, 255, 255]);
        rect(&mut img, 272, 440, 160, 12, [200, 204, 214]);
        let mut prev = None;
        for x in 0..940u32 {
            let t = x as f32 / 60.0 + seed as f32;
            let y = 600.0 - 90.0 * (t.sin() * 0.6 + (t * 0.37).cos() * 0.4) - x as f32 * 0.08;
            let p = (282 + x, y as u32);
            if let Some((px, py)) = prev {
                let (lo, hi): (u32, u32) = if py < p.1 { (py, p.1) } else { (p.1, py) };
                for yy in lo..=hi {
                    for d in 0..3 {
                        img.put_pixel(px, (yy + d).min(h - 1), image::Rgba([a[0], a[1], a[2], 255]));
                    }
                }
            }
            prev = Some(p);
        }
        img
    }

    /// Enregistre une capture de l'aide (réduite à 1100 px de large, JPEG).
    fn save_help_shot(img: &image::RgbaImage, dir: &Path, name: &str) {
        let w = img.width().min(1100);
        let h = (img.height() as f32 * w as f32 / img.width() as f32) as u32;
        let small = image::imageops::resize(img, w, h, image::imageops::FilterType::Lanczos3);
        let rgb = image::DynamicImage::ImageRgba8(small).to_rgb8();
        let mut out = std::fs::File::create(dir.join(format!("{name}.jpg"))).unwrap();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85).encode_image(&rgb).unwrap();
    }

    /// Captures d'écran de l'aide, dans chaque langue, avec des images de démonstration et un
    /// dossier de captures neutre (rien de la machine de test n'apparaît).
    fn h1_aide(b: &mut Bench) {
        b.scenario("H1", "Captures d'écran de l'aide (contenu générique)");
        let real_captures = b.captures.clone();
        let demo = PathBuf::from(r"C:\Users\Public\Pictures\ScreenCapture");
        let _ = std::fs::remove_dir_all(&demo);
        std::fs::create_dir_all(&demo).unwrap();
        b.captures = demo.clone();
        let names = ["Capture_2026-03-02_09-14-05", "Capture_2026-03-02_10-41-37", "Capture_2026-03-03_14-02-11", "Capture_2026-03-04_08-55-49", "Capture_2026-03-04_16-20-03", "Capture_2026-03-05_11-07-26"];
        for (i, n) in names.iter().enumerate() {
            demo_image(i as u32).save(demo.join(format!("{n}.png"))).unwrap();
            std::thread::sleep(Duration::from_millis(60));
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("help").join("img");
        let primary = monitors().into_iter().find(|(m, _)| m.0 == 0 && m.1 == 0).or(monitors().into_iter().next()).unwrap();
        // Fond générique plein écran : le bureau réel n'apparaît ni autour des fenêtres ni dans la
        // sélection (qui fige l'écran).
        let wall = std::env::temp_dir().join("sc_demo_backdrop.png");
        image::imageops::resize(&demo_image(3), 2560, 1600, image::imageops::FilterType::Triangle).save(&wall).unwrap();
        let backdrop_exe = b.exe.parent().unwrap().join("examples").join("demo_backdrop.exe");
        let mut backdrop = match Command::new(&backdrop_exe).arg(&wall).spawn() {
            Ok(c) => c,
            Err(e) => {
                b.check("fond de démonstration lancé", false, &format!("{} : {e}", backdrop_exe.display()));
                b.captures = real_captures;
                return;
            }
        };
        std::thread::sleep(Duration::from_secs(2));
        for lang in ["fr", "en", "es", "de"] {
            let dir = root.join(lang);
            std::fs::create_dir_all(&dir).unwrap();
            let code = match lang { "fr" => "Fr", "en" => "En", "es" => "Es", _ => "De" };
            b.extra = vec![("language".into(), code.into()), ("custom_stamp".into(), "".into())];
            b.start("ShowWorkspace", false);
            b.extra.clear();
            // Le contenu de la fenêtre au centre de l'écran principal, taille par défaut.
            let Some(m) = b.main() else { continue };
            b.move_main(primary.1 .0 + (primary.1 .2 - m.r.2) / 2, primary.1 .1 + (primary.1 .3 - m.r.3) / 2);
            std::thread::sleep(Duration::from_millis(1500));

            // 1) Espace de travail, deux captures sélectionnées par leur case.
            b.cmd("home", 900);
            let mark = b.log_mark();
            b.cmd("cards", 700);
            let cards: Vec<(f32, f32)> = b
                .log_since(mark)
                .lines()
                .filter_map(|l| {
                    let mut it = l.split("carte ").nth(1)?.split(' ').skip(1);
                    Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
                })
                .collect();
            let scale = scale_at(primary.0 .0 + 10, primary.0 .1 + 10);
            for &(x, y) in cards.iter().skip(1).take(2) {
                b.click((x + 24.0 * scale) as i32, (y + 24.0 * scale) as i32, &[]);
                std::thread::sleep(Duration::from_millis(700));
            }
            std::thread::sleep(Duration::from_millis(500));
            if let Some(m) = b.main() {
                save_help_shot(&window_shot(m.r), &dir, "accueil");
            }
            b.key(VK_ESCAPE, &[]);

            // 2) Paramètres (haut de la page).
            b.cmd("settings", 1200);
            if let Some(m) = b.main() {
                save_help_shot(&window_shot(m.r), &dir, "parametres");
            }

            // 3) Éditeur avec quelques annotations.
            b.cmd("editor", 1800);
            if let Some(m) = b.main() {
                let s = scale;
                let (l, t) = (m.r.0 as f32, m.r.1 as f32);
                // Zone de dessin approximative : entre la barre d'outils et le panneau de droite.
                let cx = l + (m.r.2 as f32 - 300.0 * s) / 2.0 + 30.0 * s;
                let cy = t + m.r.3 as f32 / 2.0 + 20.0 * s;
                let at = |dx: f32, dy: f32| ((cx + dx * s) as i32, (cy + dy * s) as i32);
                if b.key(0x52, &[]) {
                    let (x1, y1) = at(-210.0, -160.0);
                    let (x2, y2) = at(-40.0, -40.0);
                    b.drag(x1, y1, x2, y2);
                }
                if b.key(0x41, &[]) {
                    let (x1, y1) = at(120.0, 120.0);
                    let (x2, y2) = at(10.0, 40.0);
                    b.drag(x1, y1, x2, y2);
                }
                if b.key(0x4E, &[]) {
                    for (dx, dy) in [(-230.0, -180.0), (150.0, -150.0), (-220.0, 90.0)] {
                        let (x, y) = at(dx, dy);
                        b.click(x, y, &[]);
                    }
                }
                if b.key(0x53, &[]) {
                    let (x, y) = at(170.0, 150.0);
                    b.click(x, y, &[]);
                }
                b.key(0x56, &[]);
                let (x, y) = at(0.0, 230.0);
                mouse_to(x, y);
                std::thread::sleep(Duration::from_millis(400));
                if let Some(m) = b.main() {
                    save_help_shot(&window_shot(m.r), &dir, "editeur");
                }

                // 4) Sélection d'une zone (au-dessus de l'éditeur, plein écran).
                unsafe {
                    let _ = ShowWindow(HWND(m.hwnd as *mut _), SW_MAXIMIZE);
                }
                std::thread::sleep(Duration::from_millis(1200));
                b.cmd("region", 1500);
                let wa = primary.1;
                let (x1, y1) = (wa.0 + wa.2 * 30 / 100, wa.1 + wa.3 * 28 / 100);
                let (x2, y2) = (wa.0 + wa.2 * 62 / 100, wa.1 + wa.3 * 64 / 100);
                mouse_to(x1, y1);
                std::thread::sleep(Duration::from_millis(200));
                mouse_button(true);
                for i in 1..=12 {
                    mouse_to(x1 + (x2 - x1) * i / 12, y1 + (y2 - y1) * i / 12);
                    std::thread::sleep(Duration::from_millis(30));
                }
                std::thread::sleep(Duration::from_millis(700));
                save_help_shot(&grab_region(primary.0), &dir, "selection");
                b.key(VK_ESCAPE, &[]);
                mouse_button(false);
                std::thread::sleep(Duration::from_millis(600));
            }
            b.check(&format!("{lang} : 4 captures de l'aide"), ["accueil", "parametres", "editeur", "selection"].iter().all(|n| dir.join(format!("{n}.jpg")).is_file()), &dir.display().to_string());
            b.kill();
        }
        let _ = backdrop.kill();
        let _ = backdrop.wait();
        b.captures = real_captures;
        let _ = std::fs::remove_dir_all(&demo);
    }

    /// Fenêtre sans sa bordure invisible de redimensionnement (8 px à gauche, à droite et en bas).
    fn window_shot(r: (i32, i32, i32, i32)) -> image::RgbaImage {
        let s = scale_at(r.0 + r.2 / 2, r.1 + r.3 / 2);
        let e = (7.0 * s).round() as i32;
        grab_region((r.0 + e, r.1, r.2 - 2 * e, r.3 - e))
    }

    /// État de l'éditeur dans le journal : « éditeur formes=… numéros=[…] tampons=[…] ».
    fn editor_state(b: &Bench) -> String {
        let mark = b.log_mark();
        b.cmd("state", 600);
        b.log_since(mark).split("| éditeur ").nth(1).map(|s| s.lines().next().unwrap_or("").to_string()).unwrap_or_default()
    }

    fn double_click(x: i32, y: i32) {
        mouse_to(x, y);
        std::thread::sleep(Duration::from_millis(150));
        for _ in 0..2 {
            mouse_button(true);
            std::thread::sleep(Duration::from_millis(40));
            mouse_button(false);
            std::thread::sleep(Duration::from_millis(60));
        }
        std::thread::sleep(Duration::from_millis(400));
    }

    /// Fenêtre principale au centre de l'écran principal, au premier plan (tests à la souris).
    fn center_main(b: &Bench) {
        let primary = monitors().into_iter().find(|(m, _)| m.0 == 0 && m.1 == 0).or(monitors().into_iter().next()).unwrap();
        b.cmd("show", 700);
        if let Some(m) = b.main() {
            b.move_main(primary.1 .0 + (primary.1 .2 - m.r.2) / 2, primary.1 .1 + (primary.1 .3 - m.r.3) / 2);
        }
        b.cmd("show", 900);
    }

    fn s30_editeur_dupliquer_modifier(b: &mut Bench) {
        b.scenario("S30", "Éditeur : dupliquer (Ctrl+D) et modifier un numéro ou un tampon par double-clic");
        b.kill();
        for e in std::fs::read_dir(&b.captures).into_iter().flatten().flatten() {
            let _ = std::fs::remove_file(e.path());
        }
        demo_image(1).save(b.captures.join("Capture_demo.png")).unwrap();
        b.start("ShowWorkspace", false);
        center_main(b);
        b.cmd("editor", 1800);
        let Some(m) = b.main() else {
            b.check("éditeur ouvert", false, "");
            return;
        };
        let s = scale_at(m.r.0 + m.r.2 / 2, m.r.1 + m.r.3 / 2);
        let cx = m.r.0 as f32 + (m.r.2 as f32 - 300.0 * s) / 2.0 + 30.0 * s;
        let cy = m.r.1 as f32 + m.r.3 as f32 / 2.0 + 20.0 * s;
        let at = |dx: f32, dy: f32| ((cx + dx * s) as i32, (cy + dy * s) as i32);
        b.check("éditeur ouvert", editor_state(b).contains("formes=0"), &editor_state(b));

        // Deux numéros, puis le premier dupliqué : il prend le numéro suivant (3).
        if !b.key(0x4E, &[]) {
            return;
        }
        for (dx, dy) in [(-120.0, -60.0), (120.0, -60.0)] {
            let (x, y) = at(dx, dy);
            b.click(x, y, &[]);
            std::thread::sleep(Duration::from_millis(400));
        }
        b.key(0x56, &[]);
        let (x, y) = at(-120.0, -60.0);
        b.click(x, y, &[]);
        b.key(0x44, &[VK_CONTROL]);
        let st = editor_state(b);
        b.check("Ctrl+D : copie ajoutée avec le numéro suivant", st.contains("formes=3") && st.contains("numéros=[1,2,3]"), &st);

        // Double-clic sur le numéro 2 : saisie de 7 puis Entrée.
        let (x, y) = at(120.0, -60.0);
        // (pas de lecture de l'état ici : la commande de test ferait perdre le focus à la saisie)
        double_click(x, y);
        b.key(0x67, &[]); // pavé numérique : « 7 » quelle que soit la disposition du clavier (AZERTY)
        b.key(VK_RETURN, &[]);
        let st = editor_state(b);
        b.check("numéro modifié : 2 → 7", st.contains("numéros=[1,7,3]") && st.contains("édition=None"), &st);

        // Tampon posé puis renommé par double-clic.
        b.key(0x53, &[]);
        let (x, y) = at(0.0, 110.0);
        b.click(x, y, &[]);
        std::thread::sleep(Duration::from_millis(400));
        b.key(0x56, &[]);
        double_click(x, y);
        for k in [0x4F, 0x4B] {
            b.key(k, &[]);
        }
        b.key(VK_RETURN, &[]);
        let st = editor_state(b);
        b.check("double-clic sur un tampon : texte remplacé", st.contains("tampons=[ok]"), &st);
        b.shot("S30_editeur");
        // Rien d'enregistré : fermer l'éditeur sans enregistrer.
        b.kill();
    }

    fn s31_selection_par_case(b: &mut Bench) {
        b.scenario("S31", "Espace de travail : sélection par la case des vignettes et suppression groupée");
        b.kill();
        for e in std::fs::read_dir(&b.captures).into_iter().flatten().flatten() {
            let _ = std::fs::remove_file(e.path());
        }
        for i in 1..=4u32 {
            demo_image(i).save(b.captures.join(format!("Test_case_{i}.png"))).unwrap();
            std::thread::sleep(Duration::from_millis(60));
        }
        b.start("ShowWorkspace", false);
        center_main(b);
        std::thread::sleep(Duration::from_millis(1500));
        let mark = b.log_mark();
        b.cmd("cards", 700);
        let cards: Vec<(String, f32, f32, f32, f32)> = b
            .log_since(mark)
            .lines()
            .filter_map(|l| {
                let mut it = l.split("carte ").nth(1)?.splitn(6, ' ').skip(1);
                Some((0.0, 0.0, 0.0, 0.0)).and_then(|_| {
                    let (x, y, w, h) = (it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?);
                    Some((it.next()?.trim().to_string(), x, y, w, h))
                })
            })
            .collect();
        b.check("quatre vignettes", cards.len() == 4, &format!("{}", cards.len()));
        if cards.len() < 4 {
            return;
        }
        let s = scale_at(cards[0].1 as i32, cards[0].2 as i32);
        let check = |i: usize| ((cards[i].1 + 24.0 * s) as i32, (cards[i].2 + 24.0 * s) as i32);
        // Haut de la miniature, à droite de la case (loin de la barre d'actions du bas).
        let body = |i: usize| ((cards[i].1 + cards[i].3 * 0.6) as i32, (cards[i].2 + 34.0 * s) as i32);

        let (x, y) = check(0);
        b.click(x, y, &[]);
        b.check("clic sur la case : 1 sélectionnée", b.selected_count() == Some(1), &format!("{:?}", b.selected_count()));
        std::thread::sleep(Duration::from_millis(500));
        let (x, y) = body(2);
        b.click(x, y, &[]);
        b.check("puis clic simple sur une autre vignette : 2 sélectionnées", b.selected_count() == Some(2), &format!("{:?}", b.selected_count()));
        std::thread::sleep(Duration::from_millis(500));
        let (x, y) = body(3);
        b.click(x, y, &[]);
        std::thread::sleep(Duration::from_millis(500));
        let (x, y) = body(3);
        b.click(x, y, &[]);
        b.check("nouveau clic : la vignette est retirée (2 sélectionnées)", b.selected_count() == Some(2), &format!("{:?}", b.selected_count()));
        b.shot("S31_selection");
        let removed = [cards[0].0.clone(), cards[2].0.clone()];
        if b.key(VK_DELETE, &[]) {
            std::thread::sleep(Duration::from_millis(500));
            b.key(VK_RETURN, &[]);
            std::thread::sleep(Duration::from_millis(1500));
        }
        let files = b.capture_files("png");
        b.check("suppression groupée : les 2 captures sélectionnées", removed.iter().all(|r| !files.contains(r)) && files.len() == 2, &format!("{files:?}"));
    }

    fn s28_conservation(b: &mut Bench) {
        b.scenario("S28", "Conservation des captures : durée, épinglage, fichiers étrangers, taille maximale");
        b.kill();
        let dir = b.captures.clone();
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let _ = std::fs::remove_file(e.path());
        }
        dated_file(&dir, "Capture_A_3j.png", 3, 10);
        dated_file(&dir, "Capture_B_10j.png", 10, 10);
        dated_file(&dir, "Capture_C_40j.png", 40, 10);
        dated_file(&dir, "Capture_D_100j_epinglee.png", 100, 10);
        dated_file(&dir, "Collage_E_40j.png", 40, 10);
        dated_file(&dir, "Video_F_40j.mp4", 40, 10);
        dated_file(&dir, "Notes_personnelles_100j.png", 100, 10);
        // 1) Images : 30 jours, vidéos : jamais, suppression définitive, une capture épinglée.
        b.extra = vec![
            ("retention_images_days".into(), 30.into()),
            ("retention_to_trash".into(), false.into()),
            ("pinned".into(), serde_json::json!(["Capture_D_100j_epinglee.png"])),
        ];
        let mark = b.log_mark();
        b.start("ShowWorkspace", false);
        b.extra.clear();
        // Nettoyage automatique 15 s après le démarrage.
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(25) && !b.log_since(mark).contains("conservation :") {
            std::thread::sleep(Duration::from_millis(500));
        }
        let files = b.capture_files("png").into_iter().chain(b.capture_files("mp4")).collect::<Vec<_>>();
        let has = |n: &str| files.iter().any(|f| f == n);
        b.check("nettoyage automatique au démarrage", b.log_since(mark).contains("conservation :"), "");
        b.check("images de plus de 30 jours supprimées (capture et collage)", !has("Capture_C_40j.png") && !has("Collage_E_40j.png"), &format!("{files:?}"));
        b.check("images récentes conservées (3 et 10 jours)", has("Capture_A_3j.png") && has("Capture_B_10j.png"), "");
        b.check("capture épinglée conservée (100 jours)", has("Capture_D_100j_epinglee.png"), "");
        b.check("vidéo conservée (durée « Jamais » pour les vidéos)", has("Video_F_40j.mp4"), "");
        b.check("fichier personnel non concerné (100 jours)", has("Notes_personnelles_100j.png"), "");
        // 2) Épingler par l'application puis réduire la durée à 7 jours : la capture épinglée reste.
        b.cmd("pin Capture_B_10j.png", 600);
        b.check("épinglage enregistré", std::fs::read_to_string(&b.settings).unwrap_or_default().contains("Capture_B_10j.png"), "");
        b.kill();
        let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&b.settings).unwrap()).unwrap();
        v["retention_images_days"] = 7.into();
        std::fs::write(&b.settings, v.to_string()).unwrap();
        let child = Command::new(&b.exe).spawn().unwrap();
        b.pid = child.id();
        std::thread::sleep(Duration::from_secs(3));
        b.cmd("retention", 2000);
        let files = b.capture_files("png");
        b.check("7 jours : la capture de 10 jours épinglée reste", files.iter().any(|f| f == "Capture_B_10j.png"), &format!("{files:?}"));
        b.check("7 jours : la capture de 3 jours reste", files.iter().any(|f| f == "Capture_A_3j.png"), "");
        // 3) Taille maximale : 1 Mo pour 5 × 400 Ko, sans durée ; les plus anciennes partent.
        b.kill();
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let _ = std::fs::remove_file(e.path());
        }
        for (i, days) in [50u64, 40, 30, 20, 10].iter().enumerate() {
            dated_file(&dir, &format!("Capture_T{i}_{days}j.png"), *days, 400);
        }
        b.extra = vec![("retention_max_mb".into(), 1.into()), ("retention_to_trash".into(), false.into())];
        b.start("ShowWorkspace", false);
        b.extra.clear();
        b.cmd("retention", 2500);
        let files = b.capture_files("png");
        b.check("taille maximale : les 3 plus anciennes supprimées", files == vec!["Capture_T3_20j.png".to_string(), "Capture_T4_10j.png".to_string()], &format!("{files:?}"));
        // 4) Mode Corbeille : le fichier quitte le dossier (récupérable dans la Corbeille).
        b.kill();
        dated_file(&dir, "Capture_Z_60j_corbeille.png", 60, 1);
        b.extra = vec![("retention_images_days".into(), 30.into()), ("retention_to_trash".into(), true.into())];
        let mark = b.log_mark();
        b.start("ShowWorkspace", false);
        b.extra.clear();
        b.cmd("retention", 2500);
        b.check("mode Corbeille : capture ancienne retirée du dossier", !b.capture_files("png").iter().any(|f| f.starts_with("Capture_Z")), "");
        b.check("journal : suppression signalée", b.log_since(mark).contains("conservation : 1 capture"), "");
        b.check("interface réactive", b.responsive(3000), "");
    }
}
