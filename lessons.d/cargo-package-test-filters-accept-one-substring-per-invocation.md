---
category: Rust
---

# Cargo package test filters accept one substring per invocation

`cargo test` accepts one positional test-name filter. Passing two focused test
names is a command-line error, not a request to run both. Run each exact test
in its own invocation, or choose one shared substring that matches the intended
set. Also run package-local commands from the crate directory when the
repository root is not a Cargo workspace.
