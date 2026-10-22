---
category: Rust
---

# The Rust package tree has no root Cargo workspace, so run Cargo in each package directory

Running `cargo test -p ...` from `code/` failed because the repository keeps
independent `Cargo.toml` files under each Rust package and has no root Cargo
workspace there. For package-scoped work, run `cargo test`, Clippy, rustdoc,
and formatting from each affected package directory; use that package's
`BUILD` script for its declared transitive validation instead of assuming a
shared workspace manifest.
