# Build and run the image-codec-bmp tests under MSVC (cl.exe) — pure ISO C/C++ —
# via the shared iso-harness. Composes c/pixel-container; links nothing.
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
$pixel = Join-Path $d 'code\packages\c\pixel-container'

$env:ISO_INCLUDE = "include $harness\include $pixel\include"
. (Join-Path $harness 'lib\iso-lib.ps1')
Iso-BuildAndRun -Lang c -Name image_codec_bmp-tests -Sources @(
    "tests\bmp_codec_test.c",
    "src\bmp_codec.c",
    (Join-Path $pixel 'src\pixel_container.c')
)
