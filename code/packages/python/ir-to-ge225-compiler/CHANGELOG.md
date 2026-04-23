# Changelog

## 0.2.0 — 2026-04-20

### Added

- **`validate_for_ge225(program)` pre-flight validator**: inspects an
  `IrProgram` for GE-225 incompatibilities *before* any code is generated.
  Returns a list of human-readable error strings (empty list = valid).
  Four rules are checked:
  1. **Opcode support** — rejects opcodes absent from the GE-225 V1 set
     (e.g. `LOAD_BYTE`, `STORE_BYTE`, `LOAD_WORD`, `STORE_WORD`, `AND`,
     `CALL`, `RET`, `LOAD_ADDR`).
  2. **Constant range** — `LOAD_IMM` and `ADD_IMM` immediates must fit in a
     20-bit signed word (−524 288 to 524 287).
  3. **SYSCALL number** — only SYSCALL 1 (typewriter print) is wired up in V1.
  4. **AND_IMM immediate** — `AND_IMM` only supports immediate value 1 (parity
     extraction via the GE-225 mask instruction).
- `validate_for_ge225` exported from `ir_to_ge225_compiler.__init__`.
- `TestValidateForGe225` test class (14 tests) covering all four rules,
  boundary-value constants, and integration with `compile_to_ge225`.

### Changed

- `compile_to_ge225()` now calls `validate_for_ge225()` as a pre-flight check
  before the two-pass assembler runs.  Any violation raises `CodeGenError` with
  message prefix `"IR program failed GE-225 pre-flight validation"`.
- The 20-bit encoding step in `_CodeGen` now asserts that values have already
  passed validation, making the `& 0xFFFFF` mask safe (only needed for
  two's-complement negative numbers, never for silent overflow).
- Added word-size constants `_GE225_WORD_MIN = -(1 << 19)` and
  `_GE225_WORD_MAX = (1 << 19) - 1` for single-source-of-truth boundary checks.

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
