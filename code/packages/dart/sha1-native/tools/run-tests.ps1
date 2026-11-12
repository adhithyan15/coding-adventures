# run-tests.ps1 -- Windows counterpart of run-tests.sh for sha1-native.
#
# Recipe: build the Rust cdylib in this package (cargo build --release), point
# Dart's dart:ffi loader at the built DLL through SHA1_NATIVE_PATH, then run
# `dart pub get` and `dart test` exactly like run-tests.sh.
#
# The build tool prints a package's output only when the recipe fails, so this
# script is chatty on purpose and exits with the failing tool's own exit code.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Signed 32-bit exit codes; NTSTATUS values are only readable in hex
# (-1073741515 = 0xC0000135 = a DLL could not be found).
function Format-ExitCode([int]$Code) {
    $text = '{0} (0x{1:X8})' -f $Code, $Code
    if ($Code -eq -1073741515) { $text += ' = STATUS_DLL_NOT_FOUND' }
    return $text
}

# Run a native command with output going to the console and return its exit
# code.  PowerShell 5.1 turns native stderr text (cargo's progress output!)
# into errors under 'Stop', so relax that while the command runs.
function Invoke-Native([scriptblock]$Command) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $global:LASTEXITCODE = 0
    try { & $Command | Out-Host } finally { $ErrorActionPreference = $saved }
    return [int]$global:LASTEXITCODE
}

try {
    # tools\ -> package directory.
    $packageDir = Split-Path -Parent $PSScriptRoot
    Set-Location -LiteralPath $packageDir
    Write-Host "==> package dir : $packageDir"

    # ---- 1. Build the Rust cdylib ----------------------------------------------
    Write-Host '==> Building sha1-native (release cdylib)'
    $rc = Invoke-Native { & cargo build --release -q }
    if ($rc -ne 0) {
        Write-Host "==> cargo build FAILED with exit code $(Format-ExitCode $rc)"
        exit $rc
    }

    # Same layout as run-tests.sh: target\release inside the package.  A Windows
    # cdylib has no "lib" prefix: sha1_native.dll.
    $libFile = Join-Path $packageDir 'target\release\sha1_native.dll'
    if (-not (Test-Path -LiteralPath $libFile)) {
        Write-Host "target\release contents:"
        if (Test-Path -LiteralPath (Join-Path $packageDir 'target\release')) {
            Get-ChildItem -LiteralPath (Join-Path $packageDir 'target\release') -Filter '*.dll' |
                ForEach-Object { Write-Host "  $($_.Name)" }
        }
        throw "expected shared library not found at $libFile"
    }

    # ---- 2. Dart ---------------------------------------------------------------
    if (-not (Get-Command dart -ErrorAction SilentlyContinue)) { throw 'dart was not found on PATH.' }

    Write-Host '==> Fetching Dart dependencies'
    $rc = Invoke-Native { & dart pub get }
    if ($rc -ne 0) {
        Write-Host "==> dart pub get FAILED with exit code $(Format-ExitCode $rc)"
        exit $rc
    }

    # Process-local: this script is its own powershell.exe, so the variable
    # cannot leak into the build tool.  The path is absolute, as ffi.dart requires.
    $env:SHA1_NATIVE_PATH = $libFile
    Write-Host "==> Running dart test (SHA1_NATIVE_PATH=$libFile)"
    $testArgs = @('test', '--reporter=expanded')
    $rc = Invoke-Native { & dart @testArgs }
    if ($rc -ne 0) {
        Write-Host "==> dart test FAILED with exit code $(Format-ExitCode $rc)"
        exit $rc
    }
    Write-Host '==> dart test passed'
    exit 0
}
catch {
    Write-Host "==> ERROR: $($_.Exception.Message)"
    Write-Host $_.ScriptStackTrace
    exit 1
}
