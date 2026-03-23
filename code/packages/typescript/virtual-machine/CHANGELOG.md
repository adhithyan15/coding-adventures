# Changelog

All notable changes to the virtual-machine package will be documented in this file.

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
