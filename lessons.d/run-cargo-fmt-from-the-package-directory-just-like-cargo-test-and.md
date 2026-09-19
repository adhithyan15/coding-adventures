---
category: Rust
---

# Run cargo fmt from the package directory just like cargo test and cargo build

This repository has no root `Cargo.toml`, so even a read-only
`cargo fmt --check -p ...` fails when launched from the checkout root. Run
format, test, lint, and documentation commands from the Rust package directory
or pass that package's explicit manifest path.
