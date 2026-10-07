---
category: Rust
---

# A newly activated native gate can expose strict lint missed by the local toolchain

The first correctly gated Windows CV02 run failed in the build tool's strict
Clippy preflight before any compiler tests ran. CI used Rust 1.99 and rejected
constant-size `chunks_exact(4)` in the SID decoder, although local Rust 1.97
validation had passed. The earlier green Windows job had skipped this package
step and could not have demonstrated native acceptance.

Reproduce the actual failing command with the CI toolchain, keep warnings denied,
and use fixed four-byte array chunks after the existing bounded SID checks.
SID lengths are `8 + 4 * count`, so word order and remainder behavior are
preserved. Rerun native policy/compiler tests and exact-head review, then verify
the repaired compiler command really executes on fresh native CI.
