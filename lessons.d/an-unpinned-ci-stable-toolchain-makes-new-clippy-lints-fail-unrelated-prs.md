---
category: Rust
---

# An unpinned CI stable toolchain makes new clippy lints fail unrelated PRs: fix promptly with behaviour-preserving rewrites

CI installs the floating `stable` Rust toolchain and runs
`cargo clippy --all-targets -- -D warnings`. When stable moved to 1.100.0,
`needless_range_loop` began firing on `for i in 0..N { a[i] = f(i) }` loops
that had been clean for months, and an `if cond { return false; } true` guard
was flagged as a bool-literal guard. The affected crates (simulators, a
compiler, an interrupt handler, `logic-gates`) had not been touched, so the
failures landed on whichever unrelated PR happened to rebuild them, and
dependents such as `x86-simulator` failed because a dependency failed first.

What to do:

- Reproduce on the new toolchain (`rustup toolchain install <ver> --profile
  minimal --component clippy`, then `cargo +<ver> clippy -p <crate>
  --all-targets -- -D warnings`) and fix every hit in one pass. Check the old
  toolchain too, so the rewrite stays clean on both sides.
- Prefer iterator rewrites: `slice.iter_mut().enumerate()` (add `.take(n)` or
  `.skip(k)` to keep the exact range), `for &x in &v`, and a single boolean
  expression for the guard (De Morgan: `!(a || !b)` is `!a && b`). These do
  not change behaviour. Do not reach for `#[allow]` unless the index loop is
  genuinely clearer, and then explain why in a comment.
- A crate that fails to compile under clippy hides lints in its dependents, so
  re-run after each fix.
