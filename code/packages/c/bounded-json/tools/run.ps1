Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self
$harness = Join-Path $self '..\iso-harness'
$env:ISO_INCLUDE = "include $harness\include"
# Windows PowerShell 5 otherwise guesses ANSI for a UTF-8 file without a BOM.
# Decode the fixed shared source explicitly; keep its compiler scheduler intact.
$source = [IO.File]::ReadAllText((Join-Path $harness 'lib\iso-lib.ps1'), [Text.Encoding]::UTF8)
. ([scriptblock]::Create($source))
Iso-BuildAndRun -Lang c -Name bounded-json-tests -Sources @('tests\json_test.c', 'src\bounded_json.c')
