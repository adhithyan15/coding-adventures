# Changelog

## [0.1.0] - 2026-03-18

### Added
- `OpCode` module with 20 instruction constants (LOAD_CONST, ADD, SUB, MUL, DIV, etc.)
- `Instruction` immutable data type (opcode + optional operand)
- `CodeObject` immutable data type (instructions + constants pool + names pool)
- `VM` class with `execute()` and `step()` methods
- `VMTrace` data type for recording every execution step
- `CallFrame` data type for function call contexts
- Trace recording with stack snapshots, variable state, and descriptions
- Error classes: VMError, StackUnderflowError, UndefinedNameError, DivisionByZeroError, InvalidOpcodeError, InvalidOperandError
- Language-agnostic design -- no Python or Ruby specific instructions
