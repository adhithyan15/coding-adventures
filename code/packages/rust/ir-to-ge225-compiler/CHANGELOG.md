# Changelog

## Unreleased

### Added
- Initial Rust port: three-pass GE-225 assembler from Python.
- `validate_for_ge225` — checks opcode support, 20-bit constant range,
  SYSCALL number, and AND_IMM immediate constraints.
- `compile_to_ge225` — Pass 0 (register/constant collection),
  Pass 1 (label address assignment), Pass 2 (word emission).
- `GE225CodeGenerator` adapter implementing the
  `CodeGenerator<IrProgram, CompileResult>` protocol (LANG20).
- `IrOp::Mul` support — 6-word sequence `LDA; LQA; LDZ; MPY; LAQ; STA`
  using the GE-225's 40-bit accumulate multiply.
- `IrOp::Div` support — 5-word sequence `LDA; LQA; LDZ; DVD; STA`
  using the GE-225's 40-bit signed divide.
- 35+ unit tests covering every opcode emitter and validation rule.

### Fixed
- **GE-225 branch-test "inhibit" semantics** — The GE-225 uses *inhibit* semantics
  for conditional skip instructions: the named condition *prevents* the skip; the
  skip occurs when the condition is FALSE, not when it is TRUE.

  Four instruction sites were using the wrong instruction:

  | Site | Before | After |
  |------|--------|-------|
  | `BRANCH_Z` (jump when A==0) | `BNZ` | `BZE` |
  | `BRANCH_NZ` (jump when A≠0) | `BZE` | `BNZ` |
  | `CMP_EQ` equality test | `BNZ` | `BZE` |
  | `CMP_NE` inequality test | `BZE` | `BNZ` |
  | `CMP_LT`/`CMP_GT` signed diff | `BPL` (skips when A<0) | `BMI` (skips when A≥0) |
  | `AND_IMM 1` result layout | LDO@+3, LDZ@+5 | LDZ@+3, LDO@+5 |

  The double-inversion (wrong CMP result × wrong BRANCH_NZ) accidentally produced
  correct results for `IF … THEN` tests, masking the bug.  `BRANCH_Z` used with
  plain arithmetic values (leading-zero suppression in `emit_print_number`) and
  `AND_IMM` used in NOT-bool expressions exposed the true behavior.

  Updated all four `emit_*` methods and the top-level module documentation to
  describe the inhibit-semantics table precisely.
