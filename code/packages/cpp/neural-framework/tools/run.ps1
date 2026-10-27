Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self
$harness = Join-Path $self '../../c/iso-harness'
$env:ISO_INCLUDE = "include ../matrix/include ../float-math/include ../bounded-json/include $harness/include"
. (Join-Path $harness 'lib/iso-lib.ps1')
Iso-BuildAndRun -Lang cpp -Name neural-framework-tests -Sources @('tests/network_test.cpp', 'tests/allocation.cpp', 'tests/allocation_hooks.cpp')
Iso-BuildAndRun -Lang cpp -Name native-xor -Sources @('examples/xor.cpp') | Out-File (Join-Path (Iso-BuildDir) 'native-xor-harness.log') -Encoding utf8
$python = if ($env:PYTHON) { $env:PYTHON } else { 'python' }
foreach ($compiler in (Iso-Compilers -Lang cpp)) {
    $executable = Join-Path $self "$(Iso-BuildDir)/$compiler-native-xor.exe"
    & $python tests/command_test.py $executable
    if ($LASTEXITCODE -ne 0) { throw "Native Cpp command tests failed: $compiler" }
}
