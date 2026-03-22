# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- Initial release of the Starlark AST-to-bytecode compiler.
- `CompileStarlark(source string)` — one-shot compilation from source to CodeObject.
- `CompileAST(ast *parser.ASTNode)` — compile a pre-parsed AST.
- `Disassemble(code vm.CodeObject)` — human-readable bytecode disassembly.
- 46 opcode definitions in `opcodes.go` covering:
  - Stack manipulation (LOAD_CONST, LOAD_NONE, LOAD_TRUE, LOAD_FALSE)
  - Variable operations (STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL, closures)
  - Arithmetic (ADD, SUB, MUL, DIV, FLOOR_DIV, MOD, POWER, NEGATE)
  - Bitwise (BIT_AND, BIT_OR, BIT_XOR, BIT_NOT, LEFT_SHIFT, RIGHT_SHIFT)
  - Comparisons (CMP_EQ, CMP_NE, CMP_LT, CMP_GT, CMP_LE, CMP_GE, CMP_IN, CMP_NOT_IN)
  - Boolean (NOT, JUMP_IF_FALSE_OR_POP, JUMP_IF_TRUE_OR_POP)
  - Control flow (JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE, BREAK, CONTINUE)
  - Functions (MAKE_FUNCTION, CALL_FUNCTION, CALL_FUNCTION_KW, RETURN_VALUE)
  - Collections (BUILD_LIST, BUILD_DICT, BUILD_TUPLE, LIST_APPEND, DICT_SET)
  - Subscript/attribute (LOAD_SUBSCRIPT, STORE_SUBSCRIPT, LOAD_ATTR, STORE_ATTR, LOAD_SLICE)
  - Iteration (GET_ITER, FOR_ITER, UNPACK_SEQUENCE)
  - Modules (LOAD_MODULE, IMPORT_FROM)
- Compilation handlers for all Starlark grammar rules including:
  - Simple statements: assignment, return, break, continue, pass, load
  - Compound statements: if/elif/else, for, def
  - Expressions: full precedence chain with short-circuit booleans
  - Collections: list, dict, tuple literals
  - Function calls with positional and keyword arguments
  - Lambda expressions and ternary conditionals
- Comprehensive test suite covering all language features.
