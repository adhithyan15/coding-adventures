# Changelog — mos6502-backend

## [0.1.1] - 2026-08-17

### Fixed

- The defensive "already terminated?" check in `compile_to_bytes` compared
  the trailing emitted byte value against `HALT_BYTE` (`0x00`), instead of
  tracking whether a real `BRK` was actually emitted. Since `0` is also a
  valid 8-bit `LDA` immediate, `const 0` with no following `ret_*` produced
  a trailing byte identical to `HALT_BYTE`, fooling the check into skipping
  the real terminator. (On real 6502 hardware this specific case still
  halted at runtime by coincidence — `BRK`'s opcode is also `0x00`, so
  falling into zero-filled memory hit a real `BRK` anyway — but that
  coincidence wasn't load-bearing by design and doesn't generalise to
  memory that isn't zero-filled.) Now tracks an explicit `terminated: bool`,
  set only when a real `BRK` is pushed by a `ret_*`/`ret_void` arm. Same bug
  class found and fixed in the Intel 8051 and Intel 8080 lanes of the
  9-architecture expansion this crate is part of.

## [0.1.0] - 2026-08-17

### Added

- `Mos6502Backend` implementing `jit_core::backend::Backend` for the MOS
  6502.  Minimal-viable scope: `const_*`/`ret_*`/`ret_void` only, via the
  "last const var" trivial single-accumulator allocator (mirrors
  `mips-r2000-backend`/`arm1-backend`/`armv7-backend`/`intel8008-backend`).
- `ret_*`/`ret_void` lower to `BRK` — the pre-existing HALT convention
  `mos6502-simulator` already ports from the Python original, not a new
  pseudo-halt invented for this lane (see crate module doc for the full
  rationale versus the KIL/JAM and self-jump alternatives considered and
  rejected).
- `Backend::run` panics with `"mos6502 backend is emit-only; load bytes
  into mos6502-simulator to execute"` per the migration spec.
- `BackendError` variants: `UnsupportedOp`, `InvalidOperand`,
  `UndefinedVariable` (reserved for a future allocator), and
  `ImmediateOutOfRange` (the 8-bit unsigned `LDA #imm` range `[0, 255]`).
- 14 unit/integration tests in `tests/test_backend.rs` pinning the
  canonical `LDA #42; BRK` = `[0xA9, 0x2A, 0x00]` byte sequence for the
  IIR `const 42; ret` program, plus edge cases and a genuine execution
  check against `mos6502-simulator` (not just a hand-derived byte array).

Fifth lane of the 9-architecture expansion following the pattern
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
