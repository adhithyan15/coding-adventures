param(
    [switch]$Debug
)

$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$workspace = (Resolve-Path (Join-Path $scriptRoot "..")).Path
$outDir = Join-Path $scriptRoot "pkg"
$target = "wasm32-unknown-unknown"
$profile = if ($Debug) { "debug" } else { "release" }

New-Item -ItemType Directory -Force -Path $outDir | Out-Null

if (-not $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER) {
    $sysroot = (& rustc --print sysroot).Trim()
    $rustLld = Join-Path $sysroot "lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe"
    if (Test-Path -LiteralPath $rustLld) {
        $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = $rustLld
    }
}

Write-Host "Building engram-wasm for $target ($profile)..."
Push-Location $workspace
try {
    $cargoArgs = @("build", "-p", "engram-wasm", "--target", $target)
    if (-not $Debug) {
        $cargoArgs += "--release"
    }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

$source = Join-Path $workspace "target\$target\$profile\engram_wasm.wasm"
$dest = Join-Path $outDir "engram_engine.wasm"
Copy-Item -LiteralPath $source -Destination $dest -Force

$bytes = (Get-Item -LiteralPath $dest).Length
Write-Host "Wrote $dest ($bytes bytes)"
Write-Host "Smoke-test it with: node js/smoke.mjs"
