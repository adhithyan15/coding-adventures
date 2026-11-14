# tests/run-windows.ps1 -- the Windows BUILD recipe for perl-bridge.
#
# Same steps as the Unix BUILD: cargo test, cargo build, cargo clippy -D warnings,
# build the perl_bridge_smoke example, run tests\smoke.pl.  What differs is picking
# the interpreter.  Git Bash prepends its MSYS Perl to PATH, and MSYS Perl cannot
# supply headers or an import library for a native DLL: build.rs refuses it
# (osname must be MSWin32).  So query every perl.exe on PATH (or PERL_BIN), take
# the first native one, and use it for Cargo's build script, the staging and the
# test run alike.  Choosing it only in the build script would leave the loader on
# the wrong interpreter.
#
# This file is ASCII only on purpose: Windows PowerShell 5.1 reads a BOM-less file
# in the ANSI code page.
$ErrorActionPreference = 'Stop'

$candidates = if ($env:PERL_BIN) {
    @((Get-Command $env:PERL_BIN -CommandType Application).Source)
} else {
    @(Get-Command perl.exe -All -CommandType Application | Select-Object -ExpandProperty Source -Unique)
}
$nativePerl = $null
foreach ($candidate in $candidates) {
    $osname = & $candidate -MConfig -e 'print $Config{osname}'
    if ($LASTEXITCODE -eq 0 -and "$osname".Trim() -eq 'MSWin32') {
        $nativePerl = $candidate
        break
    }
}
if (-not $nativePerl) { throw 'Native Strawberry Perl is required on PATH (or in PERL_BIN)' }
$env:PERL_BIN = $nativePerl
# Strawberry Perl ships its own MinGW gcc in <root>\c\bin, and ExtUtils::CBuilder (used by
# build.rs) runs whatever "gcc" is first on PATH. In the full build, setup-ruby's MSYS2 and
# ghcup's MinGW put other gcc builds ahead of it, and a mismatched gcc fails with no
# diagnostic ("error building xs.dll file from 'src/perl_bridge_shim.c'"), which is how this
# recipe failed in the scheduled full build while passing on pull requests (no Ruby there).
# Put Strawberry's own toolchain first, next to its perl, exactly as
# code\scripts\check-perl-extension-windows.ps1 does for the Perl extensions.
$perlBin = Split-Path -Parent $nativePerl
$strawberryRoot = Split-Path -Parent (Split-Path -Parent $perlBin)
$toolDirs = @($perlBin, (Join-Path $strawberryRoot 'c\bin')) | Where-Object { Test-Path -LiteralPath $_ }
$env:PATH = "$($toolDirs -join ';');$env:PATH"
# Log which compiler the shim build will use, so a failure is diagnosable.
$gcc = Get-Command gcc.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if ($gcc) { Write-Host "gcc: $($gcc.Source)"; & $gcc.Source --version | Select-Object -First 1 } else { Write-Host 'gcc: not found on PATH' }

# The package root, wherever this was started from.
Push-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
try {
    & cargo test -p perl-bridge
    if ($LASTEXITCODE -ne 0) { throw 'cargo test failed' }

    & cargo build --release -p perl-bridge
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

    & cargo clippy -p perl-bridge --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'cargo clippy failed' }

    & cargo build --release -p perl-bridge --example perl_bridge_smoke --target-dir target
    if ($LASTEXITCODE -ne 0) { throw 'smoke extension build failed' }

    & $nativePerl tests\smoke.pl target\release\examples
    if ($LASTEXITCODE -ne 0) { throw 'perl-bridge smoke test failed' }
} finally {
    Pop-Location
}
