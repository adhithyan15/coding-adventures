---
category: Rust
---

# Run cross-crate execution probes in a crate with both dependencies

A CLR lowering probe placed in iir-to-cil-bytecode could not import
clr_simulator. Locate the owning Cargo.toml and inspect dependencies first;
lang-aot under code/packages/rust already depends on both crates and is the
existing integration owner. Do not guess a code/programs path or add a backend
dependency just to run a temporary audit probe. Retain evidence outside the
checkout and remove only the specific temporary test after execution.