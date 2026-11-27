# Build and run the heap tests under MSVC (cl.exe) — pure ISO C/C++ — via the
# shared iso-harness (code\packages\c\iso-harness).
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self

# Locate the iso-harness by walking up to the repo dir that contains it.
$d = $self
while ($d -and -not (Test-Path (Join-Path $d 'code\packages\c\iso-harness'))) {
    $d = Split-Path -Parent $d
}
if (-not $d) { throw 'iso-harness not found (searched upward from ' + $self + ')' }
$harness = Join-Path $d 'code\packages\c\iso-harness'

$env:ISO_INCLUDE = "include $harness\include"
. (Join-Path $harness 'lib\iso-lib.ps1')
Iso-BuildAndRun -Lang cpp -Name heap-tests -Sources @("tests\heap_test.cpp")
Iso-ExpectCompileFail -Lang cpp -Source 'tests\member_pointer_comparator_rejected.cpp'
Iso-ExpectCompileFail -Lang cpp -Source 'tests\throwing_comparator_destructor_rejected.cpp'
& (Join-Path $self 'tools\installed-smoke.ps1')
if ($LASTEXITCODE -ne 0) {
    throw "Installed CMake consumer failed with exit code $LASTEXITCODE"
}
