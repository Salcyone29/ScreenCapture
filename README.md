# ScreenCapture

Outil de capture d'écran, d'annotation et d'enregistrement vidéo écrit en Rust (eframe/egui), avec une interface Material Design 3 en français.

## Fonctionnalités

- Capture de zone, plein écran, fenêtre, dernière zone et capture différée
- Éditeur : flèches, formes, texte, bulles, surligneur, flou, loupe, numérotation (1, 01, A, a, I, i, α), tampons, recadrage, découpe de bande (droite ou zigzag), déplacement de zone, pipette, nuancier personnalisable
- Reconnaissance de texte (OCR)
- Enregistrement vidéo MP4 / WebM / GIF via FFmpeg, avec micro
- Historique des captures, raccourcis globaux, icône dans la zone de notification

## Windows

Télécharger l'installeur `ScreenCapture-Setup-x.y.z.exe` dans les [Releases](../../releases). Il propose d'installer FFmpeg s'il est absent.

### Compiler

Prérequis : [Rust](https://rustup.rs) et, pour l'installeur, [Inno Setup 6](https://jrsoftware.org/isinfo.php).

```powershell
cargo build --release          # target\release\ScreenCapture.exe
.\build-installer.ps1          # dist\ScreenCapture-Setup-x.y.z.exe
```

## macOS

Télécharger `ScreenCapture-x.y.z-macOS.dmg` dans les [Releases](../../releases) (binaire universel Apple Silicon et Intel, macOS 11 ou plus récent), puis glisser ScreenCapture dans Applications.

- L'application n'est pas signée par un compte Apple Developer : au premier lancement, faire **clic droit → Ouvrir**, puis **Ouvrir**.
- Autoriser ScreenCapture dans **Réglages Système → Confidentialité et sécurité → Enregistrement de l'écran** (et **Micro** pour le son des vidéos), puis relancer l'application.
- Vidéo : installer FFmpeg avec `brew install ffmpeg`.
- Raccourcis par défaut : ⌘⇧2 zone, ⌘⇧1 plein écran, ⌘⇧7 fenêtre, ⌘⇧8 dernière zone, ⌘⇧9 vidéo (⌘⇧3/4/5 restent ceux de macOS).
- Une capture de zone porte sur l'écran où se trouve le pointeur.

### Compiler

Prérequis : [Rust](https://rustup.rs) et les outils de ligne de commande Xcode (`xcode-select --install`).

```bash
./build-macos.sh            # binaire universel → dist/ScreenCapture-x.y.z-macOS.dmg
./build-macos.sh --native   # architecture du Mac uniquement, plus rapide
```

## Compilation automatique

Le workflow GitHub Actions `Build` compile l'installeur Windows et le `.dmg` macOS : lancement manuel depuis l'onglet Actions, ou automatique à chaque tag `vX.Y.Z`, qui joint les fichiers à la Release.
