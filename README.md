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

Version en cours de portage.
