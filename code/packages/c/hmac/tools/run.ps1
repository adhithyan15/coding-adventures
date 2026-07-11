# Build and run the hmac tests under MSVC (cl.exe) — pure ISO C/C++ — via the
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

$env:ISO_INCLUDE = "include ..\sha256\include $harness\include"
. (Join-Path $harness 'lib\iso-lib.ps1')
# The test compiles the sibling sha256 source in to supply the hash primitive.
Iso-BuildAndRun -Lang c -Name hmac-tests -Sources @("tests\hmac_test.c", "src\hmac.c", "..\sha256\src\sha256.c")
