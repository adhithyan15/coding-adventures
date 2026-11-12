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

# Diagnostics must never fail the build: run a block under a heading, swallow
# any error, and keep going.  (Copied from stage-conduit-capi-windows.ps1; the
# scripts are separate powershell.exe processes and share no code.)
function Invoke-Diag([string]$Title, [scriptblock]$Block) {
    Write-Host "---- $Title"
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $Block | Out-Host }
    catch { Write-Host "(diagnostic failed: $($_.Exception.Message))" }
    finally { $ErrorActionPreference = $saved }
}

# A failing `network` configure step (or any other dependency build) is logged
# to a per-package build log under the Cabal logs directory, NOT to the console,
# so after a failed build print the tail of the newest log.  Best effort: any
# error here is swallowed so the original cabal exit code is what gets reported.
function Show-CabalLogTail {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $logDir = Join-Path $env:APPDATA 'cabal\logs'
        if (Test-Path -LiteralPath $logDir) {
            $f = Get-ChildItem -LiteralPath $logDir -Recurse -Filter '*.log' -ErrorAction SilentlyContinue |
                Sort-Object LastWriteTime -Descending |
                Select-Object -First 1
            if ($f) {
                Write-Host "==> newest cabal log: $($f.FullName)"
                Get-Content -LiteralPath $f.FullName -Tail 150 | Out-Host
            } else {
                Write-Host "==> no *.log files under $logDir"
            }
        } else {
            Write-Host "==> no cabal log directory at $logDir"
        }
    }
    catch { Write-Host "(could not show cabal log: $($_.Exception.Message))" }
    finally { $ErrorActionPreference = $saved }
}

try {
    # tools\ -> conduit -> haskell -> packages -> code -> repo root.
    $packageDir = Split-Path -Parent $PSScriptRoot
    $repoRoot = $packageDir
    for ($i = 0; $i -lt 4; $i++) { $repoRoot = Split-Path -Parent $repoRoot }
    $includeDir = Join-Path $repoRoot 'code\packages\rust\conduit-capi\include'
    $stageHelper = Join-Path $repoRoot 'code\scripts\stage-conduit-capi-windows.ps1'
    $tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { $env:TEMP }
    # Language-specific leaf name: the helper wipes and recreates this directory,
    # so another language reusing the helper must never share (and race on) it.
    $stageDir = Join-Path $tempRoot 'conduit-capi-stage-haskell'

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

    # ---- 3. Toolchain diagnostics (best effort) --------------------------------
    # Which tools cabal/GHC will pick up, in PATH order.  A stray gcc, ld, perl,
    # sh or clang earlier on PATH than GHC's own bundled toolchain is the usual
    # cause of confusing configure and link failures.
    Invoke-Diag 'tools that can shadow GHC toolchain (PATH order)' {
        Get-Command -All sh.exe, gcc.exe, clang.exe, ld.exe, perl.exe -ErrorAction SilentlyContinue |
            ForEach-Object { $_.Source }
    }
    Invoke-Diag 'ghc --version and cabal --version' {
        & ghc --version
        & cabal --version
    }
    # Only the Cabal config lines that change where tools, libraries and package
    # indexes come from; the rest of the (long, commented) file is noise.
    Invoke-Diag 'cabal config (relevant lines)' {
        $cfg = if ($env:CABAL_CONFIG -and (Test-Path -LiteralPath $env:CABAL_CONFIG)) {
            $env:CABAL_CONFIG
        } else {
            Join-Path $env:APPDATA 'cabal\config'
        }
        Write-Host "config file: $cfg"
        if (Test-Path -LiteralPath $cfg) {
            # Anchored: a line that merely contains one of these words (for example in a URL or credential) is not printed.
            Select-String -LiteralPath $cfg -Pattern '^\s*(extra-prog-path|extra-lib-dirs|extra-include-dirs|store-dir|remote-repo-cache)\s*:|^\s*repository\s' |
                ForEach-Object { $_.Line }
        } else {
            Write-Host 'config file does not exist'
        }
    }

    # ---- 4. cabal build, then cabal test ----------------------------------------
    # Two phases so a compile/link failure is separated from a test failure.  The
    # flags MUST be identical in both phases, otherwise Cabal sees a changed
    # configuration and reconfigures (rebuilding dependencies) between them.
    Set-Location -LiteralPath $packageDir
    $flags = @("--extra-lib-dirs=$stageDir", "--extra-include-dirs=$includeDir")

    Write-Host "==> cabal build --enable-tests $($flags -join ' ')"
    $rc = Invoke-Native { & cabal build --enable-tests @flags }
    if ($rc -ne 0) {
        Write-Host "==> cabal build FAILED with exit code $(Format-ExitCode $rc)"
        # The -v3 rerun is for diagnostics only; its exit code is ignored and the
        # ORIGINAL failure code is what we report.
        Write-Host '==> Re-running the build with -v3 to show the failing compiler/linker command'
        # WHY: at -v3 Cabal may print the child-process environment, so the rerun
        # is skipped whenever the job holds anything that looks like a secret.
        # This pull_request job has none, but the script may be reused elsewhere.
        # Names only are collected and printed, never values.
        $sensitive = @(Get-ChildItem Env: | Where-Object { $_.Name -match 'TOKEN|SECRET|PASSWORD|CREDENTIAL|PRIVATE|_KEY$' -and $_.Name -ne 'GITHUB_ACTIONS' } | ForEach-Object { $_.Name })
        if ($sensitive.Count -eq 0) {
            $null = Invoke-Native { & cabal build --enable-tests @flags -v3 }
        } else {
            Write-Host "==> NOT re-running with -v3: at -v3 Cabal can print the process environment, and these secret-looking variables are set: $($sensitive -join ', ')"
        }
        Show-CabalLogTail
        exit $rc
    }

    # 'direct' streams the test's output as it runs, so a hung test still shows
    # how far it got ('always' would buffer it until the test finished).
    Write-Host "==> cabal test $($flags -join ' ') --test-show-details=direct"
    $rc = Invoke-Native { & cabal test @flags --test-show-details=direct }
    if ($rc -ne 0) {
        # No rerun: the tests already ran with full output above.
        Write-Host "==> cabal test FAILED with exit code $(Format-ExitCode $rc)"
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
