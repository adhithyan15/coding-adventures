# Changelog

All notable changes to the Starlark Compiler package will be documented in this file.

## [0.1.0] - 2026-03-20

### Added
- Initial release of the Starlark compiler package.
- `compile_starlark()` one-call function for source-to-bytecode compilation.
- `create_starlark_compiler()` factory that returns a `GenericCompiler` configured for Starlark.
- `Op` IntEnum with ~50 opcodes organized by high nibble:
  - 0x0_: Stack — `LOAD_CONST`, `POP`, `DUP`, `LOAD_NONE`, `LOAD_TRUE`, `LOAD_FALSE`
  - 0x1_: Variables — `STORE_NAME`, `LOAD_NAME`, `STORE_LOCAL`, `LOAD_LOCAL`, `STORE_CLOSURE`, `LOAD_CLOSURE`
  - 0x2_: Arithmetic — `ADD`, `SUB`, `MUL`, `DIV`, `FLOOR_DIV`, `MOD`, `POWER`, `NEGATE`, `BIT_AND/OR/XOR/NOT`, `LSHIFT`, `RSHIFT`
  - 0x3_: Comparison — `CMP_EQ/NE/LT/GT/LE/GE/IN/NOT_IN`, `NOT`
  - 0x4_: Control flow — `JUMP`, `JUMP_IF_FALSE/TRUE`, `JUMP_IF_FALSE_OR_POP`, `JUMP_IF_TRUE_OR_POP`
  - 0x5_: Functions — `MAKE_FUNCTION`, `CALL_FUNCTION`, `CALL_FUNCTION_KW`, `RETURN`
  - 0x6_: Collections — `BUILD_LIST/DICT/TUPLE`, `LIST_APPEND`, `DICT_SET`
  - 0x7_: Subscript — `LOAD_SUBSCRIPT`, `STORE_SUBSCRIPT`, `LOAD_ATTR`, `STORE_ATTR`, `LOAD_SLICE`
  - 0x8_: Iteration — `GET_ITER`, `FOR_ITER`, `UNPACK_SEQUENCE`
  - 0xA_: I/O — `PRINT`
  - 0xFF: `HALT`
- Operator mapping tables: `BINARY_OP_MAP`, `COMPARE_OP_MAP`, `AUGMENTED_ASSIGN_MAP`.
- Handlers for all Starlark grammar rules covering:
  - Expressions: literals, names, binary/unary ops, comparisons, boolean ops, ternary (if-else)
  - Statements: assignment, augmented assignment, return, pass, break, continue
  - Compound statements: if/elif/else, for loops, def (function definitions)
  - Data structures: list/dict/tuple literals, list/dict comprehensions
  - Function calls: positional args, keyword args
  - Member access: attribute access, subscript, slicing
- Helper functions: `_parse_string_literal()`, `_type_name()` exported for testing.
- 22 unit tests covering opcodes, operator maps, string parsing, and compiler factory.
