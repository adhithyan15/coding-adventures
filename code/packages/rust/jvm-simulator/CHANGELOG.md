# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `JVMSimulator` -- typed stack-based virtual machine with local variable slots
- Full opcode set: iconst_N, bipush, ldc, iload/istore, iadd, isub, imul, idiv
- Control flow: if_icmpeq, if_icmpgt, goto
- Method return: ireturn, return
- Constant pool support via ldc instruction
- 32-bit integer overflow wrapping
- Division by zero detection (panics with ArithmeticException message)
- Encoding helpers and assembler
