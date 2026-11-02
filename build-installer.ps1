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
