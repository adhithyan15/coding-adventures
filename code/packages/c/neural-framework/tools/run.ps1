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
Iso-BuildAndRun -Lang c -Name native-xor -Sources @(
    'examples\xor.c', 'src\neural_framework.c',
    '..\matrix\src\matrix.c', '..\float-math\src\float_math.c',
    '..\bounded-json\src\bounded_json.c')
# Inspect the existing harness's discovery result; no parallel compiler policy.
$compilers = @(Iso-Compilers -Lang c)
foreach ($compiler in $compilers) {
    $executable = [IO.Path]::GetFullPath((Join-Path (Iso-BuildDir) "$compiler-native-xor.exe"))
    & python tests/command_test.py $executable
    if ($LASTEXITCODE -ne 0) { throw 'Native C command boundary validation failed' }
    & python ../../../scripts/validate_neural_framework_c.py --executable $executable
    if ($LASTEXITCODE -ne 0) { throw 'Native C full checkpoint resume validation failed' }
}
& python ../../../scripts/tests/test_neural_framework_c.py
if ($LASTEXITCODE -ne 0) { throw 'Native C validator regressions failed' }
