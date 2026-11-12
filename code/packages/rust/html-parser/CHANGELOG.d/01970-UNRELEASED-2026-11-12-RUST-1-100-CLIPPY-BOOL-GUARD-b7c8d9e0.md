- **Rust 1.100 clippy: the autofill-token field predicate is one boolean
  expression** (in `src/lib.rs` and the matching test oracle in
  `tests/browser_readiness_test.rs`), replacing an `if` guard that returned a
  bool literal; behaviour is unchanged.
