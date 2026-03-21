# Changelog

All notable changes to the Starlark VM package will be documented in this file.

## [0.1.0] - 2026-03-20

### Added
- Initial release of the Starlark VM package.
- `execute_starlark()` one-call function for source-to-result execution.
- `create_starlark_vm()` factory that returns a `GenericVM` configured for Starlark.
- `StarlarkResult` dataclass containing execution results: variables, output, and traces.
- `StarlarkFunction` wrapper for compiled CodeObjects with parameter metadata.
- `StarlarkIterator` implementing the iterator protocol for `for` loops.
- ~50 opcode handlers covering:
  - Stack: `LOAD_CONST`, `POP`, `DUP`, `LOAD_NONE`, `LOAD_TRUE`, `LOAD_FALSE`
  - Variables: `STORE_NAME`, `LOAD_NAME`, `STORE_LOCAL`, `LOAD_LOCAL`
  - Arithmetic: `ADD`, `SUB`, `MUL`, `DIV`, `FLOOR_DIV`, `MOD`, `POWER` with int/float promotion
  - Bitwise: `BIT_AND`, `BIT_OR`, `BIT_XOR`, `BIT_NOT`, `LSHIFT`, `RSHIFT`
  - Unary: `NEGATE`, `NOT`
  - Comparison: `CMP_EQ/NE/LT/GT/LE/GE/IN/NOT_IN`
  - Control flow: `JUMP`, `JUMP_IF_FALSE/TRUE`, `JUMP_IF_FALSE_OR_POP`, `JUMP_IF_TRUE_OR_POP`
  - Functions: `MAKE_FUNCTION`, `CALL_FUNCTION`, `CALL_FUNCTION_KW`, `RETURN`
  - Collections: `BUILD_LIST/DICT/TUPLE`, `LIST_APPEND`, `DICT_SET`
  - Subscript: `LOAD_SUBSCRIPT`, `STORE_SUBSCRIPT`, `LOAD_ATTR`, `STORE_ATTR`, `LOAD_SLICE`
  - Iteration: `GET_ITER`, `FOR_ITER`, `UNPACK_SEQUENCE`
  - I/O: `PRINT`
- ~25 built-in functions: `type`, `bool`, `int`, `float`, `str`, `len`, `list`, `dict`, `tuple`, `range`, `sorted`, `reversed`, `enumerate`, `zip`, `min`, `max`, `abs`, `all`, `any`, `repr`, `hasattr`, `getattr`, `print`.
- Starlark type semantics:
  - Int/float promotion in arithmetic operations
  - String concatenation (`+`) and repetition (`*`)
  - Truthiness rules for all types
  - Membership testing (`in`, `not in`) for lists, dicts, strings, tuples
- `get_all_builtins()` function returning the complete built-in function registry.
- 123 tests: 68 end-to-end (source → compile → execute → verify) + 55 unit tests for handlers and builtins.
