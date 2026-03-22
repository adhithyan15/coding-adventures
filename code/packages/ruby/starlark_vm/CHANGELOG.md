# Changelog

All notable changes to `coding_adventures_starlark_vm` will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- Initial release
- StarlarkFunction, StarlarkIterator, and StarlarkResult types
- 46 opcode handlers covering all Starlark bytecode instructions:
  - Stack: LOAD_CONST, POP, DUP, LOAD_NONE, LOAD_TRUE, LOAD_FALSE
  - Variables: STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL, STORE_CLOSURE, LOAD_CLOSURE
  - Arithmetic: ADD, SUB, MUL, DIV, FLOOR_DIV, MOD, POWER, NEGATE, BIT_AND/OR/XOR/NOT, LSHIFT, RSHIFT
  - Comparison: CMP_EQ/NE/LT/GT/LE/GE/IN/NOT_IN, NOT
  - Control flow: JUMP, JUMP_IF_FALSE/TRUE, JUMP_IF_FALSE_OR_POP, JUMP_IF_TRUE_OR_POP, BREAK, CONTINUE
  - Functions: MAKE_FUNCTION, CALL_FUNCTION, CALL_FUNCTION_KW, RETURN_VALUE
  - Collections: BUILD_LIST, BUILD_DICT, BUILD_TUPLE, LIST_APPEND, DICT_SET
  - Subscript/Attr: LOAD_SUBSCRIPT, STORE_SUBSCRIPT, LOAD_ATTR, STORE_ATTR, LOAD_SLICE
  - Iteration: GET_ITER, FOR_ITER, UNPACK_SEQUENCE
  - Module: LOAD_MODULE (stub), IMPORT_FROM
  - I/O: PRINT_VALUE
  - Control: HALT
- 23 builtin functions: print, len, type, bool, int, float, str, list, dict, tuple, range, sorted, reversed, enumerate, zip, min, max, abs, all, any, repr, hasattr, getattr
- create_starlark_vm() factory function
- execute_starlark() one-shot execution function
- Attribute resolution for list, dict, and string methods
- Comprehensive test suite with 50+ tests
