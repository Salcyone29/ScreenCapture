//! Conservation des captures : suppression automatique des anciennes captures (durée par type)
//! et plafond de taille du dossier, avec garde-fous.
//!
//! Garde-fous : seuls les fichiers créés par ScreenCapture (`Capture_…`, `Video_…`, `Collage_…`)
//! sont concernés ; les captures épinglées et celle ouverte dans l'éditeur ne sont jamais
//! supprimées ; l'âge se compte depuis la création du fichier (une retouche ne le remet pas à zéro).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::settings::Settings;

const PREFIXES: [&str; 3] = ["Capture_", "Video_", "Collage_"];
const IMAGE_EXT: [&str; 5] = ["png", "jpg", "jpeg", "bmp", "gif"];
const VIDEO_EXT: [&str; 3] = ["mp4", "webm", "mkv"];

/// Pourquoi un fichier est supprimé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// Plus ancien que la durée de conservation.
    Age,
    /// Le dossier dépasse la taille maximale (les plus anciens partent en premier).
    Size,
}

#[derive(Clone, Debug)]
pub struct Candidate {
    pub path: PathBuf,
    pub bytes: u64,
    pub reason: Reason,
}

/// Fichiers à supprimer selon les réglages.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub files: Vec<Candidate>,
}

impl Plan {
    pub fn bytes(&self) -> u64 {
        self.files.iter().map(|f| f.bytes).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// Une règle de conservation est active.
pub fn enabled(s: &Settings) -> bool {
    s.retention_images_days > 0 || s.retention_videos_days > 0 || s.retention_max_mb > 0
}

/// Fichier géré par ScreenCapture (nom et extension).
fn managed(path: &Path) -> Option<bool> {
    let name = path.file_name()?.to_string_lossy();
    if !PREFIXES.iter().any(|p| name.starts_with(p)) || name.ends_with(".tmp.mp4") {
        return None;
    }
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    if IMAGE_EXT.contains(&ext.as_str()) {
        Some(false)
    } else if VIDEO_EXT.contains(&ext.as_str()) {
        Some(true)
    } else {
        None
    }
}

pub fn is_pinned(s: &Settings, path: &Path) -> bool {
    path.file_name().is_some_and(|n| s.pinned.iter().any(|p| p.as_str() == n.to_string_lossy()))
}

/// Calcule ce qui serait supprimé maintenant. `keep` : fichier à ne jamais supprimer (éditeur).
pub fn plan(s: &Settings, keep: Option<&Path>) -> Plan {
    plan_at(s, keep, SystemTime::now())
}

fn plan_at(s: &Settings, keep: Option<&Path>, now: SystemTime) -> Plan {
    struct File {
        path: PathBuf,
        bytes: u64,
        created: SystemTime,
        video: bool,
        protected: bool,
    }
    let mut files: Vec<File> = std::fs::read_dir(&s.save_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let video = managed(&path)?;
            let meta = e.metadata().ok().filter(|m| m.is_file())?;
            let created = meta.created().or_else(|_| meta.modified()).unwrap_or(now);
            let protected = is_pinned(s, &path) || keep.is_some_and(|k| k == path);
            Some(File { path, bytes: meta.len(), created, video, protected })
        })
        .collect();
    // Les plus anciennes d'abord.
    files.sort_by_key(|f| f.created);

    let day = Duration::from_secs(24 * 3600);
    let too_old = |f: &File| {
        let days = if f.video { s.retention_videos_days } else { s.retention_images_days };
        days > 0 && now.duration_since(f.created).unwrap_or_default() > day * days
    };
    let mut out: Vec<Candidate> = Vec::new();
    let mut remaining = 0u64;
    let mut kept: Vec<&File> = Vec::new();
    for f in &files {
        if !f.protected && too_old(f) {
            out.push(Candidate { path: f.path.clone(), bytes: f.bytes, reason: Reason::Age });
        } else {
            remaining += f.bytes;
            kept.push(f);
        }
    }
    if s.retention_max_mb > 0 {
        let max = s.retention_max_mb * 1024 * 1024;
        for f in kept {
            if remaining <= max {
                break;
            }
            if f.protected {
                continue;
            }
            remaining -= f.bytes;
            out.push(Candidate { path: f.path.clone(), bytes: f.bytes, reason: Reason::Size });
        }
    }
    Plan { files: out }
}

/// Supprime les fichiers du plan (Corbeille ou définitivement). Renvoie (nombre, octets, erreurs).
pub fn execute(plan: &Plan, to_trash: bool) -> (usize, u64, Vec<String>) {
    let (mut n, mut bytes, mut errors) = (0, 0, Vec::new());
    for f in &plan.files {
        let r = if to_trash { crate::platform::move_to_recycle_bin(&f.path) } else { std::fs::remove_file(&f.path).map_err(anyhow::Error::from) };
        match r {
            Ok(()) => {
                n += 1;
                bytes += f.bytes;
            }
            Err(e) => errors.push(format!("{} : {e}", f.path.display())),
        }
    }
    (n, bytes, errors)
}

/// « 42 captures (380 Mo) ».
pub fn describe(n: usize, bytes: u64) -> String {
    let size = if bytes >= 1 << 30 {
        crate::i18n::trf("{0} Go", &[&(format!("{:.1}", bytes as f64 / (1u64 << 30) as f64))])
    } else if bytes >= 1 << 20 {
        crate::i18n::trf("{0} Mo", &[&(format!("{:.0}", bytes as f64 / (1u64 << 20) as f64))])
    } else {
        crate::i18n::trf("{0} Ko", &[&((bytes + 1023) / 1024)])
    };
    if n > 1 { crate::i18n::trf("{0} captures ({1})", &[&n, &size]) } else { crate::i18n::trf("{0} capture ({1})", &[&n, &size]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(name: &str) -> (Settings, PathBuf) {
        let dir = std::env::temp_dir().join(format!("sc_retention_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let s = Settings { save_dir: dir.clone(), ..Settings::default() };
        (s, dir)
    }

    fn file(dir: &Path, name: &str, kb: usize) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, vec![0u8; kb * 1024]).unwrap();
        p
    }

    #[test]
    fn ages_pins_and_foreign_files() {
        let (mut s, dir) = setup("ages");
        file(&dir, "Capture_old.png", 1);
        file(&dir, "Capture_pinned.png", 1);
        file(&dir, "Video_old.mp4", 1);
        file(&dir, "Notes.png", 1);
        s.retention_images_days = 30;
        s.pinned = vec!["Capture_pinned.png".into()];
        // 40 jours plus tard : seule l'image non épinglée et gérée part (vidéos : jamais).
        let later = SystemTime::now() + Duration::from_secs(40 * 24 * 3600);
        let p = plan_at(&s, None, later);
        let names: Vec<_> = p.files.iter().map(|f| f.path.file_name().unwrap().to_string_lossy().to_string()).collect();
        assert_eq!(names, vec!["Capture_old.png"]);
        // Le fichier ouvert dans l'éditeur est protégé.
        let p = plan_at(&s, Some(&dir.join("Capture_old.png")), later);
        assert!(p.is_empty());
    }

    #[test]
    fn max_size_removes_oldest_first() {
        let (mut s, dir) = setup("size");
        for i in 0..5 {
            file(&dir, &format!("Capture_{i}.png"), 400);
            std::thread::sleep(Duration::from_millis(30));
        }
        s.retention_max_mb = 1;
        let p = plan(&s, None);
        // 5 × 400 Ko = 2000 Ko > 1024 Ko : les 3 plus anciennes partent.
        let names: Vec<_> = p.files.iter().map(|f| f.path.file_name().unwrap().to_string_lossy().to_string()).collect();
        assert_eq!(names, vec!["Capture_0.png", "Capture_1.png", "Capture_2.png"]);
        assert!(p.files.iter().all(|f| f.reason == Reason::Size));
    }
}
