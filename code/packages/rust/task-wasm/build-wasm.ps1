# Compile the task-wasm boundary to pkg/task_engine.wasm (Windows / PowerShell).
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Workspace = Split-Path -Parent $ScriptDir
$OutDir = Join-Path $ScriptDir "pkg"
$Target = "wasm32-unknown-unknown"

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Write-Host "Building task-wasm for $Target (release)..."
Push-Location $Workspace
try {
    cargo build -p task-wasm --target $Target --release
} finally {
    Pop-Location
}

$Src = Join-Path $Workspace "target/$Target/release/task_wasm.wasm"
$Dst = Join-Path $OutDir "task_engine.wasm"
Copy-Item -Force $Src $Dst
Write-Host "Wrote $Dst ($((Get-Item $Dst).Length) bytes)"
