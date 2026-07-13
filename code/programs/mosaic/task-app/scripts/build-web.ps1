# Build the task-app web host (Windows / PowerShell).
#   1. build the task-core engine to wasm,
#   2. emit the TaskApp React component into host/web/src,
#   3. copy the wasm runtime into the package.
# The host is a committed npm package; this only refreshes generated/copied artifacts.
$ErrorActionPreference = "Stop"

$Here = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path) # task-app dir
$Web = Join-Path $Here "host/web"
$Rust = Resolve-Path (Join-Path $Here "../../../packages/rust")
$Wasm = Join-Path $Rust "task-wasm"

Write-Host "[1/3] Building the engine to wasm..."
& (Join-Path $Wasm "build-wasm.ps1")

Write-Host "[2/3] Emitting the TaskApp component into the web host..."
# `mosaic-compile pkg` emits into <output>/react/, so emit to a scratch dir and
# copy just the component file into the host's src (main.tsx imports ./TaskApp).
New-Item -ItemType Directory -Force -Path (Join-Path $Web "src") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Web "public") | Out-Null
$Emit = Join-Path $Web ".emit"
if (Test-Path $Emit) { Remove-Item -Recurse -Force $Emit }
Push-Location $Rust
try {
    cargo run -q -p mosaic-compile -- pkg $Here --backend react --output $Emit
} finally {
    Pop-Location
}
Copy-Item -Force (Join-Path $Emit "react/TaskApp.tsx") (Join-Path $Web "src/TaskApp.tsx")
Remove-Item -Recurse -Force $Emit

Write-Host "[3/3] Copying the wasm runtime..."
Copy-Item -Force (Join-Path $Wasm "js/task-engine.mjs")   (Join-Path $Web "src/task-engine.mjs")
Copy-Item -Force (Join-Path $Wasm "pkg/task_engine.wasm") (Join-Path $Web "public/task_engine.wasm")

Write-Host ""
Write-Host "Ready. Run:  cd `"$Web`" ; npm install ; npm run dev   (http://localhost:5173)"
