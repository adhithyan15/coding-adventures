---
category: Rust
---

# Integration tests must import IR operands from their public owner

The exact cap23 integration companion imported algol_iir_compiler::Operand,
but that crate only privately imports the enum. Actual68c focused build failed
E0603 before any harness/runtime execution; the external wrapper recorded the
failed build and correctly stopped. Import interpreter_ir::Operand, the public
owner already declared in the package's normal dependencies. Do not expose a
new compiler API or add a dependency to repair a test import. Preserve the
failed receipt and rerun to actual harness completion.
