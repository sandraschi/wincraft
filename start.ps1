param([switch]$NoPause)
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
$exe = Join-Path $PSScriptRoot "target\release\wincraft-app.exe"
$cargo = "C:\Users\sandr\.cargo\bin\cargo.exe"

function Fail($msg) {
    Write-Host $msg -ForegroundColor Red
    if (-not $NoPause) { Read-Host "Press Enter to close" | Out-Null }
    exit 1
}

# The release exe is file-locked while it runs, so `cargo run` fails with
# "Access is denied (os error 5)" when an instance is already open.
$running = Get-Process -Name "wincraft-app" -ErrorAction SilentlyContinue
if ($running) {
    Write-Host "WinCraft is already running (PID $($running.Id)). Reusing the open window - no rebuild needed."
    if (-not $NoPause) { Start-Sleep -Seconds 4 }
    exit 0
}

$needBuild = $true
if (Test-Path $exe) {
    $exeTime = (Get-Item $exe).LastWriteTime
    $newerSrc = Get-ChildItem -Path (Join-Path $PSScriptRoot "crates") -Recurse -Filter "*.rs" -ErrorAction SilentlyContinue | Where-Object { $_.LastWriteTime -gt $exeTime }
    $needBuild = ($null -ne $newerSrc)
} else {
    Write-Host "No release binary found - building first (one-time, takes a few minutes)."
}

if ($needBuild) {
    Write-Host "Sources changed since last build - rebuilding release binary..."
    & $cargo build -p wincraft-app --release
    if ($LASTEXITCODE -ne 0) { Fail "Release build failed with exit code $LASTEXITCODE." }
}

if (-not (Test-Path $exe)) { Fail "Build reported success but $exe is missing." }
Write-Host "Starting WinCraft..."
& $exe
