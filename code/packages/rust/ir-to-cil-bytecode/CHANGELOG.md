# Changelog

## [0.4.0] — 2026-06-10 — `isinst` opcode (McCarthy W7 `pair?`)

Added the `IsInst` (0x75) `CILOpcode` + `emit_isinst(token)` builder method — the
CLR type test backing McCarthy `pair?` (is the boxed value an `object[]` cons cell?).

## [0.3.0] — 2026-06-10 — `box`/`unbox.any` opcodes (McCarthy W6b)

Added the `Box` (0x8C) and `UnboxAny` (0xA5) CIL opcodes + `emit_box`/`emit_unbox_any`
builder methods and an `INT32_TYPE_TOKEN` (the `System.Object[]` cons-cell atom
boxing). These let the CLR backend box an `int32` McCarthy atom into an `object`
(the CLR counterpart of the wasm `i31ref` / JVM `Integer`).

## [0.2.0] — 2026-05-11

### Added — Bitwise OR, XOR, NOT lowering

- **`IrOp::Or`** — lowered to CIL `or` (opcode `0x60`).
- **`IrOp::OrImm`** — register + immediate OR: `ldloc src; ldc.i4 imm; or; stloc dst`.
- **`IrOp::Xor`** — lowered to CIL `xor` (opcode `0x61`).
- **`IrOp::XorImm`** — register + immediate XOR: `ldloc src; ldc.i4 imm; xor; stloc dst`.
- **`IrOp::Not`** — one's-complement NOT synthesised as `ldc.i4.m1 (0x15); xor`
  (CIL has no native bitwise-NOT; matches the JVM `iconst_m1+ixor` and
  WASM `i32.const -1; i32.xor` strategies).
- All five new ops added to `CLR_SUPPORTED_OPS`; opcode support count 27 → 32.
- 7 new unit tests covering opcode presence in emitted bytecode, all-ops lowering
  success sweep, and `validate_for_clr` acceptance sweep.

---

## [0.1.0] — Unreleased

- add the first Rust `ir-to-cil-bytecode` backend
- port the Python `cil-bytecode-builder` two-pass CIL assembler to Rust
  (`builder.rs`): `CILBytecodeBuilder`, `CILOpcode`, `CILBranchKind`,
  branch-promotion algorithm, and encoding helpers
- port the Python `ir-to-cil-bytecode` backend to Rust (`backend.rs`):
  `validate_for_clr`, `lower_ir_to_cil_bytecode`, `CILProgramArtifact`,
  `CILMethodArtifact`, `CILTokenProvider`, `SequentialCILTokenProvider`,
  `CILHelper`, `CILHelperSpec`
- implement `CILCodeGenerator` LANG20 adapter
  (`CodeGenerator<IrProgram, CILProgramArtifact>`)
- validation rules: opcode support (25 ops), int32 immediate range,
  SYSCALL whitelist (1/2/10), static data ≤ 16 MiB
- full instruction coverage: LOAD_IMM, LOAD_ADDR, LOAD_BYTE, LOAD_WORD,
  STORE_BYTE, STORE_WORD, ADD, ADD_IMM, SUB, AND, AND_IMM, CMP_EQ,
  CMP_NE, CMP_LT, CMP_GT, JUMP, BRANCH_Z, BRANCH_NZ, CALL, RET,
  HALT, SYSCALL, LABEL, COMMENT, NOP
- write 40+ unit tests covering builder, backend, and adapter
