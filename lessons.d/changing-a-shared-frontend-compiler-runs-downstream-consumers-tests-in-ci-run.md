---
category: CI & GitHub Actions
---

# Changing a shared frontend compiler runs DOWNSTREAM consumers' tests in CI — run them locally first

Editing `twig-ir-compiler`'s lowering (LANG-FULL TW2: value-defines → typed locals) passed `cargo test -p twig-ir-compiler` and the lang-aot matrix locally, but CI's affected-package detection also rebuilt `twig-vm`, whose `dispatch.rs` test compiled a real Twig program and asserted on the *exact* emitted ops (`(define x 5)` → a `global_set` writing `x` to the host global table).  The new lowering dropped that `global_set`, so the downstream test's expectation broke (the result was still correct).  This is NOT a latent bug to defer — it's a test that legitimately tracks the compiler output you changed, so update it in the same PR.  Rule: when a PR touches a shared `*-ir-compiler` (or any crate many others depend on), enumerate consumers with `grep -rln '<crate>' */Cargo.toml` and run **each** consumer's tests (`cargo test -p <consumer>`) before pushing — not just the changed crate + the integration matrix.  Preserve a downstream test's *intent* when updating it (here: keep exercising `global_set` by switching to a lambda-**captured** define, which still hits the global table) rather than deleting the assertion.
