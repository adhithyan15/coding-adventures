# Changelog

All notable changes to the `starlark-vm` package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- Initial implementation of the Starlark virtual machine.
- `types.go`: Runtime types — `StarlarkFunction`, `StarlarkIterator`, `StarlarkResult`.
- `handlers.go`: All 59 opcode handlers covering:
  - Stack manipulation (LOAD_CONST, POP, DUP, LOAD_NONE/TRUE/FALSE)
  - Variable access (STORE/LOAD_NAME, STORE/LOAD_LOCAL, STORE/LOAD_CLOSURE)
  - Arithmetic (ADD, SUB, MUL, DIV, FLOOR_DIV, MOD, POWER, NEGATE)
  - Bitwise operators (AND, OR, XOR, NOT, LSHIFT, RSHIFT)
  - Comparisons (EQ, NE, LT, GT, LE, GE, IN, NOT_IN, NOT)
  - Control flow (JUMP, JUMP_IF_FALSE/TRUE, JUMP_IF_FALSE/TRUE_OR_POP, BREAK, CONTINUE)
  - Functions (MAKE_FUNCTION, CALL_FUNCTION, CALL_FUNCTION_KW, RETURN_VALUE)
  - Collections (BUILD_LIST/DICT/TUPLE, LIST_APPEND, DICT_SET)
  - Subscript/attribute (LOAD/STORE_SUBSCRIPT, LOAD/STORE_ATTR, LOAD_SLICE)
  - Iteration (GET_ITER, FOR_ITER, UNPACK_SEQUENCE)
  - Modules (LOAD_MODULE, IMPORT_FROM — stubs)
  - Output (PRINT_VALUE)
  - Halt (HALT)
- `builtins.go`: 23 built-in functions (print, len, type, bool, int, float, str, list, dict, tuple, range, sorted, reversed, enumerate, zip, min, max, abs, all, any, repr, hasattr, getattr).
- `vm.go`: Factory (`CreateStarlarkVM`) and convenience executor (`ExecuteStarlark`).
- `vm_test.go`: Comprehensive test suite covering all handler categories, builtins, helpers, and error cases.
