# run-tests.ps1 -- Windows counterpart of the Unix BUILD for lua/tcp_client.
#
# Unix recipe:  luarocks install luasocket --local && cd tests && busted ...
#
# luasocket is a C module, so luarocks compiles it with the MSVC toolchain the CI
# job puts on PATH.  The first Windows run showed the trap this script avoids:
# `cl` compiled every file, then the link step died with
#
#     link: unknown option -- d
#
# because the bare name `link` resolved to Git for Windows' GNU coreutils
# `link.exe` (C:\Program Files\Git\usr\bin), which sits ahead of MSVC's
# `link.exe` on PATH.  This script removes Git's Unix tool directories from PATH
# for this process only (it is its own powershell.exe, so nothing leaks into the
# build tool), then checks that `link` is MSVC's before running luarocks.
#
# The build tool prints a package's output only when its recipe fails, so this
# script is chatty on purpose and exits with the failing tool's own exit code.

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
    # tools\ -> package directory (the build tool already runs us from there).
    $packageDir = Split-Path -Parent $PSScriptRoot
    Set-Location -LiteralPath $packageDir
    Write-Host "==> package dir : $packageDir"

    # ---- 1. Make `link` mean MSVC's linker ---------------------------------------
    $kept = @()
    foreach ($entry in ($env:PATH -split ';')) {
        if ($entry -eq '') { continue }
        # Git for Windows' MSYS tools (link.exe, find.exe, sort.exe, ...) shadow
        # the Windows/MSVC tools of the same name.
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
    if (-not (Get-Command cl -ErrorAction SilentlyContinue)) { throw 'cl.exe is not on PATH.' }
    if (-not (Get-Command luarocks -ErrorAction SilentlyContinue)) { throw 'luarocks was not found on PATH.' }
    if (-not (Get-Command busted -ErrorAction SilentlyContinue)) { throw 'busted was not found on PATH.' }

    # ---- 2. Install luasocket into the tree busted lives in -----------------------
    # No --local: the user tree would need extra LUA_PATH/LUA_CPATH entries.
    Write-Host '==> luarocks install luasocket'
    $rc = Invoke-Native { & luarocks install luasocket }
    if ($rc -ne 0) { Write-Host "==> luarocks install FAILED with exit code $rc"; exit $rc }

    # ---- 3. Same busted command as the Unix BUILD -------------------------------
    Set-Location -LiteralPath (Join-Path $packageDir 'tests')
    $env:LUA_PATH = '../src/?.lua;../src/?/init.lua;;'
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
