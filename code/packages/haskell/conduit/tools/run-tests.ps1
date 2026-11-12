# run-tests.ps1 -- Windows counterpart of run-tests.sh for Haskell conduit.
#
# Recipe: build conduit-capi (Rust), stage its DLL + import library in a private
# directory (see code\scripts\stage-conduit-capi-windows.ps1 for why we never
# point GHC at target\release), then run `cabal test`.
#
# The build tool prints a package's output only when the recipe fails, so this
# script is chatty on purpose and exits with cabal's own non-zero exit code.

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
    # tools\ -> conduit -> haskell -> packages -> code -> repo root.
    $packageDir = Split-Path -Parent $PSScriptRoot
    $repoRoot = $packageDir
    for ($i = 0; $i -lt 4; $i++) { $repoRoot = Split-Path -Parent $repoRoot }
    $includeDir = Join-Path $repoRoot 'code\packages\rust\conduit-capi\include'
    $stageHelper = Join-Path $repoRoot 'code\scripts\stage-conduit-capi-windows.ps1'
    $tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { $env:TEMP }
    $stageDir = Join-Path $tempRoot 'conduit-capi-stage'

    Write-Host "==> package dir : $packageDir"
    Write-Host "==> repo root   : $repoRoot"
    Write-Host "==> stage dir   : $stageDir"
    if (-not (Test-Path -LiteralPath $stageHelper)) { throw "Missing helper script: $stageHelper" }
    if (-not (Test-Path -LiteralPath $includeDir)) { throw "Missing include dir: $includeDir" }

    # ---- 1. Build + stage the Rust library -----------------------------------
    & $stageHelper -StageDir $stageDir -RepoRoot $repoRoot

    # ---- 2. Arrange PATH for the cabal process only ---------------------------
    # (This script is its own powershell.exe process, so editing $env:PATH here
    # cannot leak into the caller.)
    #   * stage dir FIRST: the test exe finds conduit_capi.dll through PATH.
    #   * Strawberry Perl ships a gcc that hijacks hsc2hs, so drop it.
    #   * `network` has a configure script that needs sh.exe.  If none is on
    #     PATH, APPEND (never prepend) an MSYS2 usr\bin: it contains a
    #     coreutils link.exe that would shadow MSVC's linker if it came first.
    $entries = @($env:PATH -split ';' | Where-Object { $_ -ne '' })
    $kept = @($entries | Where-Object { $_ -notlike '*\Strawberry\*' -and $_ -notlike '*\Strawberry' })
    foreach ($e in $entries) {
        if ($kept -notcontains $e) { Write-Host "==> PATH: removed $e" }
    }
    $env:PATH = (@($stageDir) + $kept) -join ';'
    Write-Host "==> PATH: prepended $stageDir"

    if (Get-Command sh.exe -ErrorAction SilentlyContinue) {
        Write-Host "==> sh.exe already on PATH: $((Get-Command sh.exe).Source)"
    } else {
        $msys = @('C:\ghcup\msys64\usr\bin', 'C:\msys64\usr\bin') |
            Where-Object { Test-Path -LiteralPath (Join-Path $_ 'sh.exe') } |
            Select-Object -First 1
        if ($msys) {
            $env:PATH = "$env:PATH;$msys"
            Write-Host "==> sh.exe not on PATH; appended $msys"
        } else {
            Write-Host '==> WARNING: sh.exe not found anywhere; the network package configure step may fail'
        }
    }
    foreach ($tool in 'cabal', 'ghc', 'sh') {
        $found = Get-Command "$tool.exe" -ErrorAction SilentlyContinue
        if ($found) { Write-Host "==> $tool -> $($found.Source)" } else { Write-Host "==> $tool -> NOT FOUND" }
    }
    if (-not (Get-Command cabal -ErrorAction SilentlyContinue)) { throw 'cabal was not found on PATH.' }
    Write-Host '==> where.exe conduit_capi.dll (PATH as cabal will see it)'
    $null = Invoke-Native { & where.exe conduit_capi.dll }

    # ---- 3. cabal test ---------------------------------------------------------
    Set-Location -LiteralPath $packageDir
    $cabalArgs = @(
        'test',
        "--extra-lib-dirs=$stageDir",
        "--extra-include-dirs=$includeDir",
        '--test-show-details=always'
    )
    Write-Host "==> cabal $($cabalArgs -join ' ')"
    $rc = Invoke-Native { & cabal @cabalArgs }
    if ($rc -ne 0) {
        Write-Host "==> cabal test FAILED with exit code $(Format-ExitCode $rc)"
        # Second run is for diagnostics only; its exit code is ignored and the
        # ORIGINAL failure code is what we report.
        Write-Host '==> Re-running with -v2 for diagnostics'
        $verboseArgs = $cabalArgs + '-v2'
        $null = Invoke-Native { & cabal @verboseArgs }
        exit $rc
    }
    Write-Host '==> cabal test passed'
    exit 0
}
catch {
    Write-Host "==> ERROR: $($_.Exception.Message)"
    Write-Host $_.ScriptStackTrace
    exit 1
}
