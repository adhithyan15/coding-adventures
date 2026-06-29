param(
    [string]$Output = "target/mosaic-engram-app",
    [switch]$Release
)

$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$packageRoot = (Resolve-Path (Join-Path $scriptRoot "..")).Path
$rustWorkspace = (Resolve-Path (Join-Path $packageRoot "..\..\..\packages\rust")).Path

if ([System.IO.Path]::IsPathRooted($Output)) {
    $outputRoot = [System.IO.Path]::GetFullPath($Output)
} else {
    $outputRoot = [System.IO.Path]::GetFullPath((Join-Path $packageRoot $Output))
}

New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

if (-not $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER) {
    $sysroot = (& rustc --print sysroot).Trim()
    $rustLld = Join-Path $sysroot "lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe"
    if (Test-Path -LiteralPath $rustLld) {
        $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = $rustLld
    }
}

$backends = @(
    "html",
    "webcomponent",
    "react",
    "electron",
    "swiftui",
    "qt",
    "xaml",
    "flutter",
    "compose"
)

foreach ($backend in $backends) {
    Write-Host "==> Emitting EngramApp $backend project"
    $cargoArgs = @("run")
    if ($Release) {
        $cargoArgs += "--release"
    }
    $cargoArgs += @(
        "-p",
        "mosaic-compile",
        "--",
        "pkg",
        $packageRoot,
        "--backend",
        $backend,
        "--output",
        $outputRoot,
        "--emit-project"
    )

    Push-Location $rustWorkspace
    try {
        & cargo @cargoArgs
        if ($LASTEXITCODE -ne 0) {
            throw "mosaic-compile failed for backend '$backend' with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
}

Write-Host "Engram Mosaic host shells written to $outputRoot"
