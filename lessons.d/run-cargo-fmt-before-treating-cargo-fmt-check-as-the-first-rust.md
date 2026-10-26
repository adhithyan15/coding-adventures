---
category: Rust
---

# Run cargo fmt before treating cargo fmt check as the first Rust implementation gate

The first focused gate used `cargo fmt --check` immediately after replacing a
generated Rust placeholder, so a harmless line-wrap difference stopped the
combined command before tests, clippy, or rustdoc ran. Run package-scoped
`cargo fmt` after the implementation edit, inspect its diff, and then use
`cargo fmt --check` in the repeatable validation sequence.
