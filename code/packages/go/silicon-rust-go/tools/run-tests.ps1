# run-tests.ps1 -- Windows recipe for Go silicon-rust-go.
#
# Unix recipe (BUILD):
#     cargo build --manifest-path ../../rust/Cargo.toml -p silicon-rust-cgo --release && go test ./...
#
# Windows recipe: the Go package links libsilicon_rust_cgo.a through cgo, and cgo on
# Windows means MinGW-w64 gcc, so the Rust staticlib must be the GNU-ABI one
# (target x86_64-pc-windows-gnu), not the cdylib's import library. Linking the import
# library made the test binary exit 0xc0000135 (STATUS_DLL_NOT_FOUND), because
# silicon_rust_cgo.dll was not on PATH. The shared helper
# code\scripts\build-conduit-capi-windows-gnu.ps1 (parametrized by -CrateDir and
# -ArchiveName) builds the archive, finds a MinGW gcc and puts it first on PATH; the
# `#cgo windows LDFLAGS` line in silicon_rust_go.go points at the archive it produces.
#
# Like the Unix script this builds the Rust dependency itself, so the BUILD entry
# is self-sufficient even when run from a pre-generated plan.
#
# The build tool prints a package's output only when its recipe fails, so this
# script is chatty on purpose and exits with `go test`'s own exit code.

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
    # tools\ -> silicon-rust-go -> go -> packages -> code -> repo root.
    $packageDir = Split-Path -Parent $PSScriptRoot
    $repoRoot = $packageDir
    for ($i = 0; $i -lt 4; $i++) { $repoRoot = Split-Path -Parent $repoRoot }
    $helper = Join-Path $repoRoot 'code\scripts\build-conduit-capi-windows-gnu.ps1'
    if (-not (Test-Path -LiteralPath $helper)) { throw "Missing helper script: $helper" }

    Write-Host "==> package dir : $packageDir"
    Write-Host "==> repo root   : $repoRoot"

    # ---- 1. Rust staticlib (GNU ABI) + MinGW gcc on PATH -------------------------
    # Invoked with `&` so its PATH / CC edits apply to this process.
    & $helper -RepoRoot $repoRoot -CrateDir 'silicon-rust-cgo' -ArchiveName 'libsilicon_rust_cgo.a'

    # ---- 2. Same go test command as the Unix script ------------------------------
    Set-Location -LiteralPath $packageDir
    Write-Host '==> go test ./... -v -cover'
    $rc = Invoke-Native { & go test ./... -v -cover }
    if ($rc -ne 0) { Write-Host "==> go test FAILED with exit code $rc"; exit $rc }
    Write-Host '==> go test passed'
    exit 0
}
catch {
    Write-Host "==> ERROR: $($_.Exception.Message)"
    Write-Host $_.ScriptStackTrace
    exit 1
}
