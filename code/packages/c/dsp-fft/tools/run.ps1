# Build and run the dsp-fft tests under MSVC (cl.exe) — pure ISO C17 — via the
# shared iso-harness (code\packages\c\iso-harness).
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

$env:ISO_INCLUDE = "include ..\dsp-complex\include $harness\include"
. (Join-Path $harness 'lib\iso-lib.ps1')
Iso-BuildAndRun -Lang c -Name dsp_fft-tests -Sources @("tests\dsp_fft_test.c", "src\dsp_fft.c", "..\dsp-complex\src\dsp_complex.c")
