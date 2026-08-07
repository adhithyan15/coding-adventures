<#
    run.ps1 — build & run the tcp-runtime test on Windows. tcp-runtime is
    OS-agnostic; it is compiled with net's and reactor's Winsock backends, which
    hold the per-OS code. Links ws2_32 (Winsock, used by both).
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
$net = Join-Path $d 'code\packages\c\net'
$reactor = Join-Path $d 'code\packages\c\reactor'
if (-not (Test-Path (Join-Path $harness 'lib\platform-lib.ps1'))) {
    throw "platform-harness not found (searched upward from $self)"
}
$env:PLATFORM_INCLUDE = "$(Join-Path $self 'include') $(Join-Path $net 'include') $(Join-Path $reactor 'include') $(Join-Path $osp 'include') $(Join-Path $iso 'include')"
$env:PLATFORM_LIBS = 'ws2_32.lib'
. (Join-Path $harness 'lib\platform-lib.ps1')
Write-Host "tcp-runtime (windows): C compilers: $((Platform-Compilers -Lang c) -join ' ')"
# The mailbox mutex comes from os-platform's thread backend (Win32); its source
# needs only the CRT + kernel32, already linked by default.
Platform-BuildAndRun -Lang c -Name 'tcp-runtime-tests' -Sources @(
    'tests\tcp_runtime_test.c',
    'src\tcp_runtime.c',
    (Join-Path $net 'src\net_windows.c'),
    (Join-Path $reactor 'src\reactor_windows.c'),
    (Join-Path $osp 'src\thread_windows.c')
)
