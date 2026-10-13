---
category: Rust
---

# Read the Cargo package name before adding a prefix

A downstream validation command guessed `coding-adventures-javascript-to-semantic-ir`.
Cargo rejected the package selection before running any tests: its manifest names
the package `javascript-to-semantic-ir`. Read each manifest's package name instead
of inferring it from sibling naming conventions. A package-selection failure is
not a test result; rerun with the actual name and capture completion. Also run
workspace commands from `code/packages/rust`: this repo's root has no Cargo
manifest. A later root-level `cargo build` failed before building the native
runner and had to be rerun from that actual workspace.
