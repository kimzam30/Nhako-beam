# Builds the Windows desktop app installer (.exe) on your own Windows PC.
#
#   powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1
#
# The installer ends up in out\. Setup steps are in docs\building.md.
$ErrorActionPreference = 'Stop'

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$problems = @()
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { $problems += 'Rust is missing. Install it from https://rustup.rs' }
if (-not (Get-Command npm -ErrorAction SilentlyContinue)) { $problems += 'Node is missing. Install Node 20 or newer (https://nodejs.org)' }
if ($problems.Count -gt 0) {
    Write-Host "Can't build yet:"
    $problems | ForEach-Object { Write-Host "  - $_" }
    exit 1
}

Set-Location (Join-Path $Root 'desktop')
Write-Host "Installing the UI's packages..."
npm ci
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
# The app embeds the built UI, so it's built before any Rust step.
npm run build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host 'Building the installer...'
npx tauri build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$out = Join-Path $Root 'out'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$exe = Get-ChildItem (Join-Path $Root 'target\release\bundle\nsis\*.exe') | Select-Object -First 1
Copy-Item $exe.FullName $out
Write-Host ''
Write-Host "Done: out\$($exe.Name)"
Write-Host "It isn't signed yet: Windows SmartScreen asks first (More info, then Run anyway)."
