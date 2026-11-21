//! Capture avec défilement : une zone plus haute que l'écran (page web, document…).
//!
//! La zone choisie est capturée, la page défile (molette simulée, ou défilement fait à la main
//! quand le système ne permet pas de simuler la molette), puis chaque nouvelle image est recalée
//! sur la précédente en comparant ses lignes de pixels : seules les lignes nouvelles sont ajoutées.
//! Les en-têtes et pieds de page fixes (qui ne défilent pas) ne sont gardés qu'une fois.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use image::RgbaImage;

use crate::capture::{self, Rect};

/// Hauteur maximale de l'image assemblée (pixels).
const MAX_HEIGHT: u32 = 30_000;
/// Durée maximale d'une capture défilante.
const MAX_DURATION: Duration = Duration::from_secs(180);

/// Nombre de blocs de colonnes résumant une ligne.
const BLOCKS: usize = 48;

/// Résumé d'une ligne de pixels : luminosité moyenne de `BLOCKS` blocs de colonnes. Une page peut
/// défiler d'une fraction de pixel (mise à l'échelle de l'écran) : les lignes ne sont alors jamais
/// identiques au pixel près, d'où une comparaison approchée.
struct RowSig {
    blocks: [f32; BLOCKS],
    /// Ligne unie (fond blanc…) : ne permet pas de recaler deux images.
    uniform: bool,
}

fn row_sigs(img: &RgbaImage) -> Vec<RowSig> {
    let w = img.width() as usize;
    // La barre de défilement (à droite) change d'une image à l'autre : ignorée.
    let skip = if w > 100 { 24.min(w / 10) } else { 0 };
    let span = (w - skip).max(1);
    let raw = img.as_raw();
    (0..img.height() as usize)
        .map(|y| {
            let row = &raw[y * w * 4..(y * w + span) * 4];
            let mut blocks = [0f32; BLOCKS];
            let mut counts = [0u32; BLOCKS];
            let (mut lo, mut hi) = (255u8, 0u8);
            for (x, p) in row.chunks_exact(4).enumerate() {
                let l = ((p[0] as u32 * 54 + p[1] as u32 * 183 + p[2] as u32 * 19) >> 8) as u8;
                lo = lo.min(l);
                hi = hi.max(l);
                let b = x * BLOCKS / span;
                blocks[b] += l as f32;
                counts[b] += 1;
            }
            for (v, c) in blocks.iter_mut().zip(counts) {
                *v /= c.max(1) as f32;
            }
            RowSig { blocks, uniform: hi - lo < 6 }
        })
        .collect()
}

fn dist(a: &RowSig, b: &RowSig) -> f32 {
    a.blocks.iter().zip(&b.blocks).map(|(x, y)| (x - y).abs()).sum::<f32>() / BLOCKS as f32
}

/// Recalage de la nouvelle image sur la précédente.
#[derive(Debug, PartialEq, Eq)]
pub struct Shift {
    /// Nombre de lignes dont la page a défilé (0 : rien n'a bougé).
    pub rows: usize,
    /// Lignes fixes en bas de la zone (pied de page qui ne défile pas).
    pub footer: usize,
}

/// Écart moyen toléré entre deux lignes « identiques » (niveaux de luminosité).
const SAME: f32 = 1.5;
/// Écart moyen maximal d'un recalage accepté.
const MATCH: f32 = 4.0;

/// Décalage vertical entre `prev` et `cur` (mêmes dimensions). `None` : aucun recalage fiable.
pub fn find_shift(prev: &RgbaImage, cur: &RgbaImage) -> Option<Shift> {
    if prev.dimensions() != cur.dimensions() {
        return None;
    }
    if prev.as_raw() == cur.as_raw() {
        return Some(Shift { rows: 0, footer: 0 });
    }
    let (sp, sc) = (row_sigs(prev), row_sigs(cur));
    let h = sp.len();
    // Lignes fixes en haut et en bas (pareilles au même endroit).
    let header = (0..h).take_while(|&i| dist(&sp[i], &sc[i]) < SAME).count();
    let footer = (0..h).take_while(|&i| dist(&sp[h - 1 - i], &sc[h - 1 - i]) < SAME).count();
    if header + footer >= h {
        return Some(Shift { rows: 0, footer: 0 });
    }
    let mut best: Option<(f32, usize)> = None;
    for s in 1..h - header - footer {
        let (mut seen, mut total) = (0usize, 0f32);
        for i in header..h - s - footer {
            if !sc[i].uniform {
                seen += 1;
                total += dist(&sc[i], &sp[i + s]);
            }
        }
        if seen < 8 {
            continue;
        }
        let score = total / seen as f32;
        if best.is_none_or(|(b, _)| score < b) {
            best = Some((score, s));
        }
    }
    match best {
        Some((score, rows)) if score < MATCH => Some(Shift { rows, footer }),
        _ => None,
    }
}

/// Image en cours d'assemblage.
pub struct Stitcher {
    width: u32,
    rows: Vec<u8>,
    prev: RgbaImage,
    /// Images reçues après la première.
    pushes: usize,
}

impl Stitcher {
    pub fn new(first: RgbaImage) -> Self {
        Self { width: first.width(), rows: first.as_raw().clone(), prev: first, pushes: 0 }
    }

    pub fn height(&self) -> u32 {
        (self.rows.len() / (self.width as usize * 4)) as u32
    }

    /// Ajoute une nouvelle image. Renvoie le recalage trouvé (`None` : image écartée).
    pub fn push(&mut self, cur: RgbaImage) -> Option<Shift> {
        self.pushes += 1;
        let shift = find_shift(&self.prev, &cur)?;
        if shift.rows > 0 {
            let stride = self.width as usize * 4;
            let h = cur.height() as usize;
            // Le pied de page fixe déjà ajouté est remplacé : il reste une seule fois, en bas.
            let keep = self.rows.len() - shift.footer * stride;
            self.rows.truncate(keep);
            let from = h - shift.rows - shift.footer;
            self.rows.extend_from_slice(&cur.as_raw()[from * stride..]);
            self.prev = cur;
        }
        Some(shift)
    }

    pub fn finish(self) -> RgbaImage {
        let h = self.height();
        RgbaImage::from_raw(self.width, h, self.rows).expect("taille de l'image assemblée")
    }
}

/// Suivi d'une capture défilante, partagé avec l'interface (barre « Terminer »).
#[derive(Clone, Default)]
pub struct Progress {
    pub stop: Arc<AtomicBool>,
    pub height: Arc<AtomicU32>,
    /// L'utilisateur fait défiler lui-même (molette simulée arrêtée, ou impossible sur ce Mac).
    pub manual: Arc<AtomicBool>,
}

/// Capture qui ne change plus (la page a fini de défiler) : deux images identiques de suite.
pub fn capture_settled(r: Rect) -> anyhow::Result<RgbaImage> {
    let mut last = capture::capture_rect(r, false)?;
    for _ in 0..5 {
        std::thread::sleep(Duration::from_millis(90));
        let img = capture::capture_rect(r, false)?;
        if img.as_raw() == last.as_raw() {
            return Ok(img);
        }
        last = img;
    }
    Ok(last)
}

/// Capture défilante de la zone `r` (bloquante : à lancer dans un thread).
/// `held` : zone dessinée en tournant la molette, bouton gauche maintenu. L'utilisateur fait
/// défiler lui-même la page ; la capture se termine quand il relâche le bouton. `first_notches` :
/// crans déjà tournés pendant la sélection, transmis à la page. `first` : la zone telle que
/// l'utilisateur la voyait en la dessinant (point de départ de l'image).
pub fn run(r: Rect, progress: Progress, held: bool, first_notches: i32, first: Option<RgbaImage>) -> anyhow::Result<RgbaImage> {
    let mut auto = !held && crate::platform::can_simulate_scroll();
    progress.manual.store(!auto, Ordering::SeqCst);
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    let cursor_before = capture::cursor_pos();
    crate::diag::log(&format!("capture défilante {r:?} (molette simulée : {auto})"));
    // Laisser disparaître la sélection.
    std::thread::sleep(Duration::from_millis(if held { 120 } else { 250 }));
    let first = match first.filter(|f| f.width() == r.w as u32 && f.height() == r.h as u32) {
        Some(f) => f,
        None => capture_settled(r)?,
    };
    let mut stitch = Stitcher::new(first);
    progress.height.store(stitch.height(), Ordering::SeqCst);
    if held && first_notches > 0 {
        // Crans reçus par la sélection avant sa fermeture : à la page, sous le pointeur.
        crate::platform::send_wheel(first_notches);
    }
    let started = Instant::now();
    let mut notches: i32 = 2;
    let (mut still, mut lost) = (0, 0);
    // L'utilisateur reprend la main : la molette simulée s'arrête et l'assemblage suit son défilement.
    let take_over = |why: &str| {
        crate::diag::log(&format!("capture défilante : défilement à la main ({why})"));
        progress.manual.store(true, Ordering::SeqCst);
    };
    while !progress.stop.load(Ordering::SeqCst) && started.elapsed() < MAX_DURATION && stitch.height() < MAX_HEIGHT {
        // Bouton relâché : dernière image, puis fin.
        let released = held && !crate::platform::left_button_down();
        if auto {
            // Pointeur déplacé depuis la dernière molette simulée : la souris est dans la main.
            let (x, y) = capture::cursor_pos();
            if notches_sent(&stitch) && ((x - cx).abs() > 3 || (y - cy).abs() > 3) {
                auto = false;
                take_over("souris déplacée");
            }
        }
        let cur = if auto {
            crate::platform::scroll_wheel(cx, cy, notches);
            std::thread::sleep(Duration::from_millis(250));
            capture_settled(r)?
        } else {
            // À la main : images rapprochées pour garder un recouvrement entre deux crans.
            std::thread::sleep(Duration::from_millis(if held { 40 } else { 70 }));
            if released {
                capture_settled(r)?
            } else {
                capture::capture_rect(r, false)?
            }
        };
        match stitch.push(cur) {
            Some(Shift { rows: 0, .. }) => {
                // Bas de la page atteint (rien ne bouge malgré la molette).
                still += 1;
                if auto && still >= 2 {
                    break;
                }
            }
            Some(s) => {
                still = 0;
                lost = 0;
                // Défiler d'environ la moitié de la zone à chaque fois, pour garder un recouvrement.
                let per_notch = (s.rows as f32 / notches as f32).max(1.0);
                notches = ((r.h as f32 * 0.45 / per_notch) as i32).clamp(1, 10);
                progress.height.store(stitch.height(), Ordering::SeqCst);
            }
            None => {
                // Contenu qui change tout seul (animation), défilement trop grand, ou molette
                // tournée par l'utilisateur en même temps : on passe à la main plutôt que d'arrêter.
                lost += 1;
                if auto {
                    notches = (notches / 2).max(1);
                    if lost >= 2 {
                        auto = false;
                        take_over("recalage perdu");
                    }
                }
            }
        }
        if released {
            break;
        }
    }
    if !progress.manual.load(Ordering::SeqCst) {
        crate::platform::move_cursor(cursor_before.0, cursor_before.1);
    }
    let img = stitch.finish();
    crate::diag::log(&format!("capture défilante terminée : {}×{}", img.width(), img.height()));
    Ok(img)
}

/// Une molette simulée a déjà placé le pointeur au centre de la zone (le premier tour ne compte
/// pas : le pointeur est encore là où l'utilisateur a cliqué).
fn notches_sent(stitch: &Stitcher) -> bool {
    stitch.pushes > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Page de test : chaque ligne a un motif unique.
    fn page(h: u32) -> RgbaImage {
        RgbaImage::from_fn(200, h, |x, y| {
            let v = ((y * 7919 + x * 31) % 251) as u8;
            image::Rgba([v, (y % 256) as u8, (y / 256) as u8, 255])
        })
    }

    fn view(p: &RgbaImage, top: u32, h: u32) -> RgbaImage {
        image::imageops::crop_imm(p, 0, top, p.width(), h).to_image()
    }

    #[test]
    fn assemble_une_page() {
        let p = page(1000);
        let mut st = Stitcher::new(view(&p, 0, 300));
        for top in [120, 250, 400, 560, 700, 700] {
            st.push(view(&p, top, 300)).expect("recalage");
        }
        let out = st.finish();
        assert_eq!(out.height(), 1000);
        assert_eq!(out.as_raw(), p.as_raw());
    }

    #[test]
    fn entete_et_pied_fixes() {
        let p = page(1000);
        let fixed = |top: u32| {
            let mut v = view(&p, top, 300);
            for y in 0..40 {
                for x in 0..200 {
                    v.put_pixel(x, y, image::Rgba([10, 200, (x % 256) as u8, 255]));
                    v.put_pixel(x, 299 - y, image::Rgba([200, 10, (x % 256) as u8, 255]));
                }
            }
            v
        };
        let mut st = Stitcher::new(fixed(0));
        for top in [100, 200, 300] {
            let s = st.push(fixed(top)).expect("recalage");
            assert_eq!((s.rows, s.footer), (100, 40));
        }
        let out = st.finish();
        assert_eq!(out.height(), 600);
        // En-tête une fois en haut, pied une fois en bas, contenu continu entre les deux.
        assert_eq!(out.get_pixel(5, 0).0[1], 200);
        assert_eq!(out.get_pixel(5, 599).0[0], 200);
        assert_eq!(out.get_pixel(5, 300), p.get_pixel(5, 300));
        assert_eq!(out.get_pixel(5, 500), p.get_pixel(5, 500));
    }

    #[test]
    fn rien_ne_bouge_et_image_differente() {
        let p = page(600);
        assert_eq!(find_shift(&view(&p, 0, 300), &view(&p, 0, 300)), Some(Shift { rows: 0, footer: 0 }));
        let other = RgbaImage::from_fn(200, 300, |x, y| image::Rgba([(x * y % 255) as u8, 3, 9, 255]));
        assert_eq!(find_shift(&view(&p, 0, 300), &other), None);
    }

    /// Essai réel (Windows) : page de test `examples/scroll_page`, défilée par la molette simulée.
    /// `SC_SCROLL_PAGE` : programme de la page ; `SC_SCROLL_OUT` : image assemblée.
    #[test]
    #[ignore]
    fn page_reelle() {
        // Comme l'application (winit) : coordonnées en pixels physiques sur tous les écrans.
        #[cfg(windows)]
        unsafe {
            use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        let exe = std::env::var("SC_SCROLL_PAGE").expect("SC_SCROLL_PAGE");
        let out = std::env::var("SC_SCROLL_OUT").expect("SC_SCROLL_OUT");
        let rect_file = std::env::temp_dir().join("sc_scroll_rect.txt");
        let _ = std::fs::remove_file(&rect_file);
        let mut child = std::process::Command::new(exe).arg(&rect_file).spawn().unwrap();
        let start = Instant::now();
        while !rect_file.exists() && start.elapsed() < Duration::from_secs(15) {
            std::thread::sleep(Duration::from_millis(100));
        }
        std::thread::sleep(Duration::from_millis(1500));
        let v: Vec<i32> = std::fs::read_to_string(&rect_file).unwrap().split_whitespace().map(|x| x.parse().unwrap()).collect();
        let r = Rect::new(v[0], v[1], v[2], v[3]);
        let res = run(r, Progress::default(), false, 0, None);
        let _ = child.kill();
        let img = res.unwrap();
        img.save(&out).unwrap();
        assert!(img.height() > r.h as u32 * 3, "{}×{} pour une zone de {}×{}", img.width(), img.height(), r.w, r.h);
    }

    /// Essai réel sur une zone donnée (`SC_SCROLL_RECT` = « x y l h », pixels physiques) : page
    /// d'un navigateur ouverte par le script d'essai. `SC_SCROLL_OUT` : image assemblée.
    #[test]
    #[ignore]
    fn zone_reelle() {
        #[cfg(windows)]
        unsafe {
            use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        let v: Vec<i32> = std::env::var("SC_SCROLL_RECT").unwrap().split_whitespace().map(|x| x.parse().unwrap()).collect();
        let r = Rect::new(v[0], v[1], v[2], v[3]);
        let progress = Progress::default();
        if let Some(secs) = std::env::var("SC_SCROLL_STOP_AFTER").ok().and_then(|v| v.parse::<u64>().ok()) {
            let stop = progress.stop.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(secs));
                stop.store(true, Ordering::SeqCst);
            });
        }
        let held = std::env::var_os("SC_SCROLL_HELD").is_some();
        let img = run(r, progress.clone(), held, 0, None).unwrap();
        eprintln!("A LA MAIN : {}", progress.manual.load(Ordering::SeqCst));
        img.save(std::env::var("SC_SCROLL_OUT").unwrap()).unwrap();
        eprintln!("RESULTAT {}x{} pour {}x{}", img.width(), img.height(), r.w, r.h);
    }

}
