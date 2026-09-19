---
category: Rust
---

# Binary-only Rust crates require cargo test --bin, not --lib, for embedded unit tests

`closurec` keeps its unit tests in `src/main.rs` and declares no library
target. Running `cargo test --lib <filter>` therefore fails with `no library
targets found` even though the tests exist. Use `cargo test --bin closurec
<filter>` for binary-embedded unit tests; use `--test <name>` for integration
targets. Check `Cargo.toml` or `cargo metadata` before choosing the target flag
instead of assuming every Rust package exposes a library.
