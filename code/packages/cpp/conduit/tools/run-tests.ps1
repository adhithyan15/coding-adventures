# run-tests.ps1 -- Windows counterpart of run-tests.sh for C++ Conduit.
#
# Unix recipe (tools/run-tests.sh):
#     cargo rustc --release --crate-type staticlib -- --print native-static-libs   (conduit-capi)
#     g++/clang++ -std=c++17 -Wall -Wextra -Wpedantic -Werror tests/test_*.cpp libconduit_capi.a <native libs>
#     run each test binary; fail if any fails
#
# Windows recipe: the same three steps with MSVC.
#   * The shared helper code\scripts\build-conduit-capi-windows-msvc.ps1 fixes PATH so
#     `link` is MSVC's (not Git's GNU link.exe), asserts cl/link/cargo, builds the
#     MSVC-ABI staticlib conduit_capi.lib and works out the system libraries it needs
#     (kernel32 advapi32 ntdll userenv bcrypt ws2_32 synchronization).
#   * cl.exe compiles each test with /std:c++17 /EHsc /W4 /MD.  /MD (dynamic CRT) is
#     what Rust's MSVC std is built against; /MT would mix two C runtimes.
#   * tests\test_server.cpp is the end-to-end suite: it drives a real server over a
#     loopback socket.  tests\net_compat.h maps its BSD-socket calls onto Winsock.
#
# Why cl.exe directly and not CMake: the Unix recipe compiles the tests straight from
# the shell, CMakeLists.txt only describes the header-only library for consumers, and
# a generator choice (Visual Studio vs Ninja) would be one more thing to get wrong
# without a Windows machine to try it on.  CMakeLists.txt does know the MSVC library
# name and system libs, so a CMake consumer links the same way.
#
# The build tool prints a package's output only when its recipe fails, so this script
# is chatty on purpose and exits with the failing tool's own exit code.  A failing
# test does not stop the others: every suite runs, then the script reports and fails.
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
    # tools\ -> conduit -> cpp -> packages -> code -> repo root.
    $packageDir = Split-Path -Parent $PSScriptRoot
    $repoRoot = $packageDir
    for ($i = 0; $i -lt 4; $i++) { $repoRoot = Split-Path -Parent $repoRoot }
    $helper = Join-Path $repoRoot 'code\scripts\build-conduit-capi-windows-msvc.ps1'
    if (-not (Test-Path -LiteralPath $helper)) { throw "Missing helper script: $helper" }

    Write-Host "==> package dir : $packageDir"
    Write-Host "==> repo root   : $repoRoot"
    Set-Location -LiteralPath $packageDir

    # ---- 1. PATH fix + Rust staticlib + system libs ---------------------------------
    # Invoked with `&` so its PATH edit applies to this process.
    $capi = & $helper -RepoRoot $repoRoot | Select-Object -Last 1
    if (-not $capi -or -not $capi.StaticLib) { throw 'the conduit-capi helper returned no result' }

    # ---- 2. Compile and run each suite ---------------------------------------------
    # Output dir is _build\, NOT build\ (mirrors run-tests.sh: build\ would collide
    # with the BUILD file on case-insensitive file systems).
    New-Item -ItemType Directory -Force -Path '_build' | Out-Null
    $clFlags = @('/nologo', '/std:c++17', '/permissive-', '/EHsc', '/W4', '/MD',
                 '/Iinclude', '/Itests', "/I$($capi.CapiInclude)")

    $failed = 0
    $total = 0
    $passed = 0
    foreach ($suite in 'test_response', 'test_application', 'test_server') {
        $total++
        $exe = "_build\$suite.exe"
        Write-Host "==> Compiling $suite"
        $rc = Invoke-Native {
            & cl @clFlags "tests\$suite.cpp" "/Fe:$exe" "/Fo:_build\$suite.obj" /link $capi.StaticLib @($capi.SystemLibs)
        }
        if ($rc -ne 0) { Write-Host "==> cl FAILED for $suite with exit code $(Format-ExitCode $rc)"; exit $rc }
        if (-not (Test-Path -LiteralPath $exe)) { throw "cl reported success but $exe does not exist" }

        Write-Host "==> Running $suite"
        $rc = Invoke-Native { & ".\$exe" }
        if ($rc -eq 0) { $passed++ }
        else { $failed++; Write-Host "==> $suite FAILED with exit code $(Format-ExitCode $rc)" }
    }

    Write-Host ''
    Write-Host "Suites: $passed / $total passed, $failed failed"
    if ($failed -ne 0) { exit 1 }
    exit 0
}
catch {
    Write-Host "==> ERROR: $($_.Exception.Message)"
    Write-Host $_.ScriptStackTrace
    exit 1
}
