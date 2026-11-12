- **Clippy 1.100 clean**: the autofill-token predicate in `src/lib.rs` and its
  mirror in `tests/browser_readiness_test.rs` return a single negated
  expression instead of an `if`-returns-`false`-then-`true` pair (clippy
  `needless_bool`). Behaviour is unchanged.
