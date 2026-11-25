# Build and run the C++ test-bootstrap tests under MSVC (cl.exe) via the shared
# iso-harness.  The C engine (c\test-bootstrap\src) is compiled as C and the tests
# as C++ in one cl invocation (cl chooses the language by file extension).  The
# ASan+UBSan pass of run.sh is POSIX only.  NOTE: authored without a Windows host;
# unverified locally.
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
$ceng = Join-Path $d 'code\packages\c\test-bootstrap'

$env:ISO_INCLUDE = "include tests $ceng\include $ceng\src $harness\include"
. (Join-Path $harness 'lib\iso-lib.ps1')

$lib = @('pbt_big', 'pbt_prng', 'pbt_val', 'pbt_budget', 'pbt_shrink', 'pbt_gen', 'pbt_runner', 'pbt_json', 'pbt_tfv', 'pbt_hooks') |
    ForEach-Object { Join-Path $ceng "src\$_.c" }
$tests = @('test_api', 'test_exceptions', 'test_alloc_throw', 'test_corpus')
foreach ($t in $tests) {
    Write-Host "== $t"
    Iso-BuildAndRun -Lang cpp -Name "pbt-cpp-$t" -Sources (@("tests\$t.cpp") + $lib)
}
