---
category: Repo policy / workflow reminders
---

# Check the Rust workspace root before running Cargo

A package test was launched from `code/`, which has no `Cargo.toml`, even
though this repository's shared Rust workspace manifest is one directory
deeper at `code/packages/rust/Cargo.toml`. Before running `cargo -p ...`,
locate the applicable manifest with `rg --files -g Cargo.toml` or test the
expected workspace path, then set the command working directory to that
manifest's directory.
