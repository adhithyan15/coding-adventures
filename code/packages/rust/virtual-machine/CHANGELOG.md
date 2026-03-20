# Changelog

## 0.1.0 — 2026-03-20

### Added
- `GenericVM` — pluggable stack-based virtual machine with handler registration
- `CodeObject` — compiled bytecode representation (instructions + constants + names)
- `Value` enum — Int, Float, Str, Bool, Code, Null
- `Instruction` and `Operand` types for bytecode representation
- `VMTrace` — execution traces capturing state at each step
- `VMError` — typed error handling (StackUnderflow, UndefinedName, DivisionByZero, etc.)
- Standard opcodes module with 25+ common opcodes (stack, arithmetic, comparison, logic, control, I/O)
- Stack operations: push, pop, peek
- Call stack with configurable max recursion depth
- Program counter control: advance_pc, jump_to
- Built-in function registration
- VM freezing to prevent handler modification after setup
- Reset functionality that preserves registered handlers
- Step-by-step and full execution modes
