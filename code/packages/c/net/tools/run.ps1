<#
    run.ps1 — build & run the net tests on Windows (MSVC family) via
    platform-harness. Driven by BUILD_windows.

    Winsock lives in ws2_32.lib, linked explicitly via PLATFORM_LIBS. The Unix
    half is run.sh.
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

# Our headers, os-platform's (for os_platform/status.h), and iso_test.h.
$env:PLATFORM_INCLUDE = "$(Join-Path $self 'include') $(Join-Path $osp 'include') $(Join-Path $iso 'include')"
# Winsock import library.
$env:PLATFORM_LIBS = 'ws2_32.lib'

. (Join-Path $harness 'lib\platform-lib.ps1')

Write-Host "net (windows): C compilers: $((Platform-Compilers -Lang c) -join ' ')"

# tcp — Winsock backend.
Platform-BuildAndRun -Lang c -Name 'net-tests' -Sources @('tests\net_test.c', 'src\net_windows.c')
