# Changelog

All notable changes to the `@coding-adventures/bytecode-compiler` package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- **BytecodeCompiler**: Compiles ASTs into CodeObject instructions for our custom VM.
  - Supports NumberLiteral, StringLiteral, Name, BinaryOp, and Assignment nodes.
  - Constant pool and name pool with deduplication.
  - `compileSource()` convenience function for end-to-end compilation.
- **JVMCompiler**: Compiles ASTs into real JVM bytecode bytes.
  - Tiered number encoding: iconst_0-5 (1 byte), bipush (2 bytes), ldc (2 bytes).
  - Tiered local variable encoding: istore_0-3 / iload_0-3 (1 byte), istore / iload (2 bytes).
  - Constant pool for values outside bipush range.
- **CLRCompiler**: Compiles ASTs into real CLR IL bytecode bytes.
  - Tiered number encoding: ldc.i4.0-8 (1 byte), ldc.i4.s (2 bytes), ldc.i4 (5 bytes).
  - Tiered local variable encoding: stloc.0-3 / ldloc.0-3 (1 byte), stloc.s / ldloc.s (2 bytes).
  - Inline 4-byte little-endian encoding for large constants (no constant pool needed).
- **WASMCompiler**: Compiles ASTs into real WebAssembly bytecode bytes.
  - Uniform encoding: i32.const + 4-byte LE int32 for all integers.
  - local.get / local.set + 1-byte index for all local variables.
  - No pop for expression statements (WASM validates stack at function boundary).
- **VM types**: OpCode enum, Instruction, CodeObject, and minimal VirtualMachine for testing.
- Comprehensive test suites for all four compilers with >80% coverage.
- Knuth-style literate programming comments throughout all source files.

### Notes

- Ported from the Python `bytecode_compiler` package.
- The VirtualMachine is a minimal implementation for end-to-end tests. A full VM package will be created separately.
- CLR and WASM compilers do not yet support string literals (throws TypeError).
