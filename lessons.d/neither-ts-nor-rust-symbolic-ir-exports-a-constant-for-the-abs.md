---
category: Compiler / VM / language pipeline
---

# Neither TS nor Rust `symbolic-ir` exports a constant for the `Abs` head

When implementing `Abs` rules in the Phase 29-33 port, references must use the string form (`sym("Abs")` in TS, `IRNode::Symbol("Abs".to_string())` in Rust). The asymmetry with `SIN`, `COS`, etc. — which do have head-symbol constants — is silent: imports succeed for the wrong-cased near-misses (`SQRT` is exported, `ABS` is not). Add an `ABS` constant to `symbolic-ir` when one of the languages first ships an Abs handler, or accept the string-form convention indefinitely.
