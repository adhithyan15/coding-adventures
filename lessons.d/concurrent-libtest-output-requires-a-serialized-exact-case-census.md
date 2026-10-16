---
category: Testing & coverage
---

# Concurrent libtest output requires a serialized exact-case census

At exact09fb the ALGOL unit suite actually passed965/0/0, but the merged
nocapture stdout/stderr stream split four post-assertion marker IDs across
libtest progress lines. The fail-closed helper correctly rejected the344/347
and125/126 census and did not run strict. Preserve that failed receipt/log;
do not infer missing IDs from source or silently relax the counter.

A new helper/receipt/log with --test-threads=1 retained every965/347/126,
source/lock/helper and executable-join assertion. The actual serialized run
passed the complete census and package strict; producer81099 was fully consumed0.

A later three-test expected-red integration wrapper also assumed the test name
and FAILED appeared on one line under nocapture. The actual compiled result was
Cargo101,2P1F and the failure list named the intended22-unit old oracle, but the
wrapper assertion failed because panic output separated the progress fragments.
Preserve the original wrapper failure; read the structured terminal summary and
explicit failure-list/panic attribution in a separate reconciliation. Never
match an arbitrarily interleaved single progress line as terminal evidence.
