# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-04-05

### Added

- Comprehensive test suite (16 tests) covering:
  VMValue equality and all variants, TypedVMValue creation,
  stack push/pop/peek, typed stack operations, empty-stack behavior,
  code execution with registered handlers, unknown opcode halting,
  context-aware execution, reset, PC jump control, CodeObject creation,
  max recursion depth setting

## [0.1.0] - 2026-04-05

### Added

- GenericVM stack-based bytecode interpreter with Strategy Pattern
- VMValue, TypedVMValue, Instruction, CodeObject types
- Untyped and typed operand stacks
- OpcodeHandler and ContextOpcodeHandler registration
- executeWithContext for WASM-style context-aware handlers
