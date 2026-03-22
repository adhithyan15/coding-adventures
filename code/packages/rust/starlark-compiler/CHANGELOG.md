# Changelog

## 0.1.0 — 2026-03-21

### Added
- `Op` enum defining all Starlark bytecode opcodes (organized by category: stack, variables, arithmetic, comparison, control flow, functions, collections, subscript, iteration, module, I/O, VM control)
- `BINARY_OP_MAP`, `COMPARE_OP_MAP`, `AUGMENTED_ASSIGN_MAP`, `UNARY_OP_MAP` lookup tables
- `compile_starlark()` convenience function for source-to-bytecode compilation
- Comprehensive test suite for opcode definitions and operator maps
