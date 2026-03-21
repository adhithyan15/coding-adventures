# Changelog

## 0.1.0 (2026-03-19)

### Added
- Initial TypeScript port from Python jvm-simulator
- JVMSimulator: complete JVM bytecode execution engine
- Typed integer opcodes: iconst_N, bipush, ldc, iload, istore, iadd, isub, imul, idiv
- Control flow: goto, if_icmpeq, if_icmpgt
- Return instructions: return (void), ireturn (with value)
- 32-bit signed integer wrapping (two's complement)
- Encoding helpers: encodeIconst, encodeIstore, encodeIload, assembleJvm
- Full test suite ported from Python with vitest
- Knuth-style literate programming comments preserved from Python source
