# Build the HWEB00 API with the shared strict ISO C17 compiler harness.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$self = Split-Path -Parent $PSScriptRoot
Set-Location $self

$boot = Join-Path $self '..\test-bootstrap'
$json = Join-Path $self '..\bounded-json'
$harness = Join-Path $self '..\iso-harness'
$env:ISO_INCLUDE = "include $boot\include $boot\src $boot\tests $json\include $harness\include"
# Windows PowerShell 5 can decode a UTF-8 script incorrectly without a BOM.
$source = [IO.File]::ReadAllText((Join-Path $harness 'lib\iso-lib.ps1'), [Text.Encoding]::UTF8)
. ([scriptblock]::Create($source))

$bootstrap = @(
    "$boot\src\pbt_big.c", "$boot\src\pbt_prng.c", "$boot\src\pbt_val.c",
    "$boot\src\pbt_budget.c", "$boot\src\pbt_shrink.c", "$boot\src\pbt_gen.c",
    "$boot\src\pbt_runner.c", "$boot\src\pbt_json.c", "$boot\src\pbt_tfv.c",
    "$boot\src\pbt_corpus.c", "$boot\src\pbt_hooks.c", "$boot\src\fuzz_hooks.c",
    "$boot\src\fuzz_mutate.c", "$boot\src\fuzz_bitmap.c", "$boot\src\fuzz_sig.c",
    "$boot\src\fuzz_engine.c", "$boot\src\fuzz_corpus.c"
)
$sources = @(
    'tests\historical_web_test.c', "$boot\tests\test_util.c",
    'src\html_tags_1992.c', 'src\http09.c', "$json\src\bounded_json.c"
) + $bootstrap
Iso-BuildAndRun -Lang c -Name historical-web-tests -Sources $sources
