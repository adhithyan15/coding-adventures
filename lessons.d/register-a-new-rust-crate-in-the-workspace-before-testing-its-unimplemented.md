---
category: Workspace & package metadata
---

# Register a new Rust crate in the workspace before testing its unimplemented API boundary

A new crate directory with a valid package manifest still inherits the nearest
Rust workspace. If its path is absent from the root `workspace.members`, Cargo
stops at workspace discovery before compiling a deliberately failing API test.
Add the package to `code/packages/rust/Cargo.toml` as part of the test scaffold,
then confirm the red test reaches the intended unresolved API or assertion.
