# Changelog

All notable changes to @coding-adventures/starlark-vm will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- Initial release of the Starlark VM for TypeScript.
- 59 opcode handlers covering the full Starlark instruction set:
  - Stack operations (LOAD_CONST, POP, DUP, LOAD_NONE, LOAD_TRUE, LOAD_FALSE)
  - Variable operations (STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL, STORE_CLOSURE, LOAD_CLOSURE)
  - Arithmetic (ADD, SUB, MUL, DIV, FLOOR_DIV, MOD, POWER, NEGATE, BIT_AND, BIT_OR, BIT_XOR, BIT_NOT, LSHIFT, RSHIFT)
  - Comparisons (CMP_EQ, CMP_NE, CMP_LT, CMP_GT, CMP_LE, CMP_GE, CMP_IN, CMP_NOT_IN)
  - Boolean (NOT)
  - Control flow (JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE, JUMP_IF_FALSE_OR_POP, JUMP_IF_TRUE_OR_POP)
  - Functions (MAKE_FUNCTION, CALL_FUNCTION, CALL_FUNCTION_KW, RETURN)
  - Collections (BUILD_LIST, BUILD_DICT, BUILD_TUPLE, LIST_APPEND, DICT_SET)
  - Subscript/attribute (LOAD_SUBSCRIPT, STORE_SUBSCRIPT, LOAD_ATTR, STORE_ATTR, LOAD_SLICE)
  - Iteration (GET_ITER, FOR_ITER, UNPACK_SEQUENCE)
  - Module (LOAD_MODULE, IMPORT_FROM)
  - I/O (PRINT)
  - VM control (HALT)
- 23 built-in functions: type, bool, int, float, str, len, list, dict, tuple, range, sorted, reversed, enumerate, zip, min, max, abs, all, any, repr, hasattr, getattr, print
- Factory function `createStarlarkVM()` for creating configured VMs
- Convenience function `executeStarlark()` for one-call execution
- Type definitions: StarlarkFunction, StarlarkIterator, StarlarkResult
- Op enum with all 46 opcodes
- Helper utilities: isTruthy, starlarkRepr, starlarkValueRepr, starlarkTypeName
- Literate programming style with extensive inline documentation
- 100+ tests with vitest
