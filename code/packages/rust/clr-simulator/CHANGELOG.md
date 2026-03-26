# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `CLRSimulator` -- type-inferring stack-based virtual machine with nullable values
- Load/store: ldc.i4 (compact 0-8, short -128..127, full 32-bit), ldloc/stloc
- Arithmetic: add, sub, mul, div with DivideByZeroException detection
- Control flow: br.s, brfalse.s, brtrue.s
- Two-byte comparison opcodes: ceq, cgt, clt via 0xFE prefix
- Special: nop, ldnull (nullable stack support), ret
- Encoding helpers and assembler
