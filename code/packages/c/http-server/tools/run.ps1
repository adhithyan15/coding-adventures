<#
    run.ps1 — build & run the http-server test on Windows. Compiled with
    tcp-runtime, net's and reactor's Winsock backends, and http-core.
    Links ws2_32 (Winsock).
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
$tcprt = Join-Path $d 'code\packages\c\tcp-runtime'
$http = Join-Path $d 'code\packages\c\http-core'
if (-not (Test-Path (Join-Path $harness 'lib\platform-lib.ps1'))) {
    throw "platform-harness not found (searched upward from $self)"
}
$env:PLATFORM_INCLUDE = "$(Join-Path $self 'include') $(Join-Path $tcprt 'include') $(Join-Path $net 'include') $(Join-Path $reactor 'include') $(Join-Path $http 'include') $(Join-Path $osp 'include') $(Join-Path $iso 'include')"
$env:PLATFORM_LIBS = 'ws2_32.lib'
. (Join-Path $harness 'lib\platform-lib.ps1')
Write-Host "http-server (windows): C compilers: $((Platform-Compilers -Lang c) -join ' ')"
# tcp-runtime's mailbox uses os-platform's thread mutex (Win32 backend); its
# source needs only the CRT + kernel32, linked by default.
Platform-BuildAndRun -Lang c -Name 'http-server-tests' -Sources @(
    'tests\http_server_test.c',
    'src\http_server.c',
    (Join-Path $tcprt 'src\tcp_runtime.c'),
    (Join-Path $net 'src\net_windows.c'),
    (Join-Path $reactor 'src\reactor_windows.c'),
    (Join-Path $http 'src\http_core.c'),
    (Join-Path $osp 'src\thread_windows.c')
)
