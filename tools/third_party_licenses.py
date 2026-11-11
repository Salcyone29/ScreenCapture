"""Génère THIRD-PARTY-LICENSES.txt : licences des bibliothèques compilées dans ScreenCapture.

Usage : python tools/third_party_licenses.py  (depuis la racine du projet ; nécessite cargo).
Les bibliothèques des deux plateformes (Windows et macOS) sont recensées ; le texte de licence
fourni par chaque bibliothèque est recopié, les textes identiques sont regroupés.
"""
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TARGETS = ["x86_64-pc-windows-msvc", "aarch64-apple-darwin"]
LICENSE_FILE = re.compile(r"^(LICEN[CS]E|COPYING|NOTICE|UNLICENSE|COPYRIGHT)([-_.].*)?$", re.I)
# Licences des polices intégrées (epaint_default_fonts : OFL.txt, UFL.txt, Hack-Regular.txt…).
FONT_LICENSE = re.compile(r"^(OFL|UFL|.*licen[cs]e.*|Hack-Regular)\.txt$", re.I)
SPDX_DIR = os.path.join(ROOT, "tools", "spdx")

packages = {}
for target in TARGETS:
    meta = json.loads(subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--filter-platform", target],
        cwd=ROOT, capture_output=True, check=True).stdout)
    used = {n["id"] for n in meta["resolve"]["nodes"]}
    for p in meta["packages"]:
        if p["id"] in used and p["source"]:  # dépendances seulement (pas ScreenCapture lui-même)
            packages[(p["name"], p["version"])] = p


def texts(p):
    """Textes de licence fournis avec la bibliothèque (fichiers LICENSE*, COPYING*, NOTICE*…)."""
    d = os.path.dirname(p["manifest_path"])
    files = []
    if p.get("license_file"):
        files.append(os.path.join(d, p["license_file"]))
    for root, dirs, names in os.walk(d):
        dirs[:] = [x for x in dirs if x not in ("target", ".git", "tests", "benches", "examples")]
        depth = 0 if os.path.samefile(root, d) else os.path.relpath(root, d).count(os.sep) + 1
        if depth > 1:
            continue
        files += [os.path.join(root, n) for n in sorted(names) if LICENSE_FILE.match(n) or (depth == 1 and FONT_LICENSE.match(n))]
    out = []
    for f in dict.fromkeys(os.path.normpath(x) for x in files):
        try:
            t = open(f, encoding="utf-8", errors="replace").read().strip()
        except OSError:
            continue
        if t:
            out.append((os.path.relpath(f, d).replace(os.sep, "/"), t))
    return out


groups = {}  # texte → liste de bibliothèques
missing = []
for (name, version), p in sorted(packages.items()):
    found = texts(p)
    if not found:
        authors = ", ".join(p.get("authors") or []) or "les auteurs de " + name
        missing.append((f"{name} {version} ({p['license']}) — Copyright (c) {authors}", p["license"] or ""))
    for fname, t in found:
        key = re.sub(r"\s+", " ", t)
        groups.setdefault(key, [t, []])[1].append(f"{name} {version} — {p['license']} — {fname}")

lines = []
w = lines.append
w("ScreenCapture — licences des composants tiers")
w("=" * 46)
w("")
w("ScreenCapture est distribué sous licence GNU GPL version 3 ou ultérieure (fichier LICENSE).")
w("Il est compilé avec les bibliothèques libres ci-dessous, chacune sous sa propre licence,")
w("toutes compatibles avec la GPL-3.0. Leurs mentions de copyright et textes de licence suivent.")
w("")
w("FFmpeg (enregistrement vidéo) n'est pas inclus dans ScreenCapture : c'est un programme séparé,")
w("installé à la demande de l'utilisateur (https://ffmpeg.org, versions Windows de gyan.dev),")
w("distribué sous licence GNU GPL / LGPL par ses auteurs.")
w("")
w("Les polices Segoe UI et Arial ne sont pas incluses : elles sont lues sur l'ordinateur.")
w("")
w("Bibliothèques (" + str(len(packages)) + ") :")
for (name, version), p in sorted(packages.items()):
    w(f"  - {name} {version} : {p['license'] or p.get('license_file')}")
w("")
w("Icônes : police Material Symbols (Google), licence Apache-2.0, intégrée via egui_material_icons.")
if missing:
    w("")
    w("Bibliothèques sans fichier de licence séparé : licence indiquée dans leurs métadonnées,")
    w("dont le texte standard figure à la fin de ce fichier.")
    for m, _ in missing:
        w("  - " + m)
for t, users in groups.values():
    w("")
    w("")
    w("-" * 78)
    for u in users:
        w(u)
    w("-" * 78)
    w("")
    w(t)

# Textes standard des licences des bibliothèques sans fichier, et de la police Material Symbols.
ids = {"Apache-2.0"}
for _, lic in missing:
    ids.update(re.findall(r"[A-Za-z0-9.+-]+", lic.replace(" OR ", " ").replace(" AND ", " ")))
for spdx in sorted(ids):
    f = os.path.join(SPDX_DIR, spdx + ".txt")
    if not os.path.isfile(f):
        continue
    w("")
    w("")
    w("-" * 78)
    w(f"Texte standard de la licence {spdx}")
    w("-" * 78)
    w("")
    w(open(f, encoding="utf-8").read().strip())
w("")

out = os.path.join(ROOT, "THIRD-PARTY-LICENSES.txt")
with open(out, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines))
print(f"{len(packages)} bibliothèques, {len(groups)} textes, {len(missing)} sans fichier → {out}", file=sys.stderr)
