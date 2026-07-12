# Build and run the bignum-core tests under MSVC (cl.exe) — pure ISO C/C++ — via the
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
Iso-BuildAndRun -Lang c -Name bignum_core-tests -Sources @("tests\bignum_core_test.c", "src\bignum_core.c")
Iso-BuildAndRun -Lang c -Name bignum_decimal-tests -Sources @("tests\bignum_decimal_test.c", "src\bignum_decimal.c", "src\bignum_core.c")
Iso-BuildAndRun -Lang c -Name bignum_rational-tests -Sources @("tests\bignum_rational_test.c", "src\bignum_rational.c", "src\bignum_decimal.c", "src\bignum_core.c")
