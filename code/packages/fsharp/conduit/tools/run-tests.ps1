# run-tests.ps1 -- Windows counterpart of run-tests.sh for F# Conduit.
#
# Recipe (same as the Unix script): build conduit-capi (Rust), tell the .NET
# native-library resolver exactly where the library is, then run `dotnet test`.
#
# The Rust build and the staging of conduit_capi.dll are delegated to the shared
# helper code\scripts\stage-conduit-capi-windows.ps1.  The helper copies the DLL
# into a private, freshly created directory, so no other file in target\release
# can be picked up by accident.
#
# The build tool prints a recipe's output only when it fails, so this script is
# chatty on purpose and exits with dotnet's own exit code.

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
    # tools\ -> conduit -> fsharp -> packages -> code -> repo root.
    $packageDir = Split-Path -Parent $PSScriptRoot
    $repoRoot = $packageDir
    for ($i = 0; $i -lt 4; $i++) { $repoRoot = Split-Path -Parent $repoRoot }
    $stageHelper = Join-Path $repoRoot 'code\scripts\stage-conduit-capi-windows.ps1'
    $tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { $env:TEMP }
    # Per-package leaf name: the helper wipes and recreates this directory, so
    # another package reusing the helper must never share (and race on) it.
    $stageDir = Join-Path $tempRoot 'conduit-capi-stage-fsharp'

    Write-Host "==> package dir : $packageDir"
    Write-Host "==> repo root   : $repoRoot"
    Write-Host "==> stage dir   : $stageDir"
    if (-not (Test-Path -LiteralPath $stageHelper)) { throw "Missing helper script: $stageHelper" }

    # ---- 1. Build + stage the Rust library -----------------------------------
    & $stageHelper -StageDir $stageDir -RepoRoot $repoRoot

    # ---- 2. Tell the .NET resolver where the library is ------------------------
    # The package's resolver reads CONDUIT_CAPI_PATH first and loads exactly that
    # ABSOLUTE path (the Windows file name has no "lib" prefix).  PATH is a
    # belt-and-braces fallback for the bare DllImport name `conduit_capi`.  This
    # script is its own powershell.exe process, so neither edit can leak out.
    $dll = Join-Path $stageDir 'conduit_capi.dll'
    if (-not (Test-Path -LiteralPath $dll)) { throw "Staged library not found: $dll" }
    $env:CONDUIT_CAPI_PATH = $dll
    $env:PATH = "$stageDir;$env:PATH"
    Write-Host "==> CONDUIT_CAPI_PATH=$env:CONDUIT_CAPI_PATH"

    # ---- 3. dotnet test ---------------------------------------------------------
    Set-Location -LiteralPath $packageDir
    Write-Host '==> dotnet --version'
    $null = Invoke-Native { & dotnet --version }

    # An ARRAY of single-quoted literals: PowerShell hands each element to
    # dotnet.exe as one argument, verbatim, so MSBuild sees the property
    # syntax exactly as the Unix script writes it, with no shell quoting layer.
    $dotnetArgs = @(
        'test',
        'tests/CodingAdventures.Conduit.Tests/CodingAdventures.Conduit.Tests.fsproj',
        '--disable-build-servers',
        '/p:CollectCoverage=true',
        '/p:Include=[CodingAdventures.Conduit.FSharp]*'
    )
    Write-Host "==> dotnet $($dotnetArgs -join ' ')"
    $rc = Invoke-Native { & dotnet @dotnetArgs }
    if ($rc -ne 0) {
        Write-Host "==> dotnet test FAILED with exit code $(Format-ExitCode $rc)"
        exit $rc
    }
    Write-Host '==> dotnet test passed'
    exit 0
}
catch {
    Write-Host "==> ERROR: $($_.Exception.Message)"
    Write-Host $_.ScriptStackTrace
    exit 1
}
