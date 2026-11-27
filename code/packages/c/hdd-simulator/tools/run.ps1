$ErrorActionPreference = 'Stop'
$Self = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Self
$d = $Self
while ($d -and -not (Test-Path (Join-Path $d 'code\packages\c\iso-harness'))) { $d = Split-Path -Parent $d }
if (-not $d) { throw 'iso-harness not found' }
$Harness = Join-Path $d 'code\packages\c\iso-harness'
$env:ISO_INCLUDE = "include ..\bounded-json\include ..\iso-harness\include"
. (Join-Path $Harness 'lib\iso-lib.ps1')
Iso-BuildAndRun -Lang c -Name hdd-simulator-tests -Sources @('tests\hdd_simulator_test.c', '..\bounded-json\src\bounded_json.c')
Iso-BuildAndRun -Lang c -Name hdd-simulator-public-smoke -Sources @('tests\hdd_public_smoke.c', 'src\hdd_simulator.c')
