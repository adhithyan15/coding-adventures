$ErrorActionPreference='Stop'
$self=Split-Path -Parent $PSScriptRoot
$root=(Resolve-Path (Join-Path $self '..\..\..\..')).Path
$env:ISO_INCLUDE="include $(Join-Path $root 'code\packages\c\iso-harness\include')"
. (Join-Path $root 'code\packages\c\iso-harness\lib\iso-lib.ps1')
Set-Location $self
Iso-BuildAndRun -Lang c -Name rank-statistics -Sources @('tests\rank_test.c','src\rank_statistics.c')
