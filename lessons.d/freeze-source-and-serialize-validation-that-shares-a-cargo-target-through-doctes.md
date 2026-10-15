---
category: Testing & coverage
---

# Freeze source and serialize validation that shares a Cargo target through doctests

During CV04a, a broad 181-package Rust test run reached execution after its
compilation phase. A targeted rebuild of the modified shared lexer then used the
same workspace target directory. Cargo's build lock did not protect the entire
remaining execution/doc-test phase: newer rlibs replaced dependencies expected
by the original rustdoc commands. The broad run reported E0460 (possibly newer
lexer/parser crate) and E0463 (cannot find dependent crate), rather than a source
range regression.

The initial broad run is not accepted as final validation. Freeze the complete
candidate, let the active shared-target run end, then rerun the full original
batch sequentially through its doctests and strict lint. Do not launch a focused
rebuild, reviewer Cargo test or source edit against that same target mid-run.
Independent targets may run concurrently, and Miri uses its own target tree.
Preserve failed logs and final source-bound receipts; never subtract the failed
doctests from a green claim or alter their assertions.
