# Changelog — ir-to-beam

## [0.1.0] — 2026-04-28

### Added

- **`encoder` module** — BEAM IFF container builder.
  - `BEAMTag` — 3-bit compact-term type tags (U, I, A, X, Y, F).
  - `BEAMOperand` / `BEAMInstruction` — typed instruction representation.
  - `BEAMImport` / `BEAMExport` — import and export table row types.
  - `BEAMModule` — complete in-memory BEAM module.
  - `encode_compact_term()` — variable-width BEAM operand encoding
    (small / medium / large forms).
  - `encode_beam()` — serialize a `BEAMModule` to a complete `.beam` binary
    with AtU8, Code, StrT, ImpT, ExpT, LocT, Attr, CInf chunks.

- **`backend` module** — IR → BEAM lowering pass.
  - `BEAMBackendConfig` — lowering configuration (module name).
  - `BEAMBackendError` — typed errors (ValidationFailed, UnsupportedOp,
    InvalidOperand, UndefinedLabel).
  - `validate_for_beam()` — pre-flight validation (detects unsupported ops,
    empty entry label).
  - `lower_ir_to_beam()` — two-pass lowering:
    - Pass 1: collect LABEL instructions → assign BEAM label numbers (starting
      at 3; 1 and 2 are reserved for the `func_info` preamble).
    - Pass 2: translate each IR instruction to BEAM bytecode using the
      mapping described below.
  - Supported opcodes: LABEL, LOAD_IMM, ADD, ADD_IMM, SUB, AND, AND_IMM,
    JUMP, BRANCH_Z, BRANCH_NZ, CALL, RET, HALT, NOP, COMMENT.
  - Synthesised ops: ADD_IMM / AND_IMM are expanded to MOVE + GC_BIF2;
    BRANCH_Z → `is_ne_exact`; BRANCH_NZ → `is_eq_exact`.
  - Unsupported (validation errors): LOAD_BYTE, STORE_BYTE, LOAD_WORD,
    STORE_WORD, LOAD_ADDR, SYSCALL, CMP_EQ, CMP_NE, CMP_LT, CMP_GT.

- **`codegen` module** — LANG20 adapter.
  - `BEAMCodeGenerator` — implements `CodeGenerator<IrProgram, BEAMModule>`.
    - `name()` → `"beam"`.
    - `validate()` → delegates to `validate_for_beam`.
    - `generate()` → delegates to `lower_ir_to_beam`, panics on invalid IR.
  - `BEAMCodeGenerator::new(module_name)` and `::default_module()`.

- **Tests** — 14 encoder tests + 24 backend tests + 11 codegen tests = 49 total.
