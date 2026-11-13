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
$env:PATH = "$(Split-Path -Parent $nativePerl);$env:PATH"

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
