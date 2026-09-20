---
category: Testing & coverage
---

# Declare direct dev dependencies for cross-crate builder execution tests

A lang-aot integration test imported ir_to_cil_bytecode because it was already a transitive dependency. Rust integration tests cannot import transitive dependencies directly, causing E0433. Add the builder as an explicit path dev-dependency before compiling builder-to-simulator proofs. Check the chosen crate manifest rather than assuming the dependency set of an earlier probe is sufficient.

The same change hit Clippy type_complexity for an explicitly annotated tuple-case vector. Let Rust infer the vector element type from its pushed cases instead of adding a redundant nested annotation.
