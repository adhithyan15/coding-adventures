# Changelog

## 0.1.0 (2026-03-19)

### Added
- Initial TypeScript port from Python clr-simulator
- CLRSimulator: complete CLR IL bytecode execution engine
- Constant loading: ldc.i4.0-8, ldc.i4.s, ldc.i4
- Local variable access: ldloc.0-3, stloc.0-3, ldloc.s, stloc.s
- Type-inferred arithmetic: add, sub, mul, div
- Two-byte comparison opcodes: ceq, cgt, clt (0xFE prefix)
- Branch instructions: br.s, brfalse.s, brtrue.s
- Miscellaneous: nop, ldnull, ret
- Encoding helpers: encodeLdcI4, encodeStloc, encodeLdloc, assembleClr
- Full test suite ported from Python with vitest
- Knuth-style literate programming comments preserved from Python source
