# Changelog

## 0.1.0 (2026-04-19)

Initial release of the IR-to-GE-225 compiler backend.

### Added

- `compile_to_ge225(program)` function: translates a target-independent `IrProgram`
  into a GE-225 binary image (packed 3-bytes-per-word)
- Two-pass assembler: pass 1 assigns label addresses; pass 2 emits machine words
- Pass 0 pre-scan: collects virtual register indices and builds a constants table
- V1 IR opcode support: NOP, HALT, LOAD_IMM, ADD, ADD_IMM, SUB, AND_IMM (imm=1),
  MUL, DIV, CMP_EQ, CMP_NE, CMP_LT, CMP_GT, JUMP, BRANCH_Z, BRANCH_NZ, SYSCALL 1
- Prologue: `TON` (typewriter on) emitted at address 0 before all IR code
- Halt stub: `BRU code_end` self-loop appended after all code words
- Data section: zero-initialised spill slots (one per virtual register) + constants table
- `CompileResult` dataclass: `binary`, `halt_address`, `data_base`, `label_map`
- `CodeGenError` raised for unsupported opcodes, non-1 AND_IMM immediates, undefined labels
