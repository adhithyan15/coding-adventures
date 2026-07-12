# Build and run the scrypt tests under MSVC (cl.exe) — pure ISO C/C++ — via the
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

$env:ISO_INCLUDE = "include ..\pbkdf2\include ..\hmac\include ..\sha1\include ..\sha256\include ..\sha512\include $harness\include"
. (Join-Path $harness 'lib\iso-lib.ps1')
Iso-BuildAndRun -Lang c -Name scrypt-tests -Sources @("tests\scrypt_test.c", "src\scrypt.c", "..\pbkdf2\src\pbkdf2.c", "..\hmac\src\hmac.c", "..\sha1\src\sha1.c", "..\sha256\src\sha256.c", "..\sha512\src\sha512.c")
