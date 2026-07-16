<#
    run.ps1 — build the example plugin DLL, then build & run the host test.

    The Windows counterpart to run.sh: it first compiles the plugin into a DLL
    with `cl /LD` (the plugin's entry is marked __declspec(dllexport) via the
    ABI header, so GetProcAddress can resolve it), then builds and runs the host
    test through platform-harness, linking the os-platform dynlib backend. If the
    plugin DLL cannot be built, it SKIPS gracefully (exit 0).
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$self = Split-Path -Parent $PSScriptRoot
Set-Location $self

$d = $self
while ($d -and -not (Test-Path (Join-Path $d 'code\packages\c\platform-harness'))) {
    $parent = Split-Path -Parent $d
    if ($parent -eq $d) { break }
    $d = $parent
}
$harness = Join-Path $d 'code\packages\c\platform-harness'
$iso = Join-Path $d 'code\packages\c\iso-harness'
$osp = Join-Path $d 'code\packages\c\os-platform'
if (-not (Test-Path (Join-Path $harness 'lib\platform-lib.ps1'))) {
    throw "platform-harness not found (searched upward from $self)"
}

. (Join-Path $harness 'lib\platform-lib.ps1')

$env:PLATFORM_INCLUDE = "$(Join-Path $self 'include') $(Join-Path $osp 'include') $(Join-Path $iso 'include')"

Write-Host "plugin-host (windows): C compilers: $((Platform-Compilers -Lang c) -join ' ')"

# ── step 1: build the plugin DLL (graceful skip if we cannot) ────────────────
New-Item -ItemType Directory -Force -Path '_build' | Out-Null
if (-not (Get-Command 'cl' -ErrorAction SilentlyContinue)) {
    Write-Host 'plugin-host: cl not found; skipping gracefully'
    exit 0
}
& cl /nologo /LD "/I$(Join-Path $self 'include')" 'plugins\example_plugin.c' `
    '/Fo_build\' '/Fe_build\osp_plugin.dll' 2>&1 | Write-Host
if ($LASTEXITCODE -ne 0) {
    Write-Host 'plugin-host: this toolchain cannot build a shared plugin; skipping gracefully'
    exit 0
}

# ── step 2: build & run the host test ────────────────────────────────────────
Platform-BuildAndRun -Lang c -Name 'plugin-host-tests' -Sources @(
    'tests\plugin_host_test.c',
    'src\host.c',
    (Join-Path $osp 'src\dynlib_windows.c')
)
