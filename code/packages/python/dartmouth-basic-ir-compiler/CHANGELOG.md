# Changelog

## 0.1.0 (2026-04-18)

Initial release of the Dartmouth BASIC IR compiler.

### Added

- `compile_basic(ast)` function: lowers a parsed Dartmouth BASIC AST to a target-independent `IrProgram`
- V1 statement support: `REM`, `LET`, `PRINT` (string literals), `GOTO`, `IF/THEN`, `FOR/NEXT`, `END`, `STOP`
- All six relational operators for `IF`: `<`, `>`, `=`, `<>`, `<=`, `>=`
- Expression compilation: addition, subtraction, multiplication, division, unary minus
- Fixed virtual register assignment: variables A–Z → v1–v26
- GE-225 typewriter code table for `PRINT` character encoding
- `CompileError` raised for V1-excluded features (GOSUB, DIM, INPUT, DEF FN, power operator)
- `CompileResult` dataclass with `program` and `var_regs`
