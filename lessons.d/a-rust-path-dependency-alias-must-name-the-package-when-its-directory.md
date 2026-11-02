---
category: Rust
---

# A Rust path dependency alias must name the package when its directory and package names differ

An X.509 verifier declared `sha512 = { path = "../sha512" }`, assuming Cargo
would derive the package name from the dependency alias. The package at that
path is actually named `coding_adventures_sha512`, so resolution failed before
the crate compiled. When a local crate's import alias or directory differs from
its `[package] name`, declare all three facts explicitly, for example
`sha512 = { package = "coding_adventures_sha512", path = "../sha512" }`.
Inspect the dependency's `Cargo.toml` rather than inferring its package name
from either the directory or the Rust import used by a sibling crate.
