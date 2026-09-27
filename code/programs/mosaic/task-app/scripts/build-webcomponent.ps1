$ErrorActionPreference = "Stop"

$Here = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Web = Join-Path $Here "host/web"
$Rust = Resolve-Path (Join-Path $Here "../../../packages/rust")
$Wasm = Join-Path $Rust "task-wasm"
$Emit = Join-Path $Web ".emit-webcomponent"

Write-Host "[1/3] Building the TaskApp engine to WebAssembly..."
& (Join-Path $Wasm "build-wasm.ps1")

Write-Host "[2/3] Emitting the real TaskApp Custom Element in both themes..."
if (Test-Path $Emit) { Remove-Item -Recurse -Force $Emit }
foreach ($Theme in @("light", "dark")) {
    Push-Location $Rust
    try {
        cargo run -q -p mosaic-compile -- pkg $Here --backend webcomponent --theme $Theme --output (Join-Path $Emit $Theme) --emit-project
    } finally {
        Pop-Location
    }
    $Generated = Join-Path $Web "webcomponent/generated/$Theme"
    New-Item -ItemType Directory -Force -Path $Generated | Out-Null
    Copy-Item -Force (Join-Path $Emit "$Theme/webcomponent/TaskApp.js") (Join-Path $Generated "TaskApp.js")
    Copy-Item -Force (Join-Path $Emit "$Theme/webcomponent/main.js") (Join-Path $Generated "main.js")
}
Remove-Item -Recurse -Force $Emit

Write-Host "[3/3] Staging the WASM engine for the parity host..."
$Public = Join-Path $Web "webcomponent/public"
New-Item -ItemType Directory -Force -Path $Public | Out-Null
Copy-Item -Force (Join-Path $Wasm "js/task-engine.mjs") (Join-Path $Web "src/task-engine.mjs")
Copy-Item -Force (Join-Path $Wasm "pkg/task_engine.wasm") (Join-Path $Public "task_engine.wasm")

Write-Host ""
Write-Host "Ready. Run: cd `"$Web`" ; npm install ; npm run build:webcomponent"
