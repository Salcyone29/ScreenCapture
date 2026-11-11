//! Sonde technique (Windows) : une fenêtre eframe masquée par DWM continue-t-elle d'être
//! mise à jour, et une fenêtre secondaire (viewport) peut-elle être placée au pixel près ?
//! Écrit son déroulement dans le fichier passé en argument.
//! Usage : `cargo run --release --example cloak_probe -- <fichier>`

use std::io::Write;
use std::time::{Duration, Instant};

use eframe::egui;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

fn main() -> eframe::Result<()> {
    let out = std::env::args().nth(1).unwrap_or_else(|| "cloak_probe.txt".into());
    eframe::run_native(
        "Sonde",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_title("Sonde racine").with_inner_size([500.0, 300.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            let hwnd = match cc.window_handle().map(|h| h.as_raw()) {
                Ok(RawWindowHandle::Win32(h)) => h.hwnd.get(),
                _ => 0,
            };
            Ok(Box::new(Probe { hwnd, start: Instant::now(), frames: 0, log: std::fs::File::create(out).unwrap(), phase: 0 }))
        }),
    )
}

struct Probe {
    hwnd: isize,
    start: Instant,
    frames: u64,
    log: std::fs::File,
    phase: u8,
}

#[cfg(windows)]
fn cloak(hwnd: isize, on: bool) -> i32 {
    use windows::Win32::Foundation::{BOOL, HWND};
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CLOAK};
    let v = BOOL::from(on);
    unsafe {
        match DwmSetWindowAttribute(HWND(hwnd as *mut _), DWMWA_CLOAK, &v as *const _ as *const _, 4) {
            Ok(()) => 0,
            Err(e) => e.code().0,
        }
    }
}

#[cfg(windows)]
fn find(title: &str) -> Option<(isize, (i32, i32, i32, i32))> {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowRect};
    unsafe {
        let h = FindWindowW(None, &HSTRING::from(title)).ok()?;
        let mut r = RECT::default();
        let _ = GetWindowRect(h, &mut r);
        Some((h.0 as isize, (r.left, r.top, r.right, r.bottom)))
    }
}

#[cfg(windows)]
fn place(hwnd: isize, x: i32, y: i32, w: i32, h: i32) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE};
    unsafe {
        let _ = SetWindowPos(HWND(hwnd as *mut _), HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE);
    }
}

impl eframe::App for Probe {
    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        self.frames += 1;
        let t = self.start.elapsed().as_secs_f32();
        let _ = writeln!(self.log, "{t:.2} image {}", self.frames);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("Sonde : {:.1} s", t));
        });
        if self.phase == 0 && t > 2.0 {
            self.phase = 1;
            let _ = writeln!(self.log, "{t:.2} MASQUAGE racine hr={}", cloak(self.hwnd, true));
        }
        if self.phase >= 1 && self.phase < 3 && t > 3.0 {
            // Fenêtre secondaire placée au pixel près, pendant que la racine est masquée.
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("sonde_overlay"),
                egui::ViewportBuilder::default()
                    .with_title("Sonde overlay")
                    .with_decorations(false)
                    .with_taskbar(false)
                    .with_always_on_top()
                    .with_inner_size([300.0, 200.0]),
                |ctx, _| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.heading("overlay");
                    });
                },
            );
            if self.phase == 1 {
                if let Some((h, r)) = find("Sonde overlay") {
                    let _ = writeln!(self.log, "{t:.2} overlay trouvée {h:#x} rect avant {r:?}");
                    place(h, 100, 100, 640, 360);
                    self.phase = 2;
                }
            } else if let Some((_, r)) = find("Sonde overlay") {
                let _ = writeln!(self.log, "{t:.2} overlay rect {r:?}");
            }
            ctx.request_repaint();
        }
        if self.phase == 2 && t > 6.0 {
            self.phase = 3;
            let _ = writeln!(self.log, "{t:.2} fin overlay");
        }
        if self.phase == 3 && t > 8.0 {
            self.phase = 4;
            let _ = writeln!(self.log, "{t:.2} DÉMASQUAGE racine hr={}", cloak(self.hwnd, false));
        }
        if t > 10.0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        // Une image toutes les 250 ms : permet de voir si la racine masquée continue de tourner.
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}
