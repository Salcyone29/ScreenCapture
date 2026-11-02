//! Intégration système : l'implémentation dépend de la plateforme (Windows ou macOS).

#[cfg(windows)]
#[path = "platform_win.rs"]
mod imp;
#[cfg(target_os = "macos")]
#[path = "platform_mac.rs"]
mod imp;

pub use imp::*;

/// Modificateurs pour les raccourcis Impr. écran (masque).
pub const MOD_CTRL: u8 = 1;
pub const MOD_SHIFT: u8 = 2;
pub const MOD_ALT: u8 = 4;
pub const MOD_WIN: u8 = 8;

/// Bit ajouté au masque : l'évènement est Win+Maj+S (et non Impr. écran).
pub const KEY_SNIP: u8 = 0x40;
/// Bit ajouté au masque : ne pas déclencher, seulement indiquer s'il faut consommer la touche.
pub const NO_FIRE: u8 = 0x80;

/// Rappel du raccourci Impr. écran : reçoit le masque, renvoie `true` si la touche est consommée.
pub type PrintScreenHandler = Box<dyn Fn(u8) -> bool + Send + Sync>;

/// Libellé de raccourci pour la plateforme : « Ctrl+Maj+S » devient « ⌘⇧S » sur Mac.
pub fn sc(s: &str) -> String {
    if cfg!(target_os = "macos") {
        s.replace("Ctrl+", "⌘").replace("Maj+", "⇧").replace("Ctrl", "⌘")
    } else {
        s.to_string()
    }
}

/// Nom du gestionnaire de fichiers.
pub const FILE_MANAGER: &str = if cfg!(target_os = "macos") { "le Finder" } else { "l'Explorateur" };
