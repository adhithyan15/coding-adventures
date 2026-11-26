# Build and run the test-bootstrap tests under MSVC (cl.exe) - pure ISO C - via the
# shared iso-harness (code\packages\c\iso-harness).  The ASan+UBSan pass of run.sh is
# POSIX only: the harness has no sanitizer mode on Windows.
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

$env:ISO_INCLUDE = "include src tests $harness\include"
. (Join-Path $harness 'lib\iso-lib.ps1')

$lib = @(
    'src\pbt_big.c', 'src\pbt_prng.c', 'src\pbt_val.c', 'src\pbt_budget.c', 'src\pbt_shrink.c',
    'src\pbt_gen.c', 'src\pbt_runner.c', 'src\pbt_json.c', 'src\pbt_tfv.c', 'src\pbt_corpus.c',
    'src\pbt_hooks.c', 'src\fuzz_hooks.c', 'src\fuzz_mutate.c', 'src\fuzz_bitmap.c', 'src\fuzz_sig.c',
    'src\fuzz_engine.c', 'src\fuzz_corpus.c'
)
$tests = @('test_big', 'test_prng', 'test_json_tfv', 'test_val', 'test_budget', 'test_engine', 'test_oom', 'test_corpus',
    'test_fuzz_corpus', 'test_fuzz_vectors', 'test_fuzz_pinned', 'test_fuzz_engine', 'test_fuzz_oom')
foreach ($t in $tests) {
    Write-Host "== $t"
    Iso-BuildAndRun -Lang c -Name "pbt-$t" -Sources (@("tests\$t.c", 'tests\test_util.c') + $lib)
}
