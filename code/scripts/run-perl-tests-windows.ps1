# run-perl-tests-windows.ps1 -- shared Windows recipe for Perl packages.
#
# Each Perl package's BUILD_windows calls this script from the package
# directory, e.g.
#
#     REM prereqs: ../grammar-tools ../algol-lexer
#     powershell -NoProfile -ExecutionPolicy Bypass -File ..\..\..\scripts\run-perl-tests-windows.ps1 -Deps grammar-tools,algol-lexer
#
# It is the Windows counterpart of the Unix recipe
#
#     cpanm --notest --quiet Test2::V0
#     PERL5LIB=<sibling libs> prove -l -v t/
#
# WHY A SCRIPT AND NOT AN INLINE ONE-LINER
# ----------------------------------------
# The Windows runner image has TWO Perls, and the wrong one is first on PATH:
#
#   * Git for Windows ships an MSYS Perl (C:\Program Files\Git\usr\bin).  It
#     treats PERL5LIB as a ':' separated POSIX list, does not ship Test2::V0
#     and has no cpanm.  It is what a bare `perl` resolves to.
#   * Strawberry Perl (C:\Strawberry) is a native Windows Perl with cpanm.
#
# Recipes therefore have to pick Strawberry explicitly (the existing
# x509-extension recipe does the same), put its bin directories first on PATH so
# `prove` and `cpanm` resolve to it too, and hand prove ABSOLUTE `-I` paths for
# the sibling packages.  `-I` is used instead of PERL5LIB on purpose: it needs no
# path-list separator at all.
#
# The build tool prints a package's output only when its recipe fails, so this
# script is chatty on purpose and exits with prove's own exit code.
#
# Parameters
#   -Deps   comma separated names of sibling packages under code\packages\perl
#           whose lib\ directory must be on @INC (the same set the Unix BUILD
#           lists as ../<name>/lib).

param(
    [string]$Deps = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Run a native command with output going to the console and return its exit
# code.  PowerShell 5.1 turns native stderr text into errors under 'Stop', so
# relax that while the command runs.
function Invoke-Native([scriptblock]$Command) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $global:LASTEXITCODE = 0
    try { & $Command | Out-Host } finally { $ErrorActionPreference = $saved }
    return [int]$global:LASTEXITCODE
}

try {
    # The build tool runs every recipe line in the package directory.
    $packageDir = (Get-Location).ProviderPath
    $perlRoot = Split-Path -Parent $packageDir
    Write-Host "==> package dir : $packageDir"
    if (-not (Test-Path -LiteralPath (Join-Path $packageDir 't'))) {
        throw "No t\ directory in $packageDir -- is the recipe running in the package directory?"
    }

    # ---- 1. Choose the Perl ---------------------------------------------------
    $strawberryBins = @(
        'C:\Strawberry\perl\site\bin',
        'C:\Strawberry\perl\bin',
        'C:\Strawberry\c\bin'
    ) | Where-Object { Test-Path -LiteralPath $_ }
    $perl = 'perl'
    if (Test-Path -LiteralPath 'C:\Strawberry\perl\bin\perl.exe') {
        $perl = 'C:\Strawberry\perl\bin\perl.exe'
        # Process-local: this script is its own powershell.exe, so the PATH edit
        # cannot leak into the build tool.  cpanm.bat and prove.bat look up
        # `perl` on PATH, so Strawberry has to come first for them too.
        $env:PATH = (($strawberryBins -join ';') + ';' + $env:PATH)
    } else {
        Write-Host '==> Strawberry Perl not found; falling back to the perl on PATH'
    }
    # A stray POSIX-style PERL5LIB (MSYS ':' separators) would confuse Strawberry.
    foreach ($v in 'PERL5LIB', 'PERL5OPT', 'PERLLIB') { Remove-Item "Env:$v" -ErrorAction SilentlyContinue }

    Write-Host "==> perl        : $perl"
    $rc = Invoke-Native { & $perl -e 'print qq{perl $^V at $^X\n}' }
    if ($rc -ne 0) { throw "perl could not be started (exit code $rc)" }

    # ---- 2. Make sure the test framework is available -------------------------
    # Same intent as the Unix `cpanm --notest --quiet Test2::V0` line, but only
    # install when it is missing: a bundled Test2::V0 makes this a no-op.
    $rc = Invoke-Native { & $perl -MTest2::V0 -e 'print qq{Test2::V0 $Test2::V0::VERSION is available\n}' }
    if ($rc -ne 0) {
        Write-Host '==> Test2::V0 missing; installing with cpanm'
        $rc = Invoke-Native { & cpanm --notest --quiet Test2::V0 }
        if ($rc -ne 0) { throw "cpanm could not install Test2::V0 (exit code $rc)" }
    }

    # ---- 3. Sibling library paths ---------------------------------------------
    $includes = @()
    foreach ($name in ($Deps -split ',' | Where-Object { $_.Trim() -ne '' })) {
        $name = $name.Trim()
        # A bare package directory name only: no separators, no '..'.
        if ($name -notmatch '^[A-Za-z0-9._-]+$' -or $name -eq '.' -or $name -eq '..') { throw "Invalid -Deps entry: '$name'" }
        $lib = Join-Path (Join-Path $perlRoot $name) 'lib'
        if (-not (Test-Path -LiteralPath $lib)) { throw "Missing sibling library directory: $lib" }
        $includes += "-I$lib"
    }
    Write-Host "==> sibling libs: $($includes.Count)"
    foreach ($i in $includes) { Write-Host "    $i" }

    # ---- 4. Same prove invocation as the Unix BUILD ---------------------------
    $proveArgs = @('-S', 'prove') + $includes + @('-l', '-v', 't')
    $rc = Invoke-Native { & $perl @proveArgs }
    if ($rc -ne 0) {
        Write-Host "==> prove FAILED with exit code $rc"
        Write-Host '==> @INC the tests ran with:'
        Invoke-Native { & $perl @includes -Ilib -e 'print qq{  $_\n} for @INC' } | Out-Null
        exit $rc
    }
    Write-Host '==> prove passed'
    exit 0
}
catch {
    Write-Host "==> ERROR: $($_.Exception.Message)"
    Write-Host $_.ScriptStackTrace
    exit 1
}
