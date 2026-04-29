# Changelog

## Unreleased

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
