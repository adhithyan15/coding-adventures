---
category: CI & GitHub Actions
---

# Getting a large workspace to zero clippy warnings: `cargo clippy --fix` first, but it stops at the first deny-by-default hard error

`absurd_extreme_comparisons`/`approx_constant`/`not_unsafe_ptr_arg_deref`/`never_loop` are deny-by-default, and a deny error aborts that crate's compile so `--fix` can't touch its other (machine-applicable) warnings. Clear/allow the hard errors first, then re-run `--fix`; crates that previously failed to compile now get auto-fixed. `--fix` only applies `MachineApplicable` suggestions — `approx_constant` (replacing `3.14159` with `PI` changes the value) is `MaybeIncorrect`, so it is NEVER auto-fixed; resolve those with a scoped `#[allow(clippy::approx_constant)]` + justification, never by editing the literal (it's usually test data / codegen input / an intentional hand-written constant). FFI crates that expose raw-pointer C ABIs (`node-bridge`, `ruby-bridge`) get a crate-level `#![allow(clippy::not_unsafe_ptr_arg_deref)]` with a comment rather than ~80 per-fn annotations.
