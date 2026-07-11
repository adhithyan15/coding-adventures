<#
.SYNOPSIS
    iso-lib.ps1 — the Windows/MSVC half of the pure-ISO C/C++ build harness.

.DESCRIPTION
    This is the PowerShell counterpart to lib/iso-lib.sh. On Windows the compiler
    of record is MSVC's `cl.exe` (and, when present, LLVM's `clang-cl.exe`). It
    compiles each translation unit under the strictest standards-conformance
    flags MSVC offers:

        C   :  /std:c17   /permissive- /W4 /WX
        C++ :  /std:c++17 /permissive- /W4 /WX /EHsc

    `/permissive-` is MSVC's equivalent of GCC/Clang's -pedantic-errors: it
    disables Microsoft language extensions and enforces standard conformance.
    `/WX` makes every warning fatal, mirroring -Werror.

    The design mirrors iso-lib.sh exactly: compile with EVERY compiler present,
    fail if none are found, honor an ISO_REQUIRE list of must-be-present
    compilers, and provide a negative-test helper that asserts non-ISO code is
    rejected.

    Public functions:
        Iso-Compilers      -Lang c|cpp                    → present MSVC-family compilers
        Iso-BuildAndRun    -Lang c|cpp -Name n -Sources … → compile+run with each; throws on failure
        Iso-ExpectCompileFail -Lang c|cpp -Source f       → assert every compiler rejects f

    Environment knobs (same names as iso-lib.sh):
        ISO_REQUIRE, ISO_INCLUDE, ISO_BUILD_DIR (default _build), ISO_CSTD, ISO_CXXSTD.

    `cl.exe` must already be on PATH — in CI the msvc-dev-cmd action puts it
    there; locally, run from a "Developer Command Prompt" / after vcvars64.bat.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Iso-BuildDir {
    if ($env:ISO_BUILD_DIR) { return $env:ISO_BUILD_DIR }
    return '_build'
}

function Iso-Cstd  { if ($env:ISO_CSTD)   { return $env:ISO_CSTD }   else { return 'c17' } }
function Iso-Cxxstd { if ($env:ISO_CXXSTD) { return $env:ISO_CXXSTD } else { return 'c++17' } }

# Strict flag list for MSVC-family compilers, as an array of tokens.
function Iso-StrictFlags {
    param([Parameter(Mandatory)][ValidateSet('c','cpp')][string]$Lang)
    $flags = @('/nologo', '/permissive-', '/W4', '/WX')
    if ($Lang -eq 'cpp') {
        $flags += @("/std:$(Iso-Cxxstd)", '/EHsc')
    } else {
        $flags += @("/std:$(Iso-Cstd)")
    }
    foreach ($dir in ($env:ISO_INCLUDE -split '\s+' | Where-Object { $_ })) {
        $flags += "/I$dir"
    }
    return $flags
}

# Return the MSVC-family compilers present on PATH, in a stable order.
function Iso-Compilers {
    param([Parameter(Mandatory)][ValidateSet('c','cpp')][string]$Lang)
    # cl.exe and clang-cl.exe compile both C and C++ (the /std flag selects the
    # dialect), so the candidate set is the same for both languages.
    $found = @()
    foreach ($cc in @('cl', 'clang-cl')) {
        if (Get-Command $cc -ErrorAction SilentlyContinue) { $found += $cc }
    }
    return $found
}

# Verify every ISO_REQUIRE compiler is present; throw if any is missing.
function Iso-RequireCheck {
    if (-not $env:ISO_REQUIRE) { return }
    $missing = @()
    foreach ($req in ($env:ISO_REQUIRE -split '\s+' | Where-Object { $_ })) {
        if (-not (Get-Command $req -ErrorAction SilentlyContinue)) { $missing += $req }
    }
    if ($missing.Count -gt 0) {
        throw "iso-harness: required compiler(s) not found: $($missing -join ' ') (ISO_REQUIRE='$($env:ISO_REQUIRE)')"
    }
}

# Compile <Sources> into one .exe with each present compiler and run it.
function Iso-BuildAndRun {
    param(
        [Parameter(Mandatory)][ValidateSet('c','cpp')][string]$Lang,
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string[]]$Sources
    )
    Iso-RequireCheck
    $compilers = Iso-Compilers -Lang $Lang
    if ($compilers.Count -eq 0) {
        throw "iso-harness: no $Lang compiler found (need cl.exe or clang-cl.exe on PATH)"
    }
    $buildDir = Iso-BuildDir
    New-Item -ItemType Directory -Force -Path $buildDir | Out-Null
    $flags = Iso-StrictFlags -Lang $Lang

    foreach ($cc in $compilers) {
        $exe = Join-Path $buildDir "$cc-$Name.exe"
        Write-Host "iso-harness: [$cc] compiling $Name"
        # /Fe: sets the output executable; /Fo: the intermediate obj directory.
        & $cc @flags $Sources "/Fe:$exe" "/Fo:$buildDir\"
        if ($LASTEXITCODE -ne 0) { throw "iso-harness: [$cc] COMPILE FAILED for $Name" }
        Write-Host "iso-harness: [$cc] running $Name"
        & $exe
        if ($LASTEXITCODE -ne 0) { throw "iso-harness: [$cc] RUNTIME FAILURE for $Name" }
    }
}

# Assert that every present compiler REJECTS <Source> under the strict flags.
function Iso-ExpectCompileFail {
    param(
        [Parameter(Mandatory)][ValidateSet('c','cpp')][string]$Lang,
        [Parameter(Mandatory)][string]$Source
    )
    Iso-RequireCheck
    $compilers = Iso-Compilers -Lang $Lang
    if ($compilers.Count -eq 0) {
        throw "iso-harness: no $Lang compiler found for negative test"
    }
    $buildDir = Iso-BuildDir
    New-Item -ItemType Directory -Force -Path $buildDir | Out-Null
    $flags = Iso-StrictFlags -Lang $Lang

    foreach ($cc in $compilers) {
        Write-Host "iso-harness: [$cc] expecting REJECTION of $Source"
        # /c compiles without linking; we only care about the accept/reject verdict.
        & $cc @flags '/c' $Source "/Fo:$buildDir\" *> $null
        if ($LASTEXITCODE -eq 0) {
            throw "iso-harness: [$cc] ACCEPTED non-ISO source $Source — strict flags are not enforcing conformance!"
        }
        Write-Host "iso-harness: [$cc] correctly rejected $Source"
    }
}
