$ErrorActionPreference='Stop'
$self=Split-Path -Parent $PSScriptRoot
$root=(Resolve-Path (Join-Path $self '..\..\..\..')).Path
$env:ISO_INCLUDE="include $(Join-Path $root 'code\packages\c\rank-statistics\include') $(Join-Path $root 'code\packages\c\bounded-json\include') $(Join-Path $root 'code\packages\c\iso-harness\include')"
. (Join-Path $root 'code\packages\c\iso-harness\lib\iso-lib.ps1')
Set-Location $self
$pbt=@('pbt_big.c','pbt_prng.c','pbt_val.c','pbt_budget.c','pbt_shrink.c','pbt_gen.c','pbt_runner.c','pbt_json.c','pbt_tfv.c','pbt_corpus.c','pbt_hooks.c' | ForEach-Object { Join-Path $root ("code\packages\c\test-bootstrap\src\" + $_) })
$env:ISO_INCLUDE += " $(Join-Path $root 'code\packages\c\test-bootstrap\include')"
$lib=@('tests\microbenchmark_test.c','src\microbenchmark.c','src\native_clock.c','src\report_json.c',(Join-Path $root 'code\packages\c\rank-statistics\src\rank_statistics.c'),(Join-Path $root 'code\packages\c\bounded-json\src\bounded_json.c')) + $pbt
Iso-BuildAndRun -Lang c -Name microbenchmark-tests -Sources $lib
if ((Get-FileHash 'testdata\microbenchmark-pairs-v1.json' -Algorithm SHA256).Hash -ne (Get-FileHash (Join-Path $root 'code\specs\fixtures\microbenchmark-pairs-v1.json') -Algorithm SHA256).Hash) { throw 'packaged fixture drifted from canonical fixture' }
& (Join-Path $PSScriptRoot 'install-test.ps1')
if ($LASTEXITCODE -ne 0) { throw 'installed consumer test failed' }
