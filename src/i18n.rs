//! Traduction de l'interface : français (langue du code), anglais, espagnol, allemand.
//!
//! Chaque texte affiché passe par `tr` (texte fixe) ou `trf` (texte avec valeurs `{0}`, `{1}`…) ;
//! le texte français sert de clé. Les traductions sont dans `locales/<langue>.tsv`
//! (« français<TAB>traduction », une par ligne). Une traduction absente affiche le français ;
//! le test `toutes_les_traductions` vérifie qu'il n'en manque aucune.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Langue choisie dans les Paramètres.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    /// Langue de Windows / macOS (français si elle n'est pas disponible).
    #[default]
    System,
    Fr,
    En,
    Es,
    De,
}

impl Language {
    pub const ALL: [Language; 5] = [Language::System, Language::Fr, Language::En, Language::Es, Language::De];

    /// Nom de la langue dans la langue elle-même (liste des Paramètres).
    pub fn native_name(self) -> &'static str {
        match self {
            Language::System => "",
            Language::Fr => "Français",
            Language::En => "English",
            Language::Es => "Español",
            Language::De => "Deutsch",
        }
    }

    /// Langue effective (`System` résolu d'après le système).
    pub fn resolve(self) -> Language {
        match self {
            Language::System => system_language(),
            l => l,
        }
    }
}

const LOCALES: [(Language, &str); 3] = [
    (Language::En, include_str!("../locales/en.tsv")),
    (Language::Es, include_str!("../locales/es.tsv")),
    (Language::De, include_str!("../locales/de.tsv")),
];

static CURRENT: AtomicU8 = AtomicU8::new(0);

fn code(l: Language) -> u8 {
    match l {
        Language::En => 1,
        Language::Es => 2,
        Language::De => 3,
        _ => 0,
    }
}

fn from_code(c: u8) -> Language {
    match c {
        1 => Language::En,
        2 => Language::Es,
        3 => Language::De,
        _ => Language::Fr,
    }
}

/// Langue du système (préfixe de la locale : « fr-FR » → français).
pub fn system_language() -> Language {
    let locale = sys_locale::get_locale().unwrap_or_default().to_lowercase();
    match locale.get(..2) {
        Some("en") => Language::En,
        Some("es") => Language::Es,
        Some("de") => Language::De,
        _ => Language::Fr,
    }
}

/// Applique la langue choisie (immédiatement, pour toute l'interface).
pub fn set_language(l: Language) {
    CURRENT.store(code(l.resolve()), Ordering::SeqCst);
}

/// Langue effective actuelle.
pub fn current() -> Language {
    from_code(CURRENT.load(Ordering::SeqCst))
}

/// Table d'une langue : français → traduction.
fn table(l: Language) -> Option<&'static HashMap<&'static str, &'static str>> {
    static TABLES: OnceLock<HashMap<u8, HashMap<&'static str, &'static str>>> = OnceLock::new();
    let tables = TABLES.get_or_init(|| {
        LOCALES
            .iter()
            .map(|(l, text)| {
                let map = text
                    .lines()
                    .filter(|line| !line.starts_with('#') && line.contains('\t'))
                    .filter_map(|line| line.split_once('\t'))
                    .map(|(fr, tr)| (unescape(fr), unescape(tr)))
                    .collect();
                (code(*l), map)
            })
            .collect()
    });
    tables.get(&code(l))
}

/// `\t` et `\n` écrits dans le fichier de traduction → tabulation et saut de ligne.
fn unescape(s: &'static str) -> &'static str {
    if !s.contains('\\') {
        return s;
    }
    Box::leak(s.replace("\\t", "\t").replace("\\n", "\n").into_boxed_str())
}

/// Texte traduit dans la langue actuelle.
pub fn tr(fr: &'static str) -> &'static str {
    match current() {
        Language::Fr | Language::System => fr,
        l => table(l).and_then(|t| t.get(fr).copied()).unwrap_or(fr),
    }
}

/// Texte traduit avec valeurs : `trf("Enregistré : {0}", &[&nom])`.
pub fn trf(fr: &'static str, args: &[&dyn Display]) -> String {
    let mut s = tr(fr).to_string();
    for (i, a) in args.iter().enumerate() {
        s = s.replace(&format!("{{{i}}}"), &a.to_string());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Textes passés à `tr` / `trf` dans le code source.
    fn keys_in_source() -> Vec<String> {
        let mut keys = Vec::new();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = vec![dir.clone()];
        while let Some(p) = files.pop() {
            if p.is_dir() {
                files.extend(std::fs::read_dir(&p).unwrap().flatten().map(|e| e.path()));
                continue;
            }
            if p.extension().is_none_or(|x| x != "rs") || p.ends_with("i18n.rs") {
                continue;
            }
            let src = std::fs::read_to_string(&p).unwrap();
            for marker in ["tr(\"", "trf(\""] {
                let mut rest = src.as_str();
                while let Some(i) = rest.find(marker) {
                    // `tr(` doit être un appel isolé (pas la fin d'un autre nom).
                    let before = rest[..i].chars().last();
                    let after = &rest[i + marker.len()..];
                    let mut key = String::new();
                    let mut chars = after.char_indices();
                    let mut end = after.len();
                    while let Some((j, c)) = chars.next() {
                        match c {
                            '\\' => match chars.next() {
                                Some((_, 'n')) => key.push('\n'),
                                Some((_, 't')) => key.push('\t'),
                                Some((_, x)) => key.push(x),
                                None => {}
                            },
                            '"' => {
                                end = j;
                                break;
                            }
                            c => key.push(c),
                        }
                    }
                    if !before.is_some_and(|b| b.is_alphanumeric() || b == '_') {
                        keys.push(key);
                    }
                    rest = &after[end..];
                }
            }
        }
        keys.sort();
        keys.dedup();
        keys
    }

    #[test]
    fn toutes_les_traductions() {
        let keys = keys_in_source();
        assert!(keys.len() > 100, "{} textes seulement", keys.len());
        for (l, _) in LOCALES {
            let t = table(l).unwrap();
            let missing: Vec<_> = keys.iter().filter(|k| !t.contains_key(k.as_str())).collect();
            assert!(missing.is_empty(), "{l:?} : {} traduction(s) manquante(s) : {missing:#?}", missing.len());
            // Mêmes valeurs {0}, {1}… dans la traduction que dans le français.
            for k in &keys {
                let tr = t[k.as_str()];
                for i in 0..6 {
                    let p = format!("{{{i}}}");
                    assert_eq!(k.contains(&p), tr.contains(&p), "{l:?} : « {k} » → « {tr} » ({p})");
                }
            }
        }
    }

    #[test]
    fn valeurs() {
        assert_eq!(trf("{0} et {1}", &[&1, &"deux"]), "1 et deux");
    }
}
