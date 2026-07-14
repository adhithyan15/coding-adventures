<#
.SYNOPSIS
    platform-lib.ps1 — the Windows/MSVC half of the OS-dependent C/C++ harness.

.DESCRIPTION
    This is the PowerShell counterpart to lib/platform-lib.sh, for code that
    talks to the operating system (threads, clocks, sockets, dynamic loading).
    On Windows the compiler of record is MSVC's `cl.exe` (and, when present,
    LLVM's `clang-cl.exe`). It compiles each translation unit under strict
    warnings-as-errors, but — unlike iso-harness — WITHOUT `/permissive-`,
    because Win32 headers and idioms are not strict-ISO:

        C   :  /std:c17   /W4 /WX
        C++ :  /std:c++17 /W4 /WX /EHsc

    Per-OS source selection is done by the build-tool (BUILD_windows names the
    Win32 backend), so this harness compiles only what it is handed.

    Public functions:
        Platform-Os                                    → 'windows'
        Platform-Compilers   -Lang c|cpp               → present MSVC-family compilers
        Platform-BuildAndRun -Lang c|cpp -Name n -Sources …
                                                       → compile+run with each; throws on failure

    Environment knobs (same names as platform-lib.sh):
        PLATFORM_REQUIRE, PLATFORM_INCLUDE, PLATFORM_LIBS (extra .lib tokens,
        e.g. ws2_32.lib), PLATFORM_DEFINES, PLATFORM_CSTD, PLATFORM_CXXSTD,
        PLATFORM_BUILD_DIR (default _build).

    `cl.exe` must already be on PATH — in CI the msvc-dev-cmd action puts it
    there; locally, run from a "Developer Command Prompt".
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Platform-Os { return 'windows' }

function Platform-BuildDir {
    if ($env:PLATFORM_BUILD_DIR) { return $env:PLATFORM_BUILD_DIR }
    return '_build'
}

function Platform-Cstd  { if ($env:PLATFORM_CSTD)   { return $env:PLATFORM_CSTD }   else { return 'c17' } }
function Platform-Cxxstd { if ($env:PLATFORM_CXXSTD) { return $env:PLATFORM_CXXSTD } else { return 'c++17' } }

# Strict flag list for MSVC-family compilers, as an array of tokens. Note: no
# /permissive- (this is OS-dependent, deliberately non-strict-ISO code).
function Platform-StrictFlags {
    param([Parameter(Mandatory)][ValidateSet('c','cpp')][string]$Lang)
    $flags = @('/nologo', '/W4', '/WX')
    if ($Lang -eq 'cpp') {
        $flags += @("/std:$(Platform-Cxxstd)", '/EHsc')
    } else {
        $flags += @("/std:$(Platform-Cstd)")
    }
    foreach ($def in ($env:PLATFORM_DEFINES -split '\s+' | Where-Object { $_ })) {
        $flags += "/D$def"
    }
    foreach ($dir in ($env:PLATFORM_INCLUDE -split '\s+' | Where-Object { $_ })) {
        $flags += "/I$dir"
    }
    return $flags
}

# Return the MSVC-family compilers present on PATH, in a stable order.
function Platform-Compilers {
    param([Parameter(Mandatory)][ValidateSet('c','cpp')][string]$Lang)
    $found = @()
    foreach ($cc in @('cl', 'clang-cl')) {
        if (Get-Command $cc -ErrorAction SilentlyContinue) { $found += $cc }
    }
    return $found
}

# Verify every PLATFORM_REQUIRE compiler is present; throw if any is missing.
function Platform-RequireCheck {
    if (-not $env:PLATFORM_REQUIRE) { return }
    $missing = @()
    foreach ($req in ($env:PLATFORM_REQUIRE -split '\s+' | Where-Object { $_ })) {
        if (-not (Get-Command $req -ErrorAction SilentlyContinue)) { $missing += $req }
    }
    if ($missing.Count -gt 0) {
        throw "platform-harness: required compiler(s) not found: $($missing -join ' ') (PLATFORM_REQUIRE='$($env:PLATFORM_REQUIRE)')"
    }
}

# Compile <Sources> into one .exe with each present compiler (linking any
# $env:PLATFORM_LIBS tokens) and run it.
function Platform-BuildAndRun {
    param(
        [Parameter(Mandatory)][ValidateSet('c','cpp')][string]$Lang,
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string[]]$Sources
    )
    Platform-RequireCheck
    $compilers = Platform-Compilers -Lang $Lang
    if ($compilers.Count -eq 0) {
        throw "platform-harness: no $Lang compiler found (need cl.exe or clang-cl.exe on PATH)"
    }
    $buildDir = Platform-BuildDir
    New-Item -ItemType Directory -Force -Path $buildDir | Out-Null
    $flags = Platform-StrictFlags -Lang $Lang
    $libs = @($env:PLATFORM_LIBS -split '\s+' | Where-Object { $_ })

    foreach ($cc in $compilers) {
        $exe = Join-Path $buildDir "$cc-$Name.exe"
        Write-Host "platform-harness: [$cc] (windows) compiling $Name"
        # Libraries go after the sources; /link separates compiler and linker args.
        if ($libs.Count -gt 0) {
            & $cc @flags $Sources "/Fe:$exe" "/Fo:$buildDir\" /link @libs
        } else {
            & $cc @flags $Sources "/Fe:$exe" "/Fo:$buildDir\"
        }
        if ($LASTEXITCODE -ne 0) { throw "platform-harness: [$cc] COMPILE FAILED for $Name" }
        Write-Host "platform-harness: [$cc] running $Name"
        & $exe
        if ($LASTEXITCODE -ne 0) { throw "platform-harness: [$cc] RUNTIME FAILURE for $Name" }
    }
}
