---
category: Testing & coverage
---

# Runtime backend identities come from completed cell logs rather than earlier availability

The checkpoint correctly counted forty-eight normally completed backend cells
but incorrectly named Wasm as unavailable. Independent documentation review
found eight successful Wasm executions and eight unavailable CLR records.
The syntax checks passed; semantic documentation acceptance was withheld.

Derive the backend roster from the exact completed receipt, not an earlier
run's availability or an assumption about six backends. For this source the
actual executed roster is NativeAot, Llvm, Wasm, Jvm, Vm and Jit, eight cells
each; Clr is unavailable for all eight sources. Preserve the incorrect-head
receipt/review and correct the backlog from those actual records. Passing cell
counts never substitute for accurate backend identities or whole acceptance.
