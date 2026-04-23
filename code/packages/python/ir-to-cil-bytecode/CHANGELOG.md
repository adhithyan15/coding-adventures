# Changelog

## [Unreleased]

### Added

- **`validate_for_clr(program)` pre-flight validator**: inspects an `IrProgram`
  for CLR backend incompatibilities *before* any bytecode is generated.  Returns
  a list of human-readable error strings (empty list = valid).  Three rules are
  checked:
  1. **Opcode support** — every opcode must appear in `_CLR_SUPPORTED_OPCODES`.
  2. **Constant range** — `LOAD_IMM` and `ADD_IMM` immediates must fit in a CIL
     `int32` (−2 147 483 648 to 2 147 483 647).
  3. **SYSCALL number** — only SYSCALL 1 (write byte), 2 (read byte), and 10
     (process exit) are wired in the V1 CLR host.  Oct's 8008-specific SYSCALL
     numbers (20+PORT for input, 40+PORT for output) are caught here instead of
     failing silently at runtime.
- `validate_for_clr` exported from `ir_to_cil_bytecode.__init__`.
- `TestValidateForClr` test class (12 new tests in `tests/test_backend.py`)
  covering: passing arithmetic programs, all three wired SYSCALLs (1/2/10),
  int32 boundary values, Oct SYSCALL 57 rejection, Oct SYSCALL 23 rejection,
  SYSCALL 0 rejection, LOAD_IMM above/below int32 range, integration with
  `lower_ir_to_cil_bytecode()`, and multi-error accumulation.

### Changed

- `lower_ir_to_cil_bytecode()` now calls `validate_for_clr()` as a pre-flight
  check before `CILLoweringPipeline` runs.  Any violation raises
  `CILBackendError` with message prefix
  `"IR program failed CLR pre-flight validation"`.  Previously, unsupported
  SYSCALL numbers would pass through to the PE binary and only be caught at
  runtime by the CLR VM.

## 0.2.0

- Add `IrOp.OR`, `IrOp.OR_IMM` lowering: emits CIL `or` (0x60).
- Add `IrOp.XOR`, `IrOp.XOR_IMM` lowering: emits CIL `xor` (0x61).
- Add `IrOp.NOT` lowering: emits `ldc.i4.m1` (0x15) + `xor` (0x61), the
  canonical CIL bitwise-complement idiom (`NOT x = x XOR -1`).
- Switch `IrOp.AND`/`IrOp.AND_IMM` emission to use the new `emit_and()`
  builder helper for consistency.
- Add seven new tests covering OR, OR_IMM, XOR, XOR_IMM, NOT, double NOT
  round-trip, and a mixed bitwise-ops method body.

## 0.1.0

- Add the initial composable IR-to-CIL bytecode lowering package.
- Support compiler IR arithmetic, comparisons, branches, calls, static data
  offsets, memory helper calls, and syscall helper calls.
- Expose an injectable token provider so CLI metadata assembly can be composed
  above bytecode lowering.
