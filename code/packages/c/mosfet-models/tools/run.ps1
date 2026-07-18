# Build and run the mosfet-models tests under MSVC (cl.exe) — pure ISO C/C++ — via
# the shared iso-harness. Composes c/device-physics + c/float-math; links nothing.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self

$d = $self
while ($d -and -not (Test-Path (Join-Path $d 'code\packages\c\iso-harness'))) {
    $d = Split-Path -Parent $d
}
if (-not $d) { throw 'iso-harness not found (searched upward from ' + $self + ')' }
$harness = Join-Path $d 'code\packages\c\iso-harness'
$devphys = Join-Path $d 'code\packages\c\device-physics'
$fmath = Join-Path $d 'code\packages\c\float-math'

$env:ISO_INCLUDE = "include $harness\include $devphys\include $fmath\include"
. (Join-Path $harness 'lib\iso-lib.ps1')
Iso-BuildAndRun -Lang c -Name mosfet_models-tests -Sources @(
    "tests\mosfet_models_test.c",
    "src\mosfet_models.c",
    (Join-Path $devphys 'src\device_physics.c'),
    (Join-Path $fmath 'src\float_math.c')
)
