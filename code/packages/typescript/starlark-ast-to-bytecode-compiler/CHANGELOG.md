# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- Initial release porting the Python reference implementation to TypeScript
- All 46 Starlark opcodes (LOAD_CONST through HALT) with hex-grouped organization
- Operator-to-opcode mapping tables: BINARY_OP_MAP (12 ops), COMPARE_OP_MAP (8 ops), AUGMENTED_ASSIGN_MAP (12 ops), UNARY_OP_MAP (3 ops)
- ~55 grammar rule handlers covering the full Starlark language:
  - Top-level structure (file, simple_stmt)
  - Simple statements (assign_stmt, return_stmt, break_stmt, continue_stmt, pass_stmt, load_stmt)
  - Compound statements (if_stmt with elif/else, for_stmt with break/continue, def_stmt with parameters and defaults, suite)
  - Expressions (ternary if/else, or/and short-circuit, not, comparison, binary ops, unary factor, power)
  - Primary expressions (atom with suffix chaining: .attr, [subscript], (call))
  - Collection literals (list, dict, tuple, parenthesized expressions)
  - Comprehensions (list and dict comprehensions with for/if clauses)
  - Lambda expressions
  - Load statements (Starlark's import mechanism)
- `compileStarlark()` convenience function for one-step source-to-bytecode compilation
- `createStarlarkCompiler()` factory for step-by-step compilation
- `parseStringLiteral()` utility for handling escape sequences
- Comprehensive test suite with 100+ tests covering all handlers
- BUILD file with transitive dependency chain installation
