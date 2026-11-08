# Changelog

All notable changes to the `coding_adventures_warrant_ir` crate will be documented in this file.

## [0.1.0] - 2026-11-08

### Added
- Initial implementation of the Warrant Intermediate Representation (WIR) core and reference interpreter ([VER25](code/specs/VER25-wir-core-interpreter.md), VER-P021).
- Abstract syntax tree (AST) supporting scalar integer types (`U8`..`U64`, `I8`..`I64`), `Bool`, `Unit`, `ByteSlice`, `Tuple`, dynamic tagged values, and pointers with provenance.
- Explicit arithmetic overflow modes (`Wrap`, `Trap`, `Saturate`).
- Security taint and information-flow lattice (`Public`, `Secret`, `Hostile`).
- Static type checker and control-flow well-formedness validator enforcing type safety, single-assignment within blocks, valid branching, and label flow integrity.
- Memory model with allocation bounds, lifetime tracking, and provenance IDs, halting on out-of-bounds or use-after-free with explicit `Stuck` outcomes.
- Deterministic reference interpreter with step fuel budgets, call-stack recursion depth guards, and capability call dispatch.
- Observable event trace recorder capturing branch choices, memory access addresses, and capability invocations for self-composition constant-time validation (CWE-208).
- Step cost accounting semantics for operational complexity proofs.
