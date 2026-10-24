---
category: Rust
---

# Run Cargo tools from the Rust workspace that owns the manifest

Running `cargo tarpaulin -p ...` from the repository root failed because this
monorepo's Rust workspace manifest lives at `code/packages/rust/Cargo.toml`.
The package flag does not help Cargo discover a manifest above the current
directory. Run Cargo subcommands from `code/packages/rust`, or pass that exact
workspace manifest explicitly, before interpreting metadata failures as a
package or coverage problem.
