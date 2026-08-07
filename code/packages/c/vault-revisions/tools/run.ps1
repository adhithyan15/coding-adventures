<#
    run.ps1 — build & run the vault-revisions test on Windows. vault-revisions is
    OS-agnostic; its only OS dependency is os-platform's thread backend (osp_mutex),
    whose Win32 source needs only the CRT + kernel32 (linked by default). The test
    spawns worker threads via os-platform's thread.
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self
$d = $self
while ($d -and -not (Test-Path (Join-Path $d 'code\packages\c\platform-harness'))) {
    $parent = Split-Path -Parent $d
    if ($parent -eq $d) { break }
    $d = $parent
}
$harness = Join-Path $d 'code\packages\c\platform-harness'
$iso = Join-Path $d 'code\packages\c\iso-harness'
$osp = Join-Path $d 'code\packages\c\os-platform'
if (-not (Test-Path (Join-Path $harness 'lib\platform-lib.ps1'))) {
    throw "platform-harness not found (searched upward from $self)"
}
$env:PLATFORM_INCLUDE = "$(Join-Path $self 'include') $(Join-Path $osp 'include') $(Join-Path $iso 'include')"
. (Join-Path $harness 'lib\platform-lib.ps1')
Write-Host "vault-revisions (windows): C compilers: $((Platform-Compilers -Lang c) -join ' ')"
Platform-BuildAndRun -Lang c -Name 'vault-revisions-tests' -Sources @(
    'tests\vault_revisions_test.c',
    'src\vault_revisions.c',
    (Join-Path $osp 'src\thread_windows.c')
)
