# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial release of the Starlark AST-to-bytecode compiler for Ruby.
- `Op` module with all 46 Starlark opcodes and human-readable NAMES lookup.
- `Compiler` class with `compile_starlark`, `compile_ast`, and `create_starlark_compiler` methods.
- Full Starlark language support:
  - Assignments (simple, augmented `+=`, `-=`, etc.)
  - All arithmetic operators (`+`, `-`, `*`, `/`, `//`, `%`, `**`)
  - All bitwise operators (`&`, `|`, `^`, `~`, `<<`, `>>`)
  - All comparison operators (`==`, `!=`, `<`, `>`, `<=`, `>=`, `in`, `not in`)
  - Boolean operators with short-circuit evaluation (`and`, `or`, `not`)
  - Control flow (`if`/`elif`/`else`, `for` loops, `break`, `continue`, `pass`)
  - Function definitions with parameters and default values
  - Function calls with positional and keyword arguments
  - Return statements (with and without value)
  - Collection literals (list, dict, tuple)
  - Attribute access and subscript operations
  - Load statements for module imports
  - Lambda expressions
  - Ternary conditional expressions
  - Expression statements
- `disassemble` method for human-readable bytecode output.
- Comprehensive test suite covering all language features.
