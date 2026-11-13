# build-conduit-capi-windows-gnu.ps1 -- build conduit-capi for Go's cgo on Windows.
#
# Why this script exists
# ----------------------
# Go's cgo on Windows drives a C compiler, and the only one Go supports there is
# MinGW-w64 gcc (cl.exe cannot be used).  A gcc/ld link needs a library that
# speaks the GNU ABI, so the default MSVC Rust output (conduit_capi.lib) is the
# wrong file.  This script therefore:
#
#   1. adds the Rust target x86_64-pc-windows-gnu (the `rust-std` component is
#      all that is needed; no extra linker),
#   2. builds ONLY the staticlib for that target:
#          cargo rustc --release --target x86_64-pc-windows-gnu --lib --crate-type staticlib
#      `cargo build` would also try to link the crate's `cdylib` output, which
#      needs a `x86_64-w64-mingw32-gcc` linker for rustc; asking for the
#      staticlib alone produces target\x86_64-pc-windows-gnu\release\libconduit_capi.a
#      (an archive: nothing is linked, so no linker is involved),
#   3. finds a MinGW-w64 gcc and puts its directory FIRST on PATH, and sets
#      CC=gcc, so `go test` links with it.
#
# The caller must run this script with `&` (same process), NOT `powershell -File`,
# so the PATH/CC edits survive into the caller's `go test`.  The caller is itself
# its own powershell.exe (the BUILD line), so nothing leaks past the recipe.
#
# Order matters: cargo runs BEFORE the PATH edit, so host build scripts and
# proc-macros are still built by the MSVC toolchain with MSVC's linker; the
# MinGW bin directory is only exposed to Go afterwards.
#
# The Go package's `#cgo windows LDFLAGS` line names the archive by full path and
# lists the system import libraries Rust std needs; keep the two in sync.
#
# The build tool prints a package's output only when its recipe fails, so this
# script is chatty on purpose and throws on real failure.

param(
    # This file lives in <repo>\code\scripts, so the repo root is two levels up.
    [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Run a native command with output going to the console and return its exit code.
# Windows PowerShell 5.1 turns native stderr text (cargo and rustup progress
# output) into errors under 'Stop', so relax that while the command runs.
function Invoke-Native([scriptblock]$Command) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $global:LASTEXITCODE = 0
    try { & $Command | Out-Host } finally { $ErrorActionPreference = $saved }
    return [int]$global:LASTEXITCODE
}

# Same, but capture stdout (stderr discarded) and return it as one trimmed string.
function Get-NativeOutput([scriptblock]$Command) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { return ((& $Command 2>$null) -join "`n").Trim() } finally { $ErrorActionPreference = $saved }
}

$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
$rustWorkspace = Join-Path $RepoRoot 'code\packages\rust'
$capiDir = Join-Path $rustWorkspace 'conduit-capi'
if (-not (Test-Path -LiteralPath $capiDir)) { throw "Missing crate directory: $capiDir" }
foreach ($tool in 'cargo', 'rustup', 'go') {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "$tool was not found on PATH." }
}

# ---- 1. Rust: GNU-ABI staticlib ------------------------------------------------
$target = 'x86_64-pc-windows-gnu'
Write-Host "==> rustup target add $target"
$rc = Invoke-Native { & rustup target add $target }
if ($rc -ne 0) { throw "rustup target add $target failed with exit code $rc" }

Write-Host "==> cargo rustc --release --target $target --lib --crate-type staticlib (in $capiDir)"
Push-Location -LiteralPath $capiDir
try { $rc = Invoke-Native { & cargo rustc --release --target $target --lib --crate-type staticlib } }
finally { Pop-Location }
if ($rc -ne 0) { throw "cargo rustc failed with exit code $rc" }

# The Go package hard-codes <workspace>\target\<triple>\release; a CARGO_TARGET_DIR
# override would put the archive somewhere else and the link would fail late and
# confusingly, so check here.
$archive = Join-Path $rustWorkspace "target\$target\release\libconduit_capi.a"
if (-not (Test-Path -LiteralPath $archive)) {
    $dir = Split-Path -Parent $archive
    Write-Host "Contents of ${dir}:"
    if (Test-Path -LiteralPath $dir) { Get-ChildItem -LiteralPath $dir | ForEach-Object { Write-Host "  $($_.Name)" } }
    else { Write-Host '  (directory does not exist)' }
    throw "libconduit_capi.a not found at $archive (is CARGO_TARGET_DIR set?)"
}
Write-Host ('==> staticlib   : {0} ({1:N0} bytes)' -f $archive, (Get-Item -LiteralPath $archive).Length)

# ---- 2. MinGW-w64 gcc for cgo ----------------------------------------------------
# Rust's x86_64-pc-windows-gnu std is built for the msvcrt-based MinGW-w64
# runtime, so prefer the runner image's own MinGW (C:\mingw64, x86_64-posix-seh)
# over the other gcc builds that can sit on PATH (Strawberry Perl's, Ruby's UCRT
# MSYS2).  Ruby's ucrt64 gcc is deliberately NOT a candidate.
$candidates = @(
    'C:\mingw64\bin',
    'C:\ProgramData\mingw64\mingw64\bin',
    'C:\msys64\mingw64\bin',
    'C:\Strawberry\c\bin'
)
$gccDir = $null
foreach ($dir in $candidates) {
    if (Test-Path -LiteralPath (Join-Path $dir 'gcc.exe')) { $gccDir = $dir; break }
}
if (-not $gccDir) {
    $onPath = Get-Command gcc.exe -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($onPath) { $gccDir = Split-Path -Parent $onPath.Source }
}
if (-not $gccDir) { throw 'No MinGW-w64 gcc.exe found (looked in the standard runner locations and on PATH).' }

# Everything from here is process-local: this runs inside the recipe's own powershell.exe.
$env:PATH = "$gccDir;$env:PATH"
$env:CC = 'gcc'
$env:CGO_ENABLED = '1'
Write-Host "==> MinGW bin   : $gccDir (prepended to PATH, CC=gcc)"

$machine = Get-NativeOutput { & gcc -dumpmachine }
Write-Host "==> gcc         : $((Get-Command gcc.exe).Source)"
Write-Host "==> gcc machine : $machine"
$gccVersion = (Get-NativeOutput { & gcc --version }) -split "`n" | Select-Object -First 1
Write-Host "==> gcc version : $gccVersion"
if ($machine -notmatch '^x86_64-.*mingw') {
    throw "gcc reports target '$machine', not an x86_64 MinGW-w64 toolchain."
}

$goarch = Get-NativeOutput { & go env GOARCH }
$goos = Get-NativeOutput { & go env GOOS }
Write-Host "==> go          : $(Get-NativeOutput { & go version }) (GOOS=$goos GOARCH=$goarch)"
if ($goos -ne 'windows' -or $goarch -ne 'amd64') {
    throw "Go targets $goos/$goarch; the x86_64-pc-windows-gnu staticlib needs windows/amd64."
}
