---
summary: "Coverage instrumentation can add optimized-IR cleanup paths, so secret-wipe codegen gates should assert required lower bounds instead of one uninstrumented call count."
tags: [rust, coverage, llvm-ir, zeroize, testing]
---

# Coverage instrumentation can add optimized-IR cleanup paths

An optimized-code regression test that counted exactly two secret-owner wipe
calls passed normally but failed when the same test ran beneath tarpaulin.
Coverage counters preserved an otherwise folded error branch, so the generated
function contained a third valid wipe path. The secret was still scrubbed on
every exit; the test had confused one compiler configuration's control-flow
shape with the security property.

For exported fixed-memory boundaries, assert that required owners have at
least the success and unwind wipes, then separately verify the relevant exit
paths and ordering. Keep exact counts for small dedicated wipe helpers whose
control flow is itself the audited invariant. Always run codegen gates both
normally and under the package's coverage command before freezing a patch.
