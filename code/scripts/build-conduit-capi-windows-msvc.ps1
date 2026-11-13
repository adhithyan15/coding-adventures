# build-conduit-capi-windows-msvc.ps1 -- build conduit-capi for a cl.exe (MSVC) consumer.
#
# Why this script exists
# ----------------------
# The C++ Conduit wrapper (cpp/conduit) and its demo (cpp/conduit-hello) are linked
# against the Rust C ABI crate rust/conduit-capi.  On Unix their tools/*.sh scripts
# build the staticlib, ask rustc which system libraries it needs, and hand both to
# g++/clang++.  This is the Windows/MSVC counterpart, shared by both packages so the
# two recipes cannot drift apart:
#
#   1. Make `link` mean MSVC's linker.  The Windows runner has Git for Windows'
#      usr\bin on PATH ahead of Visual Studio, and the bare name `link` then runs
#      GNU coreutils' link.exe ("link: unknown option"), both for rustc's own link
#      steps and for cl.exe.  PATH is edited for this process only (the recipe is
#      its own powershell.exe), and the script asserts that `link` really is MSVC's.
#      See lessons.d/on-the-windows-runner-a-bare-link-is-git-s-gnu-coreutils.md.
#   2. Assert the toolchain: cl, link, cargo and rustc are present and rustc's host
#      is x86_64-pc-windows-msvc.  The default Rust toolchain on the runner is the
#      MSVC one, so the staticlib is conduit_capi.lib (MSVC ABI, dynamic CRT /MD).
#      That matches cl.exe's /MD, so NO GNU target is involved here.  (go/conduit
#      is different: cgo only drives MinGW gcc, so it uses
#      build-conduit-capi-windows-gnu.ps1.)
#   3. Build ONLY the staticlib, like the Unix scripts do:
#          cargo rustc --release --crate-type staticlib -- --print native-static-libs
#      Asking for the staticlib alone skips linking the crate's cdylib.
#   4. Work out the system import libraries to link.  A Rust staticlib does not
#      carry them.  Two sources are merged: what rustc reports (parsed from the
#      "native-static-libs:" note, exactly as the Unix scripts do) and a fixed floor
#      for Rust 1.100's std, so a note that cargo did not replay (cached build)
#      cannot drop one:
#          kernel32 advapi32 ntdll userenv bcrypt ws2_32 synchronization
#      synchronization.lib holds WaitOnAddress/WakeByAddress*, which std's parking
#      primitives call; leaving it out gives LNK2019/LNK1120 on those symbols.
#
# The caller runs this with `&` (same process) so the PATH edit also covers its own
# cl.exe invocations.  It returns one object:
#     StaticLib    absolute path of conduit_capi.lib
#     CapiInclude  absolute path of rust\conduit-capi\include (conduit_capi.h)
#     SystemLibs   string[] of import libraries, e.g. 'ws2_32.lib'
#
# The build tool prints a package's output only when its recipe fails, so this script
# is chatty on purpose and throws on real failure.  (Keep the text ASCII-only:
# Windows PowerShell 5.1 reads a BOM-less file as the ANSI code page.)

param(
    # This file lives in <repo>\code\scripts, so the repo root is two levels up.
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Run a native command, echo its output (stdout and stderr) to the console as it
# arrives, and return @{ ExitCode; Lines }.  Windows PowerShell 5.1 turns native
# stderr text (cargo's progress output) into errors under 'Stop', so relax that
# while the command runs.
function Invoke-NativeCapture([scriptblock]$Command) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $global:LASTEXITCODE = 0
    $lines = New-Object System.Collections.Generic.List[string]
    try {
        & $Command 2>&1 | ForEach-Object {
            $text = "$_"
            Write-Host $text
            $lines.Add($text)
        }
    } finally { $ErrorActionPreference = $saved }
    return [pscustomobject]@{ ExitCode = [int]$global:LASTEXITCODE; Lines = $lines.ToArray() }
}

$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
$cargoWorkspace = Join-Path $RepoRoot 'code\packages\rust'
$capiDir = Join-Path $cargoWorkspace 'conduit-capi'
if (-not (Test-Path -LiteralPath $capiDir)) { throw "conduit-capi crate not found at $capiDir" }

# ---- 1. Make `link` mean MSVC's linker ---------------------------------------------
$kept = @()
foreach ($entry in ($env:PATH -split ';')) {
    if ($entry -eq '') { continue }
    if ($entry -match '\\usr\\bin\\?$' -or $entry -match '\\Git\\(mingw64|usr)\\bin\\?$') {
        Write-Host "==> PATH: dropping $entry"
        continue
    }
    $kept += $entry
}
$env:PATH = ($kept -join ';')

# ---- 2. Assert the toolchain ------------------------------------------------------
foreach ($tool in 'cl', 'link', 'cargo', 'rustc') {
    $found = Get-Command $tool -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $found) {
        throw "$tool was not found on PATH (the MSVC environment or the Rust toolchain was not set up for this job)."
    }
    Write-Host ("==> {0,-6}: {1}" -f $tool, $found.Source)
    if ($tool -in 'cl', 'link' -and $found.Source -notmatch 'Microsoft Visual Studio|MSVC|Windows Kits') {
        throw "$tool resolved to $($found.Source), which is not the MSVC one."
    }
}
$rustcInfo = Invoke-NativeCapture { & rustc -vV }
if ($rustcInfo.ExitCode -ne 0) { throw "rustc -vV failed with exit code $($rustcInfo.ExitCode)" }
$hostLine = $rustcInfo.Lines | Where-Object { $_ -match '^host:' } | Select-Object -First 1
if (-not $hostLine -or $hostLine -notmatch 'x86_64-pc-windows-msvc') {
    throw "rustc host is '$hostLine'; the MSVC recipe needs x86_64-pc-windows-msvc (a GNU host would produce libconduit_capi.a, not conduit_capi.lib)."
}

# ---- 3. Build the staticlib and capture the native-static-libs note ---------------
# CARGO_TERM_COLOR=never: CI sets it to 'always' and rustc then colorizes the
# "native-static-libs:" note, so an ANSI reset would stick to the last token.
$env:CARGO_TERM_COLOR = 'never'
Write-Host '==> cargo rustc --release --crate-type staticlib (conduit-capi)'
Push-Location -LiteralPath $capiDir
try {
    $build = Invoke-NativeCapture { & cargo rustc --release --crate-type staticlib -- --print native-static-libs }
} finally { Pop-Location }
if ($build.ExitCode -ne 0) { throw "cargo rustc failed with exit code $($build.ExitCode)" }

# ---- 4. System libraries: rustc's note merged with the Rust 1.100 floor ------------
$floor = @('kernel32.lib', 'advapi32.lib', 'ntdll.lib', 'userenv.lib', 'bcrypt.lib', 'ws2_32.lib', 'synchronization.lib')
$reported = @()
$noteLine = $build.Lines | Where-Object { $_ -match 'native-static-libs:' } | Select-Object -Last 1
if ($noteLine) {
    $tail = ($noteLine -replace '^.*native-static-libs:\s*', '') -replace '\x1b\[[0-9;]*m', ''
    foreach ($token in ($tail -split '\s+')) {
        $name = $null
        if ($token -match '^[A-Za-z0-9_.\-]+\.lib$') { $name = $token }
        elseif ($token -match '^-l([A-Za-z0-9_.\-]+)$') { $name = "$($Matches[1]).lib" }
        if (-not $name) { continue }
        # windows.0.52.0.lib is the import library the old windows-targets crate ships
        # in its own cargo build directory and finds through a link-search path that a
        # staticlib consumer does not have.  It is not an SDK library, so naming it on
        # the link line fails with LNK1181; the SDK libraries it fronts are in the
        # floor above.
        if ($name -match '^windows\.[0-9.]+\.lib$') {
            Write-Host "==> not linking $name (cargo-private import library, not an SDK library)"
            continue
        }
        $reported += $name
    }
    Write-Host "==> native-static-libs reported by rustc: $($reported -join ' ')"
} else {
    Write-Host '==> rustc printed no native-static-libs note (cached build?); using the built-in list only'
}
$systemLibs = @()
foreach ($lib in ($floor + $reported)) {
    if ($systemLibs -notcontains $lib.ToLowerInvariant()) { $systemLibs += $lib.ToLowerInvariant() }
}
Write-Host "==> system libraries to link: $($systemLibs -join ' ')"

# ---- 5. Locate the artifact -------------------------------------------------------
# CARGO_TARGET_DIR (if set) is relative to cargo's working directory.
$targetRoot = Join-Path $cargoWorkspace 'target'
if ($env:CARGO_TARGET_DIR) {
    if ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) { $targetRoot = $env:CARGO_TARGET_DIR }
    else { $targetRoot = Join-Path $capiDir $env:CARGO_TARGET_DIR }
}
$staticLib = Join-Path $targetRoot 'release\conduit_capi.lib'
if (-not (Test-Path -LiteralPath $staticLib)) {
    Write-Host "Contents of $(Join-Path $targetRoot 'release'):"
    if (Test-Path -LiteralPath (Join-Path $targetRoot 'release')) {
        Get-ChildItem -LiteralPath (Join-Path $targetRoot 'release') | ForEach-Object { Write-Host "  $($_.Name)" }
    }
    throw "conduit_capi.lib not found at $staticLib"
}
Write-Host ("==> staticlib: {0} ({1:N0} bytes)" -f $staticLib, (Get-Item -LiteralPath $staticLib).Length)

return [pscustomobject]@{
    StaticLib   = $staticLib
    CapiInclude = (Join-Path $capiDir 'include')
    SystemLibs  = [string[]]$systemLibs
}
