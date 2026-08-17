# Changelog — intel8086-backend

## [0.1.0] - 2026-08-17

### Added

- `Intel8086Backend` implementing the `jit_core::backend::Backend` trait:
  `name() == "intel8086"`, `compile`/`compile_function` lowering
  `const_*`/`ret_*`/`ret_void` CIR to Intel 8086 bytes via
  `intel8086-encoder`, `run()` panicking per the emit-only contract.
- `compile(ctx, cir) -> Result<Vec<u8>, BackendError>` free function —
  the same shape `mos6502-backend`/`arm1-backend` expose for `lang-aot`
  to call directly.
- Trivial single-register (`AX`) "last const var" allocator.
- `ret_*`/`ret_void` lower to the genuine `HLT` (`0xF4`) hardware halt
  instruction.
- **The `terminated: bool` pattern**: the "already terminated?" check
  tracks an explicit boolean set only by a genuine `ret_*`/`ret_void`
  arm and reset on every subsequent `const_*`, rather than a trailing-
  byte-value comparison — avoiding a bug class fixed in four prior lanes
  of this campaign (Intel 8051, Intel 8080, MOS 6502, Zilog Z80), where
  a `const_*` immediate's encoded bytes could numerically collide with
  the halt opcode and suppress the real terminator.
- 19 tests in `tests/test_backend.rs`: byte-for-byte parity for the
  canonical `const 42; ret` program plus genuine execution in
  `intel8086-simulator` (through non-zero-`CS` segmented addressing) to
  confirm `AX == 42` and `halted == true`; immediate-range validation
  (`[0, 65535]`); the multi-const/unsupported-op fallthrough cases;
  `Backend` trait conformance; the `run()` emit-only panic; and —
  critically — `const_whose_encoded_high_byte_collides_with_halt_opcode_still_gets_real_terminator`,
  a regression test proving a `const_i64 v=0xF400` program (whose `MOV
  AX,0xF400` encoding's trailing byte is `0xF4`, identical to
  `HALT_BYTE`) with **no** `ret` still gets a real `HLT` appended — which
  a naive trailing-byte-comparison implementation would fail (it would
  wrongly conclude "already terminated" and emit a 3-byte program with
  no genuine halt instruction at all).

Ninth and **final** lane of the 9-architecture expansion following the
pattern documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
