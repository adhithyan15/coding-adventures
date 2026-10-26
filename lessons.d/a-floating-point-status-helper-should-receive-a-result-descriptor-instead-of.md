---
category: Rust
---

# A floating-point status helper should receive a result descriptor instead of eight scalar arguments

The Cortex-M4 binary32 completion fix initially grew `finish_float` from seven
arguments to eight when exact inexactness and exact-nonzero status replaced a
lossy `f64` estimate. Package tests and rustdoc passed, but strict all-target
Clippy rejected the helper under `clippy::too_many_arguments`.

Group the related IEEE-754 decisions into a small internal result descriptor.
That keeps the call site explicit about invalid, divide-by-zero, overflow,
underflow eligibility, and inexact status while leaving the commit helper with
only state, rounded result, and descriptor arguments. Run strict Clippy after
changing helper signatures even when release tests are already green.
