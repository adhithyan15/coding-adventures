# Changelog

## 0.2.0

- Add `IrOp.OR`, `IrOp.OR_IMM` lowering: emits CIL `or` (0x60).
- Add `IrOp.XOR`, `IrOp.XOR_IMM` lowering: emits CIL `xor` (0x61).
- Add `IrOp.NOT` lowering: emits `ldc.i4.m1` (0x15) + `xor` (0x61), the
  canonical CIL bitwise-complement idiom (`NOT x = x XOR -1`).
- Switch `IrOp.AND`/`IrOp.AND_IMM` emission to use the new `emit_and()`
  builder helper for consistency.
- Add seven new tests covering OR, OR_IMM, XOR, XOR_IMM, NOT, double NOT
  round-trip, and a mixed bitwise-ops method body.

## 0.1.0

- Add the initial composable IR-to-CIL bytecode lowering package.
- Support compiler IR arithmetic, comparisons, branches, calls, static data
  offsets, memory helper calls, and syscall helper calls.
- Expose an injectable token provider so CLI metadata assembly can be composed
  above bytecode lowering.
