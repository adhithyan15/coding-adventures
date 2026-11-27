$ErrorActionPreference = 'Stop'
$Self = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Self
$d = $Self
while ($d -and -not (Test-Path (Join-Path $d 'code\packages\c\iso-harness'))) { $d = Split-Path -Parent $d }
if (-not $d) { throw 'iso-harness not found' }
$Harness = Join-Path $d 'code\packages\c\iso-harness'
$env:ISO_INCLUDE = "include ..\bounded-json\include ..\iso-harness\include"
. (Join-Path $Harness 'lib\iso-lib.ps1')
Iso-BuildAndRun -Lang c -Name nand-flash-tests -Sources @('tests\nand_flash_test.c', 'tests\nand_flash_fuzz_support.c', 'src\nand_flash.c', '..\bounded-json\src\bounded_json.c')
