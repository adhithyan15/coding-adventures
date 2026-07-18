# Build and run the hyperloglog tests under MSVC (cl.exe) — pure ISO C/C++ — via
# the shared iso-harness. Composes c/hash-functions (hf_fnv1a_64) + c/float-math
# (fm_*) by compiling their sources in; links nothing (no math library).
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self

# Locate the repo root (the dir containing code\packages\c\iso-harness).
$d = $self
while ($d -and -not (Test-Path (Join-Path $d 'code\packages\c\iso-harness'))) {
    $d = Split-Path -Parent $d
}
if (-not $d) { throw 'iso-harness not found (searched upward from ' + $self + ')' }
$harness = Join-Path $d 'code\packages\c\iso-harness'
$hash = Join-Path $d 'code\packages\c\hash-functions'
$fmath = Join-Path $d 'code\packages\c\float-math'

$env:ISO_INCLUDE = "include $harness\include $hash\include $fmath\include"
. (Join-Path $harness 'lib\iso-lib.ps1')
Iso-BuildAndRun -Lang c -Name hyperloglog-tests -Sources @(
    "tests\hyperloglog_test.c",
    "src\hyperloglog.c",
    (Join-Path $hash 'src\hash_functions.c'),
    (Join-Path $fmath 'src\float_math.c')
)
