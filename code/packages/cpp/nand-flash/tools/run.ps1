Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self
$d = $self
while ($d -and -not (Test-Path (Join-Path $d 'code\packages\c\iso-harness'))) { $d = Split-Path -Parent $d }
if (-not $d) { throw 'iso-harness not found' }
$harness = Join-Path $d 'code\packages\c\iso-harness'
$env:ISO_INCLUDE = "include src ..\bounded-json\include ..\..\c\iso-harness\include"
. (Join-Path $harness 'lib\iso-lib.ps1')
Iso-BuildAndRun -Lang cpp -Name nand-flash-tests -Sources @('tests\nand_flash_test.cpp', 'src\nand_flash.cpp')
