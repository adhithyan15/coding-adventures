<#
    run.ps1 — build & run the reactor tests on Windows (MSVC) via platform-harness.
    WSAPoll lives in ws2_32.lib; the loopback pair in the test uses Winsock too.
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
$env:PLATFORM_LIBS = 'ws2_32.lib'
. (Join-Path $harness 'lib\platform-lib.ps1')
Write-Host "reactor (windows): C compilers: $((Platform-Compilers -Lang c) -join ' ')"
Platform-BuildAndRun -Lang c -Name 'reactor-tests' -Sources @('tests\reactor_test.c', 'src\reactor_windows.c')
