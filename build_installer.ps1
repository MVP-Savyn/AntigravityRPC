$ErrorActionPreference = "Stop"
$ProjectDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RustDir = Join-Path $ProjectDir "rust"
$DistDir = Join-Path $ProjectDir "dist"

if (-not $env:RUSTUP_HOME -and (Test-Path "$env:USERPROFILE\scoop\persist\rustup-gnu\.rustup")) {
    $env:RUSTUP_HOME = "$env:USERPROFILE\scoop\persist\rustup-gnu\.rustup"
}
if (-not $env:CARGO_HOME -and (Test-Path "$env:USERPROFILE\scoop\persist\rustup-gnu\.cargo")) {
    $env:CARGO_HOME = "$env:USERPROFILE\scoop\persist\rustup-gnu\.cargo"
}
if ($env:CARGO_HOME -and (Test-Path "$env:CARGO_HOME\bin")) {
    $env:PATH = "$env:CARGO_HOME\bin;$env:PATH"
}
if (Test-Path "$env:USERPROFILE\scoop\apps\mingw\current\bin") {
    $env:PATH = "$env:USERPROFILE\scoop\apps\mingw\current\bin;$env:PATH"
}

Write-Host "==> 1. Compilando binario nativo en Rust..." -ForegroundColor Cyan
Push-Location $RustDir
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build --release falló con código $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

$RustExe = Join-Path $RustDir "target\release\antigravity-rich-presence.exe"
if (-not (Test-Path $RustExe)) {
    throw "No se generó $RustExe"
}

New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
$DistExe = Join-Path $DistDir "AntigravityRichPresence.exe"
Copy-Item -Force $RustExe $DistExe

$IsccCandidates = @(
    "$env:LOCALAPPDATA\Programs\Antigravity IDE\resources\app\node_modules\innosetup\bin\ISCC.exe",
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe"
)

$Iscc = $IsccCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $Iscc) {
    $cmd = Get-Command iscc -ErrorAction SilentlyContinue
    if ($cmd) { $Iscc = $cmd.Source }
}

if (-not $Iscc) {
    throw "No se encontró ISCC.exe (Inno Setup 6)."
}

Write-Host "==> 2. Compilando instalador con Inno Setup ($Iscc)..." -ForegroundColor Cyan
Push-Location $ProjectDir
try {
    & $Iscc "installer.iss"
    if ($LASTEXITCODE -ne 0) {
        throw "ISCC.exe falló con código $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "Compilación completada:" -ForegroundColor Green
Write-Host "  - Ejecutable Rust portable: $DistExe"
Write-Host "  - Instalador Windows:       $ProjectDir\installer_out\AntigravityRichPresenceSetup.exe"

$WebDownloadsDir = Join-Path $ProjectDir "web\downloads"
if (Test-Path (Join-Path $ProjectDir "web")) {
    New-Item -ItemType Directory -Force -Path $WebDownloadsDir | Out-Null
    $SetupExe = Join-Path $ProjectDir "installer_out\AntigravityRichPresenceSetup.exe"
    if (Test-Path $SetupExe) {
        Copy-Item -Force $SetupExe (Join-Path $WebDownloadsDir "AntigravityRichPresenceSetup.exe")
    }
    Write-Host "  - Instalador web actualizado en: $WebDownloadsDir"
}
