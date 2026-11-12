## Unreleased - 2026-11-12 - Rust 1.100 clippy

### Changed

- Simplify the autofill-token field predicate (in `src/lib.rs` and the matching
  test oracle in `tests/browser_readiness_test.rs`) to one boolean expression
  (Rust 1.100 clippy bool-literal guard lint); behaviour is unchanged.
