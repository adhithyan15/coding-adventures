# Build and run the trig tests under MSVC (cl.exe) — pure ISO C/C++ — via the
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
Iso-BuildAndRun -Lang cpp -Name trig-tests -Sources @("tests\trig_test.cpp")
# TRIG-REF-1 bit-exact conformance (PHY00a 6): every row of the shared vectors in
# code\specs\fixtures\trig-reference-v1 must match to the last bit. The test finds
# the corpus relative to this package directory (forward slashes open fine on
# Windows). The fusion canary of run.sh stays POSIX-only.
Iso-BuildAndRun -Lang cpp -Name trig-conformance -Sources @("tests\conformance_test.cpp")
