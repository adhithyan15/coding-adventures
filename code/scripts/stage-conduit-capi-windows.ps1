# stage-conduit-capi-windows.ps1 -- build conduit-capi and stage it for GHC.
#
# Why this script exists
# ----------------------
# Haskell (and any other language that links the Rust conduit-capi C ABI through
# a MinGW-style toolchain) needs two files from `cargo build -p conduit-capi
# --release` on Windows, and needs them WITHOUT their awkward neighbours.
#
# With the default MSVC Rust toolchain, target\release holds, for conduit_capi:
#
#   conduit_capi.dll         the runtime library (what the test exe loads)
#   conduit_capi.dll.lib     the MSVC IMPORT library for that DLL (small, ~KBs)
#   conduit_capi.lib         the STATIC library (crate-type includes staticlib)
#
# GHC 9.4.8 on Windows links with clang + lld (through the MinGW driver), and
# `-lconduit_capi` makes the linker look for libconduit_capi.dll.a first and
# then conduit_capi.lib in the lib dirs.  lld reads MSVC import libraries
# natively, so the import library only needs the MinGW spelling to be found.
# If target\release itself is a lib dir, the STATIC conduit_capi.lib can win,
# and it drags MSVC CRT symbols into a MinGW link, which fails.  So we never
# expose target\release.  Instead we build a tiny, fresh "stage" directory
# holding only the DLL and the import library renamed to the MinGW spelling
# (libconduit_capi.dll.a), which is probed before the .lib name.
#
# The caller passes the stage dir to GHC as --extra-lib-dirs and puts it FIRST
# on PATH so the finished test executable can find conduit_capi.dll at run time.
#
# The build tool shows a recipe's output only when it FAILS, so this script
# prints diagnostics on every run (all best-effort) and throws on real failure.

param(
    [Parameter(Mandatory = $true)]
    [string]$StageDir,
    # This file lives in <repo>\code\scripts, so the repo root is two levels up.
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Native exit codes are signed 32-bit here.  NTSTATUS failures look like noise
# in decimal (-1073741515) but are famous in hex (0xC0000135 = DLL not found).
function Format-ExitCode([int]$Code) {
    $text = '{0} (0x{1:X8})' -f $Code, $Code
    if ($Code -eq -1073741515) { $text += ' = STATUS_DLL_NOT_FOUND' }
    return $text
}

# Run a native command with output going straight to the console and return its
# exit code.  Windows PowerShell 5.1 turns native stderr text (cargo's progress
# output!) into errors under $ErrorActionPreference='Stop', so relax it here.
function Invoke-Native([scriptblock]$Command) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $global:LASTEXITCODE = 0
    try { & $Command | Out-Host } finally { $ErrorActionPreference = $saved }
    return [int]$global:LASTEXITCODE
}

# Diagnostics must never fail the build: run a block under a heading, swallow
# any error, and keep going.
function Invoke-Diag([string]$Title, [scriptblock]$Block) {
    Write-Host "---- $Title"
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $Block | Out-Host }
    catch { Write-Host "(diagnostic failed: $($_.Exception.Message))" }
    finally { $ErrorActionPreference = $saved }
}

function Show-Dir([string]$Dir, [string]$Filter) {
    Get-ChildItem -LiteralPath $Dir -Filter $Filter |
        ForEach-Object { '{0,12:N0}  {1}' -f $_.Length, $_.Name }
}

$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
$StageDir = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($StageDir)
if ($StageDir -eq [System.IO.Path]::GetPathRoot($StageDir)) {
    throw "Refusing to use a drive root as the stage directory: $StageDir"
}

# ---- 1. Build the Rust library --------------------------------------------
$cargoWorkspace = Join-Path (Join-Path $RepoRoot 'code') 'packages\rust'
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'cargo was not found on PATH; install a Rust toolchain first.'
}
Write-Host "==> cargo build -p conduit-capi --release (in $cargoWorkspace)"
Push-Location -LiteralPath $cargoWorkspace
try { $rc = Invoke-Native { & cargo build -p conduit-capi --release } }
finally { Pop-Location }
if ($rc -ne 0) { throw "cargo build failed with exit code $(Format-ExitCode $rc)" }

# ---- 2. Locate the artifacts ----------------------------------------------
# CARGO_TARGET_DIR (if set) is relative to cargo's working directory.
$targetRoot = Join-Path $cargoWorkspace 'target'
if ($env:CARGO_TARGET_DIR) {
    # Join-Path would mangle an absolute value ("a\C:\x"), so test first.
    if ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
        $targetRoot = $env:CARGO_TARGET_DIR
    } else {
        $targetRoot = Join-Path $cargoWorkspace $env:CARGO_TARGET_DIR
    }
}
$releaseDir = Join-Path $targetRoot 'release'

function Stop-WithListing([string]$Message) {
    Write-Host "Contents of ${releaseDir}:"
    if (Test-Path -LiteralPath $releaseDir) {
        Get-ChildItem -LiteralPath $releaseDir | ForEach-Object { Write-Host "  $($_.Name)" }
    } else {
        Write-Host '  (directory does not exist)'
    }
    throw $Message
}

$dll = Join-Path $releaseDir 'conduit_capi.dll'
if (-not (Test-Path -LiteralPath $dll)) { Stop-WithListing "conduit_capi.dll not found in $releaseDir" }

# MSVC toolchain: conduit_capi.dll.lib.  GNU toolchain already uses the MinGW
# name, so accept that too rather than failing on a perfectly good build.
$importLib = Join-Path $releaseDir 'conduit_capi.dll.lib'
if (-not (Test-Path -LiteralPath $importLib)) {
    $importLib = Join-Path $releaseDir 'libconduit_capi.dll.a'
}
if (-not (Test-Path -LiteralPath $importLib)) {
    Stop-WithListing "No import library (conduit_capi.dll.lib) found in $releaseDir"
}

# ---- 3. Stage ---------------------------------------------------------------
if (Test-Path -LiteralPath $StageDir) { Remove-Item -LiteralPath $StageDir -Recurse -Force }
New-Item -ItemType Directory -Path $StageDir -Force | Out-Null
Copy-Item -LiteralPath $dll -Destination (Join-Path $StageDir 'conduit_capi.dll')
Copy-Item -LiteralPath $importLib -Destination (Join-Path $StageDir 'libconduit_capi.dll.a')
Write-Host "==> Staged conduit-capi into $StageDir"

# ---- 4. Diagnostics (best effort) -------------------------------------------
Invoke-Diag "Stage dir: $StageDir" { Show-Dir $StageDir '*' }
Invoke-Diag "conduit_capi* in $releaseDir" { Show-Dir $releaseDir 'conduit_capi*' }
Invoke-Diag 'ghc --info (platform and toolchain)' {
    if (Get-Command ghc -ErrorAction SilentlyContinue) {
        & ghc --info | Select-String -Pattern '\("(Target platform|C compiler command|ld command)"' |
            ForEach-Object { $_.Line.Trim() }
    } else { 'ghc not found on PATH' }
}
Invoke-Diag 'where.exe conduit_capi.dll (current PATH)' { & where.exe conduit_capi.dll }
# Exports AND the runtime DLLs this library depends on: a missing dependency is
# what produces 0xC0000135 (STATUS_DLL_NOT_FOUND) when the test exe starts.
# llvm-objdump -p prints PE headers followed by the export and import tables, so
# filter to the interesting lines instead of truncating blindly.
Invoke-Diag 'DLL exports and dependencies' {
    if (Get-Command dumpbin.exe -ErrorAction SilentlyContinue) {
        & dumpbin.exe /exports $dll
        & dumpbin.exe /dependents $dll
    } elseif (Get-Command llvm-objdump.exe -ErrorAction SilentlyContinue) {
        & llvm-objdump.exe -p $dll | Select-String -Pattern 'DLL Name|conduit_' |
            ForEach-Object { $_.Line.Trim() } | Select-Object -First 60
    } else { 'neither dumpbin nor llvm-objdump is on PATH' }
}
# Prove the staged .a really is the small IMPORT library and not the static
# library: an import library lists a few __imp_/thunk symbols per export, while
# the static one would be full of MSVC CRT and Rust runtime symbols.
Invoke-Diag 'staged import library symbols' {
    if (Get-Command llvm-nm.exe -ErrorAction SilentlyContinue) {
        & llvm-nm.exe (Join-Path $StageDir 'libconduit_capi.dll.a') | Select-Object -First 20
    } else { 'llvm-nm not on PATH' }
}

Write-Host "STAGE_DIR=$StageDir"
