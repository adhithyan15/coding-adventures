# Changelog

## [0.1.0] - 2026-03-18

### Added
- JVMSimulator: standalone JVM bytecode simulator with real opcode values
- Opcodes: iconst_0-5, bipush, ldc, iload/istore (shortcuts + generic), iadd/isub/imul/idiv
- Control flow: goto, if_icmpeq, if_icmpgt with relative offsets
- Return: ireturn (with return value), return (void)
- Encoding helpers: encode_iconst, encode_iload, encode_istore, assemble_jvm
- 32-bit signed integer wrapping (two's complement)
- JVMTrace: immutable Data.define trace records
