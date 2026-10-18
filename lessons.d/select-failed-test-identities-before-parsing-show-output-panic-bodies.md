---
category: Testing & coverage
---

# Select failed test identities before parsing show-output panic bodies

The independent Venn work-red reviewer first used a parser that treated a
passing --show-output section as part of the failed panic set. That review
helper exited1; the Rust producer and its raw receipt were unchanged.

Preserve the failed review invocation. Select actual FAILED identities from
the outer harness status vector first, then associate only those identities
with their saved panic bodies. The successor review completed0 and proved
the five failure-layout pairs unchanged, while separately retaining the
passing work output and all twelve completed work cells. Counts alone do
not establish a failure cause. This correction grants no production or
full-cone acceptance. Miri remains DEFERRED/PENDING and untouched.
