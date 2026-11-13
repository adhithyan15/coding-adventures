# run.ps1 -- Windows counterpart of run.sh for the C++ conduit-hello demo.
#
# Unix recipe (tools/run.sh):
#     cargo rustc --release --crate-type staticlib -- --print native-static-libs   (conduit-capi)
#     g++/clang++ ... src/main.cpp   -> _build/conduit-hello
#     g++/clang++ ... tests/smoke.cpp -> _build/smoke ; run it
#
# Windows recipe: the same, with MSVC.  The shared helper
# code\scripts\build-conduit-capi-windows-msvc.ps1 fixes PATH so `link` is MSVC's,
# builds the MSVC-ABI staticlib conduit_capi.lib and reports the system libraries it
# needs.  cl.exe then compiles with /MD (dynamic CRT, as Rust's MSVC std is).  The
# smoke test launches the real demo app on an OS-assigned port and hits it over a
# loopback socket; tests\net_compat.h maps the BSD-socket calls onto Winsock.
#
# The build tool prints a package's output only when its recipe fails, so this script
# is chatty on purpose and exits with the failing tool's own exit code.
# (Keep the text ASCII-only: Windows PowerShell 5.1 reads a BOM-less file as ANSI.)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Run a native command with output going to the console and return its exit code.
# PowerShell 5.1 turns native stderr text into errors under 'Stop', so relax that
# while the command runs.
function Invoke-Native([scriptblock]$Command) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $global:LASTEXITCODE = 0
    try { & $Command | Out-Host } finally { $ErrorActionPreference = $saved }
    return [int]$global:LASTEXITCODE
}

# Windows crash codes are NTSTATUS values, unreadable in decimal: 0xC0000135 is a
# missing DLL, 0xC0000005 an access violation, 0xC00000FD a stack overflow.
function Format-ExitCode([int]$Code) {
    return ('{0} (0x{1:X8})' -f $Code, $Code)
}

try {
    # tools\ -> conduit-hello -> cpp -> programs -> code -> repo root.
    $packageDir = Split-Path -Parent $PSScriptRoot
    $repoRoot = $packageDir
    for ($i = 0; $i -lt 4; $i++) { $repoRoot = Split-Path -Parent $repoRoot }
    $helper = Join-Path $repoRoot 'code\scripts\build-conduit-capi-windows-msvc.ps1'
    if (-not (Test-Path -LiteralPath $helper)) { throw "Missing helper script: $helper" }
    $conduitInclude = Join-Path $repoRoot 'code\packages\cpp\conduit\include'
    if (-not (Test-Path -LiteralPath (Join-Path $conduitInclude 'conduit\conduit.hpp'))) {
        throw "cpp/conduit headers not found under $conduitInclude"
    }

    Write-Host "==> package dir : $packageDir"
    Write-Host "==> repo root   : $repoRoot"
    Set-Location -LiteralPath $packageDir

    # ---- 1. PATH fix + Rust staticlib + system libs ---------------------------------
    # Invoked with `&` so its PATH edit applies to this process.
    $capi = & $helper -RepoRoot $repoRoot | Select-Object -Last 1
    if (-not $capi -or -not $capi.StaticLib) { throw 'the conduit-capi helper returned no result' }

    # ---- 2. Compile the demo and the smoke test ------------------------------------
    # Output dir is _build\, NOT build\ (mirrors run.sh).
    New-Item -ItemType Directory -Force -Path '_build' | Out-Null
    $clFlags = @('/nologo', '/std:c++17', '/permissive-', '/EHsc', '/W4', '/MD',
                 '/Isrc', '/Itests', "/I$conduitInclude", "/I$($capi.CapiInclude)")

    Write-Host '==> Compiling demo binary'
    $rc = Invoke-Native {
        & cl @clFlags 'src\main.cpp' '/Fe:_build\conduit-hello.exe' '/Fo:_build\main.obj' /link $capi.StaticLib @($capi.SystemLibs)
    }
    if ($rc -ne 0) { Write-Host "==> cl FAILED for the demo with exit code $(Format-ExitCode $rc)"; exit $rc }

    Write-Host '==> Compiling smoke test'
    $rc = Invoke-Native {
        & cl @clFlags 'tests\smoke.cpp' '/Fe:_build\smoke.exe' '/Fo:_build\smoke.obj' /link $capi.StaticLib @($capi.SystemLibs)
    }
    if ($rc -ne 0) { Write-Host "==> cl FAILED for the smoke test with exit code $(Format-ExitCode $rc)"; exit $rc }

    # ---- 3. Run the smoke test ----------------------------------------------------
    Write-Host '==> Running smoke test'
    $rc = Invoke-Native { & '.\_build\smoke.exe' }
    if ($rc -ne 0) { Write-Host "==> smoke test FAILED with exit code $(Format-ExitCode $rc)"; exit $rc }
    Write-Host '==> smoke test passed'
    exit 0
}
catch {
    Write-Host "==> ERROR: $($_.Exception.Message)"
    Write-Host $_.ScriptStackTrace
    exit 1
}
