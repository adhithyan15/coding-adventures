# Build and run the http1 tests under MSVC (cl.exe) — pure ISO C/C++ — via the
# shared iso-harness. Composes c/http-core; links nothing.
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
$httpcore = Join-Path $d 'code\packages\c\http-core'

$env:ISO_INCLUDE = "include $harness\include $httpcore\include"
. (Join-Path $harness 'lib\iso-lib.ps1')
Iso-BuildAndRun -Lang c -Name http1-tests -Sources @(
    "tests\http1_test.c",
    "src\http1.c",
    (Join-Path $httpcore 'src\http_core.c')
)
