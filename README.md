# ScreenCapture

Outil de capture d'écran, d'annotation et d'enregistrement vidéo écrit en Rust (eframe/egui), avec une interface Material Design 3 en français, anglais, espagnol et allemand.

## Fonctionnalités

- Capture de zone, plein écran, fenêtre, dernière zone et capture différée
- Capture avec défilement : page plus haute que l'écran, assemblée automatiquement
- Coller une image (Ctrl+V) ou glisser des fichiers dans l'espace de travail : ils sont ajoutés au dossier des captures
- Éditeur : flèches, formes, texte, bulles, surligneur, flou, loupe, numérotation (1, 01, A, a, I, i, α), tampons, recadrage, découpe de bande (droite ou zigzag), déplacement de zone, pipette, nuancier personnalisable
- Reconnaissance de texte (OCR)
- Enregistrement vidéo MP4 / WebM / GIF via FFmpeg, avec micro
- Historique des captures (sélection multiple par case, épinglage, conservation automatique), raccourcis globaux, icône dans la zone de notification
- Éditeur : duplication des formes (Ctrl+D), modification d'un texte, numéro ou tampon par double-clic, tampon personnalisé, quatre types de bulles avec pointe orientable, déplacement dans l'image zoomée au clic droit
- Aide intégrée (touche F1) dans les quatre langues
- Mise à jour intégrée : bandeau quand une nouvelle version est publiée, installation en un clic

## Windows

Télécharger l'installeur `ScreenCapture-Setup-x.y.z.exe` dans les [Releases](../../releases). Il propose d'installer FFmpeg s'il est absent.

**Version portable** : `ScreenCapture-x.y.z-portable.zip`, à décompresser et lancer sans installation (clé USB…). Le fichier `ScreenCapture.portable` à côté du programme indique ce mode : réglages et journal dans le dossier `data` voisin.

**Mises à jour** : l'application consulte la dernière version publiée sur GitHub (au démarrage puis une fois par jour, désactivable dans les Paramètres). Le fichier téléchargé est vérifié avec l'empreinte SHA-256 publiée par GitHub, puis installé (installeur silencieux, ou remplacement du programme pour la version portable).

### Compiler

Prérequis : [Rust](https://rustup.rs) et, pour l'installeur, [Inno Setup 6](https://jrsoftware.org/isinfo.php).

```powershell
cargo build --release          # target\release\ScreenCapture.exe
.\build-installer.ps1          # dist\ScreenCapture-Setup-x.y.z.exe et dist\ScreenCapture-x.y.z-portable.zip
```

## macOS

Télécharger `ScreenCapture-x.y.z-macOS.dmg` dans les [Releases](../../releases) (binaire universel Apple Silicon et Intel, macOS 11 ou plus récent), puis glisser ScreenCapture dans Applications.

- L'application n'est pas signée par un compte Apple Developer : au premier lancement, faire **clic droit → Ouvrir**, puis **Ouvrir**.
- Autoriser ScreenCapture dans **Réglages Système → Confidentialité et sécurité → Enregistrement de l'écran** (et **Micro** pour le son des vidéos), puis relancer l'application. Si les captures restent noires, supprimer l'entrée avec −, la rajouter, puis relancer : un bandeau rouge le rappelle dans l'application.
- Depuis la 0.1.15, l'application est signée avec un certificat stable : l'autorisation est conservée d'une mise à jour à l'autre.
- Capture avec défilement automatique : autorisation **Accessibilité** (sinon, faire défiler à la main).
- Vidéo : installer FFmpeg avec `brew install ffmpeg`.
- Raccourcis par défaut : ⌘⇧2 zone, ⌘⇧1 plein écran, ⌘⇧7 fenêtre, ⌘⇧8 dernière zone, ⌘⇧9 vidéo (⌘⇧3/4/5 restent ceux de macOS).
- Une capture de zone porte sur l'écran où se trouve le pointeur.

### Compiler

Prérequis : [Rust](https://rustup.rs) et les outils de ligne de commande Xcode (`xcode-select --install`).

```bash
./build-macos.sh            # binaire universel → dist/ScreenCapture-x.y.z-macOS.dmg
./build-macos.sh --native   # architecture du Mac uniquement, plus rapide
```

## Diagnostic

Un journal est écrit dans `%LOCALAPPDATA%\ScreenCapture\logs\screencapture.log` (Windows) ou `~/Library/Logs/ScreenCapture/screencapture.log` (macOS) ; le bouton **Paramètres → Diagnostic → Ouvrir le dossier** y mène. Il note les écrans, les captures, les changements de fenêtre et, si l'interface cesse de répondre, la dernière étape atteinte.

Si l'application ne répond plus, la relancer depuis le menu Démarrer : l'instance bloquée s'arrête (le journal note l'étape en cause) et la nouvelle prend le relais.

## Pilotage et validation (Windows)

L'instance en cours se pilote en ligne de commande : `ScreenCapture.exe --cmd <commande>`, avec `region`, `full`, `window`, `last`, `delayed`, `video`, `show`, `hide`, `cancel` ou `quit`.

Échap annule une capture en cours (compte à rebours d'une capture différée ou sélection), quelle que soit l'application au premier plan.

La campagne de validation des fenêtres (`examples/validate.rs`) pilote l'application et vérifie l'état réel des fenêtres (position, taille, réduite, masquée, barre des tâches, réactivité), sur un ou plusieurs écrans d'échelles différentes, avec un film à 60 images/s pour détecter les apparitions furtives :

```powershell
cargo build --release
cargo run --release --example validate -- tests\validation\mon-essai
```

Le rapport `RAPPORT.md` est écrit dans le dossier indiqué. Les réglages de l'utilisateur sont sauvegardés puis restaurés, et les captures de test vont dans un dossier temporaire.

## Compilation automatique

Le workflow GitHub Actions `Build` compile l'installeur Windows et le `.dmg` macOS : lancement manuel depuis l'onglet Actions, ou automatique à chaque tag `vX.Y.Z`, qui joint les fichiers à la Release.

## Licence

ScreenCapture est un logiciel libre, distribué sous licence **GNU GPL version 3 ou ultérieure** (fichier [`LICENSE`](LICENSE)) : chacun peut l'utiliser, l'étudier, le modifier et le redistribuer, à condition que toute version redistribuée reste sous la même licence, avec son code source. Il est fourni sans aucune garantie.

Les bibliothèques utilisées ont leurs propres licences (MIT, Apache-2.0, BSD, Zlib…), toutes compatibles avec la GPL-3.0 ; leurs mentions sont réunies dans [`THIRD-PARTY-LICENSES.txt`](THIRD-PARTY-LICENSES.txt), joint à l'installeur, au `.dmg` et consultable depuis la page « À propos ». Après un changement de dépendances, le régénérer avec `python tools/third_party_licenses.py`. FFmpeg, installé à part pour la vidéo, reste sous sa propre licence (GPL / LGPL).

## Aide et captures d'écran

Le texte de l'aide intégrée est dans `help/<langue>.md` et ses captures d'écran dans `help/img/<langue>/`. Elles sont produites par le scénario H1 de la campagne de validation, avec des images de démonstration, un fond plein écran générique et un dossier de captures neutre (rien de la machine qui les génère) :

```powershell
cargo build --release --examples
cargo run --release --example validate -- tests\validation\aide H1
```
