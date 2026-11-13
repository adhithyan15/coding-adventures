# run-tests.ps1 -- Windows counterpart of the Unix BUILD for lua/conduit.
#
# Unix recipe:
#     cd ext/conduit_native && cargo build --release && cd ../..
#     <copy the built library to conduit/conduit_native.so>
#     luarocks install luasocket 2>/dev/null || true
#     cd tests && busted . --verbose --pattern=test_
#
# WHY THIS IS A SCRIPT
# --------------------
# tests/test_server.lua is the end-to-end suite: it drives a real server with luasocket's
# HTTP client.  When luasocket is missing it does NOT fail, it calls
#     pending("luasocket is not installed -- skipping E2E server tests")
# so the old Windows recipe (`luarocks install luasocket 2>NUL || echo skipped`) could
# go green without ever running the server tests.  This script makes luasocket a hard
# requirement: it installs it, then loads it, and fails otherwise.
#
# luasocket is a C module, so luarocks compiles it with MSVC.  The first Windows run
# of lua/tcp_client showed the trap: the bare name `link` resolved to Git for Windows'
# GNU coreutils link.exe (C:\Program Files\Git\usr\bin), which sits ahead of MSVC's on
# PATH, and the link step died with `link: unknown option -- d`.  PATH is edited for
# this process only (it is its own powershell.exe), then the script asserts that
# `link` is MSVC's.
#
# The build tool prints a package's output only when its recipe fails, so this script
# is chatty on purpose and exits with the failing tool's own exit code.

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

try {
    # tools\ -> package directory (the build tool already runs us from there).
    $packageDir = Split-Path -Parent $PSScriptRoot
    Set-Location -LiteralPath $packageDir
    Write-Host "==> package dir : $packageDir"

    # ---- 1. Make `link` mean MSVC's linker ---------------------------------------
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

    $link = Get-Command link -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $link) { throw 'No link.exe on PATH: the MSVC environment was not set up for this job.' }
    Write-Host "==> link        : $($link.Source)"
    if ($link.Source -notmatch 'Microsoft Visual Studio|MSVC|Windows Kits') {
        throw "link resolved to $($link.Source), which is not MSVC's linker."
    }
    foreach ($tool in 'cl', 'cargo', 'lua', 'luarocks', 'busted') {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "$tool was not found on PATH." }
    }

    # ---- 2. Build the Rust extension and place it where the tests load it -------
    Write-Host '==> cargo build --release (ext\conduit_native)'
    Push-Location -LiteralPath (Join-Path $packageDir 'ext\conduit_native')
    try { $rc = Invoke-Native { & cargo build --release } } finally { Pop-Location }
    if ($rc -ne 0) { Write-Host "==> cargo build FAILED with exit code $rc"; exit $rc }

    $built = Join-Path $packageDir 'ext\conduit_native\target\release\conduit_native.dll'
    if (-not (Test-Path -LiteralPath $built)) { throw "conduit_native.dll not found at $built" }
    Copy-Item -LiteralPath $built -Destination (Join-Path $packageDir 'conduit\conduit_native.dll') -Force
    Write-Host '==> copied conduit_native.dll to conduit\'

    # ---- 3. luasocket is REQUIRED (it gates the end-to-end suite) ----------------
    Write-Host '==> luarocks install luasocket'
    $rc = Invoke-Native { & luarocks install luasocket }
    if ($rc -ne 0) { Write-Host "==> luarocks install FAILED with exit code $rc"; exit $rc }
    $rc = Invoke-Native { & lua -e "require('socket'); require('socket.http'); require('ltn12'); print('luasocket loads: the end-to-end suite will run')" }
    if ($rc -ne 0) { Write-Host "==> luasocket installed but cannot be loaded (exit code $rc): the E2E tests would be skipped"; exit 1 }

    # ---- 4. Same busted command as the Unix BUILD -------------------------------
    Set-Location -LiteralPath (Join-Path $packageDir 'tests')
    Write-Host '==> busted'
    $rc = Invoke-Native { & busted . --verbose --pattern=test_ }
    if ($rc -ne 0) { Write-Host "==> busted FAILED with exit code $rc"; exit $rc }
    Write-Host '==> busted passed'
    exit 0
}
catch {
    Write-Host "==> ERROR: $($_.Exception.Message)"
    Write-Host $_.ScriptStackTrace
    exit 1
}
