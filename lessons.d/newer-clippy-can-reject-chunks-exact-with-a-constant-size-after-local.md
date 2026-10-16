---
category: Rust
---

# Newer Clippy can reject chunks_exact with a constant size after local Clippy passes

On PR #16885, local Clippy 1.97 passed a `chunks_exact(2)` loop, but macOS CI
used Clippy 1.99 and failed under `-D warnings` with
`clippy::chunks_exact_to_as_chunks`. For a compile-time pair size, use
`slice.as_chunks::<2>()`, iterate the returned fixed-size pairs, and inspect
the returned remainder. Check the Rust versions used by protected platforms
when a newer lint appears only in CI; keep the remainder check so malformed
grammar trees still fail explicitly.


The checked Ishikawa9ff implementation repeated this known issue: actual49
library tests and2 liveness tests passed, but all-target strict failed on
scratch.chunks_exact(2). Preserve that receipt and replace only the iteration
with scratch.as_chunks::<2>().0; no lint allowance or gate weakening. Rerun the
real strict command and affected tests before claiming acceptance.
