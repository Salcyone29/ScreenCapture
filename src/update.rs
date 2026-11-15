//! Mises à jour : recherche de la dernière version publiée sur GitHub, téléchargement vérifié
//! (empreinte SHA-256 publiée par GitHub) et installation.
//!
//! - Windows, version installée : l'installeur est lancé en mode silencieux, puis relance
//!   l'application (`/update=1`, voir installer/ScreenCapture.iss).
//! - Windows, version portable : le nouveau `ScreenCapture.exe` remplace l'ancien (renommé en
//!   `ScreenCapture.old.exe`, supprimé au démarrage suivant).
//! - macOS : le `.dmg` est téléchargé puis ouvert ; l'utilisateur glisse l'application.
//!
//! Le téléchargement passe par `curl`, présent sur Windows 10/11 et macOS.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

pub const REPO: &str = "Salcyone29/ScreenCapture";
const MARKER: &str = "ScreenCapture.portable";

/// Version portable : un fichier `ScreenCapture.portable` est à côté du programme.
/// Les réglages et journaux sont alors dans le dossier `data` voisin.
pub fn portable_dir() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    dir.join(MARKER).is_file().then_some(dir)
}

pub fn portable_data_dir() -> Option<PathBuf> {
    portable_dir().map(|d| d.join("data"))
}

/// Supprime l'ancien programme laissé par une mise à jour de la version portable.
pub fn cleanup_after_update() {
    if let Some(dir) = portable_dir() {
        let _ = std::fs::remove_file(dir.join("ScreenCapture.old.exe"));
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub page: String,
    pub asset_name: String,
    pub asset_url: String,
    pub asset_size: u64,
    /// Empreinte SHA-256 publiée par GitHub (en hexadécimal), si disponible.
    pub sha256: Option<String>,
}

/// « v0.1.12 » → (0, 1, 12).
pub fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let core = s.split(['-', '+']).next()?;
    let mut it = core.split('.').map(|x| x.parse::<u32>().ok());
    Some((it.next()??, it.next().flatten().unwrap_or(0), it.next().flatten().unwrap_or(0)))
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// Nom du fichier à télécharger pour cette installation.
fn wanted_asset(version: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("ScreenCapture-{version}-macOS.dmg")
    } else if portable_dir().is_some() {
        format!("ScreenCapture-{version}-portable.zip")
    } else {
        format!("ScreenCapture-Setup-{version}.exe")
    }
}

fn curl() -> std::process::Command {
    let mut c = crate::platform::hidden_command("curl");
    c.args(["-fsSL", "--retry", "2", "--connect-timeout", "15", "-H", "User-Agent: ScreenCapture-updater"]);
    c
}

/// Interprète la réponse de l'API GitHub « latest release ».
pub fn parse_release(json: &str, wanted: impl Fn(&str) -> String) -> Option<Release> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    if v["draft"].as_bool() == Some(true) || v["prerelease"].as_bool() == Some(true) {
        return None;
    }
    let tag = v["tag_name"].as_str()?;
    let version = tag.trim_start_matches(['v', 'V']).to_string();
    let name = wanted(&version);
    let asset = v["assets"].as_array()?.iter().find(|a| a["name"].as_str() == Some(name.as_str()))?;
    Some(Release {
        page: v["html_url"].as_str().unwrap_or_default().to_string(),
        asset_name: name,
        asset_url: asset["browser_download_url"].as_str()?.to_string(),
        asset_size: asset["size"].as_u64().unwrap_or(0),
        sha256: asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).map(|d| d.to_lowercase()),
        version,
    })
}

/// Dernière version publiée, si elle est plus récente que celle-ci (appel bloquant).
pub fn check() -> anyhow::Result<Option<Release>> {
    // SC_UPDATE_API : autre adresse (tests de validation, avec une réponse et des fichiers locaux en file://).
    let url = std::env::var("SC_UPDATE_API").unwrap_or_else(|_| format!("https://api.github.com/repos/{REPO}/releases/latest"));
    let out = curl().args(["-H", "Accept: application/vnd.github+json", &url]).output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        crate::diag::log(&format!("mise à jour : échec de la recherche ({:?}) : {err}", out.status.code()));
        anyhow::bail!("{}", explain_curl_error(out.status.code(), &err));
    }
    let json = String::from_utf8_lossy(&out.stdout);
    let release = parse_release(&json, wanted_asset);
    crate::diag::log(&format!("mise à jour : dernière version {:?}", release.as_ref().map(|r| &r.version)));
    Ok(release.filter(|r| is_newer(&r.version, env!("CARGO_PKG_VERSION"))))
}

/// Message compréhensible pour une erreur de curl (codes de sortie documentés par curl).
fn explain_curl_error(code: Option<i32>, stderr: &str) -> &'static str {
    match code {
        // Erreur HTTP : 404 = aucune version publiée accessible (dépôt privé ou sans version).
        Some(22) if stderr.contains("404") => crate::i18n::tr("aucune version publiée n'est accessible pour l'instant"),
        Some(22) if stderr.contains("403") || stderr.contains("429") => crate::i18n::tr("GitHub limite les recherches pour le moment, réessayez plus tard"),
        Some(22) => crate::i18n::tr("le serveur de GitHub a répondu par une erreur"),
        // Pas de résolution DNS, connexion impossible ou trop longue.
        Some(6) | Some(7) | Some(28) => crate::i18n::tr("pas de connexion à Internet"),
        // Certificat ou échange TLS.
        Some(35) | Some(60) => crate::i18n::tr("connexion sécurisée à GitHub impossible"),
        _ => crate::i18n::tr("réponse inattendue de GitHub"),
    }
}

fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    std::io::copy(&mut f, &mut h)?;
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Télécharge le fichier de la version et vérifie sa taille et son empreinte.
pub fn download(r: &Release) -> anyhow::Result<PathBuf> {
    let dir = std::env::temp_dir().join("ScreenCapture-update");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let file = dir.join(&r.asset_name);
    let st = curl().arg("-o").arg(&file).arg(&r.asset_url).status()?;
    anyhow::ensure!(st.success(), "{}", crate::i18n::tr("téléchargement impossible"));
    let len = std::fs::metadata(&file)?.len();
    anyhow::ensure!(r.asset_size == 0 || len == r.asset_size, "{}", crate::i18n::tr("fichier téléchargé incomplet"));
    if let Some(expected) = &r.sha256 {
        let got = sha256_file(&file)?;
        anyhow::ensure!(&got == expected, "{}", crate::i18n::tr("empreinte du fichier téléchargé incorrecte"));
    }
    crate::diag::log(&format!("mise à jour : {} téléchargé ({len} octets, empreinte {})", r.asset_name, if r.sha256.is_some() { "vérifiée" } else { "non publiée" }));
    Ok(file)
}

/// Lance l'installation du fichier téléchargé. Renvoie vrai si l'application doit se fermer.
pub fn install(file: &Path) -> anyhow::Result<bool> {
    #[cfg(windows)]
    {
        if let Some(dir) = portable_dir() {
            // Version portable : extraire le nouveau programme et remplacer l'actuel.
            let tmp = file.with_extension("extract");
            let _ = std::fs::remove_dir_all(&tmp);
            std::fs::create_dir_all(&tmp)?;
            let st = crate::platform::hidden_command("tar").arg("-xf").arg(file).arg("-C").arg(&tmp).status()?;
            anyhow::ensure!(st.success(), "{}", crate::i18n::tr("archive illisible"));
            let new_exe = find_file(&tmp, "ScreenCapture.exe").ok_or_else(|| anyhow::anyhow!("ScreenCapture.exe ?"))?;
            let exe = std::env::current_exe()?;
            let old = dir.join("ScreenCapture.old.exe");
            let _ = std::fs::remove_file(&old);
            std::fs::rename(&exe, &old)?;
            if let Err(e) = std::fs::copy(&new_exe, &exe) {
                let _ = std::fs::rename(&old, &exe);
                return Err(e.into());
            }
            for extra in ["LICENSE.txt", "THIRD-PARTY-LICENSES.txt"] {
                if let Some(f) = find_file(&tmp, extra) {
                    let _ = std::fs::copy(f, dir.join(extra));
                }
            }
            // Relancée une fois celle-ci fermée (instance unique).
            relaunch_later(&format!("start \"\" \"{}\"", exe.display()))?;
            return Ok(true);
        }
        // Version installée : installeur silencieux, lancé une fois l'application fermée
        // (l'installeur refuse de remplacer un programme en cours d'exécution).
        relaunch_later(&format!("\"{}\" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /update=1", file.display()))?;
        Ok(true)
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(file).spawn()?;
        Ok(false)
    }
}

/// Exécute `command` (interpréteur cmd) deux secondes plus tard, après la fermeture de l'application.
#[cfg(windows)]
fn relaunch_later(command: &str) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;
    crate::platform::hidden_command("cmd").raw_arg(format!("/C ping -n 3 127.0.0.1 >nul & {command}")).spawn()?;
    Ok(())
}

#[cfg(windows)]
fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(f) = find_file(&p, name) {
                return Some(f);
            }
        } else if p.file_name().is_some_and(|n| n.eq_ignore_ascii_case(name)) {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(parse_version("v0.1.12"), Some((0, 1, 12)));
        assert!(is_newer("0.1.12", "0.1.11"));
        assert!(is_newer("v0.2.0", "0.1.99"));
        assert!(!is_newer("0.1.11", "0.1.11"));
        assert!(!is_newer("0.1.9", "0.1.10"));
        assert!(!is_newer("n'importe quoi", "0.1.10"));
    }

    #[test]
    fn reponse_github() {
        let json = r#"{"tag_name":"v0.1.13","html_url":"https://github.com/x/y/releases/tag/v0.1.13","draft":false,"prerelease":false,
            "assets":[{"name":"ScreenCapture-0.1.13-macOS.dmg","browser_download_url":"https://e/a.dmg","size":10,"digest":"sha256:AB"},
                      {"name":"ScreenCapture-Setup-0.1.13.exe","browser_download_url":"https://e/s.exe","size":20,"digest":"sha256:CD"}]}"#;
        let r = parse_release(json, |v| format!("ScreenCapture-Setup-{v}.exe")).unwrap();
        assert_eq!(r.version, "0.1.13");
        assert_eq!(r.asset_url, "https://e/s.exe");
        assert_eq!(r.asset_size, 20);
        assert_eq!(r.sha256.as_deref(), Some("cd"));
        // Fichier absent de la version : pas de mise à jour proposée.
        assert!(parse_release(json, |v| format!("ScreenCapture-{v}-portable.zip")).is_none());
    }
}
