<#
    run.ps1 — build & run the event-loop test on Windows. event-loop is
    OS-agnostic; it composes os-platform's thread backend (osp_mutex) and clock
    backend (osp_sleep_ns), whose Win32 sources need only the CRT + kernel32
    (linked by default). The test spawns a worker thread via os-platform's thread.
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
Write-Host "event-loop (windows): C compilers: $((Platform-Compilers -Lang c) -join ' ')"
Platform-BuildAndRun -Lang c -Name 'event-loop-tests' -Sources @(
    'tests\event_loop_test.c',
    'src\event_loop.c',
    (Join-Path $osp 'src\thread_windows.c'),
    (Join-Path $osp 'src\clock_windows.c')
)
