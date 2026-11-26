---
category: Rust
---

# A closure that is only called, never reassigned, must not be bound with let mut; CI builds Rust with -D warnings so a local build without it hides the failure

PR #19894 (warrant-ir KRN-006b) failed CI at `check.rs:300` with "variable does not need to be mutable" on `let mut check_reads = |...| {...}`. The closure was only ever called, never reassigned, and it captured nothing it mutates, so `mut` was unneeded. A plain local `cargo build` only prints that as a warning; CI builds with `-D warnings`, which turns it into a hard error, and the dependent crates then failed too because warrant-ir did not compile.

Fix: drop the `mut`. Bind a closure with `let mut` only when calling it requires `FnMut` (it mutates captured state) or you reassign the binding.

Takeaway: before handing Rust work back, build with `RUSTFLAGS="-D warnings" cargo build --all-targets` and `RUSTFLAGS="-D warnings" cargo test` from inside the crate, and check the exit status rather than grepping the output.
