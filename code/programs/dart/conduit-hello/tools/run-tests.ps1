# run-tests.ps1 -- Windows counterpart of run-tests.sh for the Dart conduit-hello program.
#
# Recipe: build conduit-capi (Rust cdylib) and conduit-dart-bridge (the Rust
# cdylib that lets native server threads post results back into Dart), stage
# both DLLs in one private directory, point Dart at them, then run the smoke tests.
#
# Dart binds to the DLLs at run time with dart:ffi, so unlike the Haskell
# recipe no import library is needed.  The build tool prints a package's output
# only when the recipe fails, so this script is chatty on purpose and exits with
# dart's own non-zero exit code.

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
    # tools\ -> conduit-hello -> dart -> programs -> code -> repo root.
    $packageDir = Split-Path -Parent $PSScriptRoot
    $repoRoot = $packageDir
    for ($i = 0; $i -lt 4; $i++) { $repoRoot = Split-Path -Parent $repoRoot }
    $stageHelper = Join-Path $repoRoot 'code\scripts\stage-conduit-capi-windows.ps1'
    $cargoWorkspace = Join-Path $repoRoot 'code\packages\rust'
    $tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { $env:TEMP }
    # Language-specific leaf name: the helper wipes and recreates this directory,
    # so another language reusing the helper must never share (and race on) it.
    $stageDir = Join-Path $tempRoot 'conduit-capi-stage-dart-hello'

    Write-Host "==> package dir : $packageDir"
    Write-Host "==> repo root   : $repoRoot"
    Write-Host "==> stage dir   : $stageDir"
    if (-not (Test-Path -LiteralPath $stageHelper)) { throw "Missing helper script: $stageHelper" }

    # ---- 1. Build + stage conduit_capi.dll ------------------------------------
    # The helper empties $stageDir first, so it MUST run before the bridge copy.
    & $stageHelper -StageDir $stageDir -RepoRoot $repoRoot

    # ---- 2. Build the Dart bridge and stage it next to conduit_capi.dll --------
    # The bridge has a build.rs that compiles C files with the `cc` crate, so
    # this needs the MSVC toolchain (present on the CI runner).
    Write-Host "==> cargo build -p conduit-dart-bridge --release (in $cargoWorkspace)"
    Push-Location -LiteralPath $cargoWorkspace
    try { $rc = Invoke-Native { & cargo build -p conduit-dart-bridge --release } }
    finally { Pop-Location }
    if ($rc -ne 0) {
        Write-Host "==> cargo build conduit-dart-bridge FAILED with exit code $(Format-ExitCode $rc)"
        exit $rc
    }

    # Same target-dir logic as the helper: CARGO_TARGET_DIR (if set) is relative
    # to cargo's working directory, and Join-Path would mangle an absolute value.
    $targetRoot = Join-Path $cargoWorkspace 'target'
    if ($env:CARGO_TARGET_DIR) {
        if ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
            $targetRoot = $env:CARGO_TARGET_DIR
        } else {
            $targetRoot = Join-Path $cargoWorkspace $env:CARGO_TARGET_DIR
        }
    }
    $releaseDir = Join-Path $targetRoot 'release'
    $bridgeBuilt = Join-Path $releaseDir 'conduit_dart_bridge.dll'
    if (-not (Test-Path -LiteralPath $bridgeBuilt)) {
        Write-Host "conduit_dart_bridge* in ${releaseDir}:"
        if (Test-Path -LiteralPath $releaseDir) {
            Get-ChildItem -LiteralPath $releaseDir -Filter 'conduit_dart_bridge*' |
                ForEach-Object { Write-Host "  $($_.Name)" }
        } else {
            Write-Host '  (directory does not exist)'
        }
        throw "conduit_dart_bridge.dll not found in $releaseDir"
    }
    Copy-Item -LiteralPath $bridgeBuilt -Destination (Join-Path $stageDir 'conduit_dart_bridge.dll')

    # ---- 3. Point Dart at the DLLs ---------------------------------------------
    # Always set BOTH variables to absolute paths.  lib\src\ffi.dart can derive the
    # bridge path from CONDUIT_CAPI_PATH, but its _dirname only splits on '/', so
    # a backslash path would derive an empty directory.  Explicit is safe.
    $capiDll = Join-Path $stageDir 'conduit_capi.dll'
    $bridgeDll = Join-Path $stageDir 'conduit_dart_bridge.dll'
    foreach ($f in @($capiDll, $bridgeDll)) {
        if (-not (Test-Path -LiteralPath $f)) { throw "Staged DLL is missing: $f" }
    }
    $env:CONDUIT_CAPI_PATH = $capiDll
    $env:CONDUIT_DART_BRIDGE_PATH = $bridgeDll
    Write-Host "==> CONDUIT_CAPI_PATH        = $env:CONDUIT_CAPI_PATH"
    Write-Host "==> CONDUIT_DART_BRIDGE_PATH = $env:CONDUIT_DART_BRIDGE_PATH"

    # Fallback only: if a dependent DLL is ever looked up by name, the loader
    # searches PATH.  (This script is its own powershell.exe process, so editing
    # $env:PATH cannot leak into the caller.)
    $env:PATH = "$stageDir;$env:PATH"
    if (-not (Get-Command dart -ErrorAction SilentlyContinue)) { throw 'dart was not found on PATH.' }
    $rc = Invoke-Native { & dart --version }
    if ($rc -ne 0) { Write-Host "==> dart --version FAILED with exit code $(Format-ExitCode $rc)"; exit $rc }

    # ---- 4. Same dart commands as run-tests.sh ---------------------------------
    Set-Location -LiteralPath $packageDir

    Write-Host '==> Fetching Dart dependencies'
    $pubArgs = @('pub', 'get')
    $rc = Invoke-Native { & dart @pubArgs }
    if ($rc -ne 0) {
        Write-Host "==> dart pub get FAILED with exit code $(Format-ExitCode $rc)"
        exit $rc
    }

    Write-Host '==> Running smoke tests'
    $testArgs = @('test', '--reporter=expanded', 'test/smoke_test.dart')
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
