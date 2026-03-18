# Changelog

## [0.1.0] - 2026-03-18

### Added
- CLRSimulator: standalone CLR IL bytecode simulator with real ECMA-335 opcode values
- Constants: ldc.i4.0-8, ldc.i4.s, ldc.i4, ldnull
- Locals: ldloc.0-3, stloc.0-3, ldloc.s, stloc.s
- Arithmetic: add, sub, mul, div (type-inferred)
- Control flow: br.s, brfalse.s, brtrue.s with relative offsets
- Comparison: ceq, cgt, clt (two-byte opcodes with 0xFE prefix)
- Utilities: nop, ret
- Encoding helpers: encode_ldc_i4, encode_stloc, encode_ldloc, assemble_clr
- CLRTrace: immutable Data.define trace records
