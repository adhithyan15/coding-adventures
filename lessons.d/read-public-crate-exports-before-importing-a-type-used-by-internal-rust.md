---
category: Rust
---

# Read public crate exports before importing a type used by internal Rust tests

An external scheduler observation copied an EsVersion import from internal test context and failed compilation: javascript-ast uses that enum privately. Internal tests can see private parent-module imports, so their use statements do not prove an external crate re-export exists.

Inspect the crate public imports/exports before reusing a test snippet. Import EsVersion from its defining javascript-tokens crate and link that tested local dependency explicitly. The corrected Rust 1.99 observer compiled and executed; no implementation or PR source changed.
