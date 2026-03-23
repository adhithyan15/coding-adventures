# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- BytecodeCompiler: hardcoded compiler translating parser AST nodes (NumberLiteral,
  StringLiteral, BinaryOp, Assignment, ExpressionStmt) to our VM's instruction set
  (LOAD_CONST, STORE_NAME, LOAD_NAME, ADD, SUB, MUL, DIV, POP, HALT).
- JVMCompiler: compiler targeting JVM-style bytecode with specialized encodings —
  ICONST_n for 0-5, BIPUSH for byte-range, LDC for constant pool, ILOAD_n/ISTORE_n
  for local variable slots 0-3, ILOAD/ISTORE for higher slots.
- GenericCompiler: pluggable AST-to-bytecode framework with handler registration
  (register_rule), instruction emission (emit, emit_jump, patch_jump, current_offset),
  pool management (add_constant, add_name with deduplication), scope management
  (enter_scope, exit_scope with CompilerScope), nested compilation (compile_nested),
  and recursive dispatch (compile_node with pass-through for single-child wrappers).
- CompilerScope: local variable tracking with slot assignment, deduplication,
  parent-linked scope chain for lexical scoping.
- AST node constructors: Program, Assignment, ExpressionStmt, NumberLiteral,
  StringLiteral, Name, BinaryOp for BytecodeCompiler input.
- Generic AST types: ASTNode (non-terminal) and TokenNode (terminal) for
  GenericCompiler input.
- Data types: Instruction, CodeObject, JVMCodeObject constructors.
- All VM OpCode constants (0x01-0xFF) and JVM bytecode constants.
- Comprehensive busted test suite with 95%+ coverage.
- Ported from Go implementation at code/packages/go/bytecode-compiler/.
