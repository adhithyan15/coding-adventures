---
category: Rust
---

# Marking a verdict enum must_use turns a computed-then-dropped check into a compile error

The L0 harness (`code/packages/rust/test-bootstrap`) runs a test body that
returns a `Verdict`. Nothing stopped a body from computing the check and then
returning `Pass` anyway:

```rust
suite.add("t", |ctx| {
    ctx.deep_equal(&a, &b); // verdict dropped
    Verdict::Pass
});
```

It compiled without a warning and reported a pass whatever `a` and `b` were,
the cheapest false pass in the whole stack.

`#[must_use]` on the `Verdict` type fixes it: every function returning a
`Verdict` inherits the attribute, `clippy -D warnings` (the BUILD gate) turns
the dropped value into an error, and a `compile_fail` doctest under
`#![deny(unused_must_use)]` proves it stays that way. Adding the attribute to
`Verdict` produced no new warning in `ct-compare`, `warrant-grammar` or
`warrant-ir`, so it cost nothing downstream.

What to do differently:

- Put `#[must_use]` on the *type* that carries a pass/fail outcome, not on
  individual functions, so new constructors and adapters are covered.
- Pair it with a `compile_fail` doctest; an attribute nobody tests is easy to
  delete in a refactor.
- It does not catch a body that never calls a check at all. That case is
  specified behaviour at this layer (TF01 §8.3) and needs a supervisor-level
  check, not a type.
