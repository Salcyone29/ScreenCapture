# Compile ScreenCapture en release puis l'installeur Inno Setup (dist\ScreenCapture-Setup-<version>.exe).
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value

if (-not (Test-Path assets\ScreenCapture.ico)) {
    cargo run --release --example make_icon
    if ($LASTEXITCODE) { throw "Échec de la génération de l'icône" }
}
cargo build --release
if ($LASTEXITCODE) { throw "Échec de la compilation" }

$iscc = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw "Inno Setup 6 introuvable (winget install JRSoftware.InnoSetup)" }

& $iscc /Qp "/DAppVersion=$version" installer\ScreenCapture.iss
if ($LASTEXITCODE) { throw "Échec de la compilation de l'installeur" }
Get-Item "dist\ScreenCapture-Setup-$version.exe"

# Version portable : programme + marqueur ScreenCapture.portable (réglages dans .\data).
$stage = "dist\portable\ScreenCapture"
Remove-Item -Recurse -Force dist\portable -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item target\release\ScreenCapture.exe $stage
Copy-Item LICENSE "$stage\LICENSE.txt"
Copy-Item THIRD-PARTY-LICENSES.txt $stage
Set-Content -Path "$stage\ScreenCapture.portable" -Encoding utf8 -Value @(
    "Version portable de ScreenCapture : les réglages et le journal sont enregistrés dans le dossier « data » à côté du programme.",
    "Supprimer ce fichier pour utiliser les réglages de l'utilisateur Windows."
)
$zip = "dist\ScreenCapture-$version-portable.zip"
Remove-Item $zip -ErrorAction SilentlyContinue
Compress-Archive -Path $stage -DestinationPath $zip
Remove-Item -Recurse -Force dist\portable
Get-Item $zip
