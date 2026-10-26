Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self
$harness = Join-Path $self '..\iso-harness'
$env:ISO_INCLUDE = "include ../matrix/include ../float-math/include ../bounded-json/include $harness\include"
# The existing shared harness owns compiler discovery and strict flag policy.
# Decode its trusted UTF-8 source explicitly for Windows PowerShell 5.
$source = [IO.File]::ReadAllText((Join-Path $harness 'lib\iso-lib.ps1'), [Text.Encoding]::UTF8)
. ([scriptblock]::Create($source))
Iso-BuildAndRun -Lang c -Name neural-framework-tests -Sources @(
    'tests\framework_test.c', 'src\neural_framework.c',
    '..\matrix\src\matrix.c', '..\float-math\src\float_math.c',
    '..\bounded-json\src\bounded_json.c')
