# Changelog

All notable changes to coding_adventures_bytecode_compiler will be documented here.

## [0.1.0] - 2026-03-18

### Added

- `Compiler` class that compiles AST to our custom VM's CodeObject
  - Supports NumberLiteral, StringLiteral, Name, BinaryOp, Assignment nodes
  - Constant pool and name pool with deduplication
  - Emits LOAD_CONST, LOAD_NAME, STORE_NAME, ADD, SUB, MUL, DIV, POP, HALT
- `JVMCompiler` class that compiles AST to real JVM bytecode bytes
  - Tiered number encoding: iconst_0..5 (1 byte), bipush (2 bytes), ldc (2 bytes)
  - Tiered local variable encoding: istore_0..3 / iload_0..3 (1 byte), istore/iload (2 bytes)
  - Arithmetic opcodes: iadd, isub, imul, idiv
  - JVMCodeObject with bytecode, constants, num_locals, local_names
- `CLRCompiler` class that compiles AST to real CLR IL bytecode bytes
  - Tiered number encoding: ldc.i4.0..8 (1 byte), ldc.i4.s (2 bytes), ldc.i4 (5 bytes)
  - Tiered local variable encoding: stloc.0..3 / ldloc.0..3 (1 byte), stloc.s/ldloc.s (2 bytes)
  - Arithmetic opcodes: add, sub, mul, div
  - CLRCodeObject with bytecode, num_locals, local_names
- `WASMCompiler` class that compiles AST to real WASM bytecode bytes
  - Uniform encoding: i32.const (5 bytes), local.set/get (2 bytes)
  - No pop for expression statements (WASM handles stack at function boundary)
  - Arithmetic opcodes: i32.add, i32.sub, i32.mul, i32.div_s
  - WASMCodeObject with bytecode, num_locals, local_names
- `compile_source` convenience method chaining lexer -> parser -> compiler
- Comprehensive test suite with >80% coverage
