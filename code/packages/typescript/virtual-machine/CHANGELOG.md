# Changelog

All notable changes to the virtual-machine package will be documented in this file.

## [0.2.0] - 2026-04-04

### Added
- **Typed value support**: `TypedVMValue` interface for VMs with typed operand stacks
  (WASM i32/i64/f32/f64, future JVM/CLR support).
- **Typed stack operations**: `pushTyped()`, `popTyped()`, `peekTyped()` on GenericVM,
  independent from the existing untyped `push()`/`pop()`.
- **BigInt in VMValue**: Extended `VMValue` type to include `bigint` for 64-bit integer
  support (WASM i64, JVM long, CLR Int64).
- **Pre-instruction hook**: `setPreInstructionHook()` allows transforming instructions
  before dispatch. Used by WASM to decode variable-length bytecodes into fixed-format
  Instruction objects.
- **Post-instruction hook**: `setPostInstructionHook()` runs after each handler for
  tracing, profiling, or assertions.
- **Context-aware handlers**: `registerContextOpcode()` and `executeWithContext()` let
  typed VMs pass per-execution state (memory, tables, globals) to handlers without
  global state. Context handlers take priority over regular handlers during context
  execution.
- 16 new tests covering typed stack, BigInt, hooks, and context execution.

## [0.1.0] - 2026-03-19

### Added
- Initial TypeScript port from the Python virtual-machine package.
- Full OpCode instruction set: LOAD_CONST, POP, DUP, STORE_NAME, LOAD_NAME,
  STORE_LOCAL, LOAD_LOCAL, ADD, SUB, MUL, DIV, CMP_EQ, CMP_LT, CMP_GT,
  JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE, CALL, RETURN, PRINT, HALT.
- VirtualMachine class with fetch-decode-execute cycle.
- VMTrace system for step-by-step execution recording.
- CallFrame support for function calls with saved execution context.
- Error hierarchy: VMError, StackUnderflowError, UndefinedNameError,
  DivisionByZeroError, InvalidOpcodeError, InvalidOperandError.
- assembleCode convenience function for building CodeObjects.
- instructionToString helper for human-readable instruction display.
- Comprehensive test suite covering every opcode, error path, and
  end-to-end programs (countdown loop, if/else, sum 1-to-5, string concat).
- Knuth-style literate programming comments throughout.
