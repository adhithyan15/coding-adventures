# Assemble the runnable task-app web project into dist/react (Windows / PowerShell).
$ErrorActionPreference = "Stop"

$Here = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path) # task-app dir
$Rust = Resolve-Path (Join-Path $Here "../../../packages/rust")
$Wasm = Join-Path $Rust "task-wasm"
$Out = Join-Path $Here "dist"

Write-Host "[1/3] Building the engine to wasm..."
& (Join-Path $Wasm "build-wasm.ps1")

Write-Host "[2/3] Emitting the React project..."
Push-Location $Rust
try {
    cargo run -q -p mosaic-compile -- pkg $Here --backend react --output $Out --emit-project
} finally {
    Pop-Location
}

Write-Host "[3/3] Overlaying host wiring + wasm runtime..."
$Rs = Join-Path $Out "react/src"
$Rp = Join-Path $Out "react/public"
New-Item -ItemType Directory -Force -Path $Rp | Out-Null
Copy-Item -Force (Join-Path $Here "host/web/main.tsx") (Join-Path $Rs "main.tsx")
Copy-Item -Force (Join-Path $Here "host/web/task-engine.d.ts") (Join-Path $Rs "task-engine.d.ts")
Copy-Item -Force (Join-Path $Wasm "js/task-engine.mjs") (Join-Path $Rs "task-engine.mjs")
Copy-Item -Force (Join-Path $Wasm "pkg/task_engine.wasm") (Join-Path $Rp "task_engine.wasm")

Write-Host ""
Write-Host "Ready. Run:  cd `"$Out/react`" ; npm install ; npm run dev   (http://localhost:5173)"
