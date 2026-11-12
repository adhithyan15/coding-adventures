# run-tests.ps1 -- Windows counterpart of run-tests.sh for conduit-hello.
#
# Mirrors code\packages\go\conduit\tools\run-tests.ps1: the Rust staticlib
# (GNU ABI, for MinGW-w64 gcc / cgo) must exist before `go test` runs, and the
# shared helper code\scripts\build-conduit-capi-windows-gnu.ps1 builds it, finds a
# MinGW gcc and puts it first on PATH.  The cgo link line itself lives in the
# go/conduit package that this program imports.
#
# The build tool prints a program's output only when its recipe fails, so this
# script is chatty on purpose and exits with `go test`'s own exit code.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Invoke-Native([scriptblock]$Command) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $global:LASTEXITCODE = 0
    try { & $Command | Out-Host } finally { $ErrorActionPreference = $saved }
    return [int]$global:LASTEXITCODE
}

try {
    # tools\ -> conduit-hello -> go -> programs -> code -> repo root.
    $programDir = Split-Path -Parent $PSScriptRoot
    $repoRoot = $programDir
    for ($i = 0; $i -lt 4; $i++) { $repoRoot = Split-Path -Parent $repoRoot }
    $helper = Join-Path $repoRoot 'code\scripts\build-conduit-capi-windows-gnu.ps1'
    if (-not (Test-Path -LiteralPath $helper)) { throw "Missing helper script: $helper" }

    Write-Host "==> program dir : $programDir"
    Write-Host "==> repo root   : $repoRoot"

    & $helper -RepoRoot $repoRoot

    Set-Location -LiteralPath $programDir
    Write-Host '==> go test ./... -v'
    $rc = Invoke-Native { & go test ./... -v }
    if ($rc -ne 0) { Write-Host "==> go test FAILED with exit code $rc"; exit $rc }
    Write-Host '==> go test passed'
    exit 0
}
catch {
    Write-Host "==> ERROR: $($_.Exception.Message)"
    Write-Host $_.ScriptStackTrace
    exit 1
}
