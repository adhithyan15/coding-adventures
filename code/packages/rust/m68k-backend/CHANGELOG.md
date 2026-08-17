# Changelog — m68k-backend

## [0.1.0] - 2026-08-17

### Added

- `M68kBackend` implementing `jit_core::backend::Backend` — minimal
  viable scope (`const_*`/`ret_*`/`ret_void` only via the "last const
  var" trivial scheme), targeting data register `D0`.
- `compile(ctx, cir) -> Result<Vec<u8>, BackendError>` free function and
  the `Backend::compile`/`compile_function` trait methods (which agree
  byte-for-byte — verified by a dedicated test).
- `ret_*`/`ret_void` lower to `TRAP #15` — the pre-existing HALT
  convention `m68k-simulator`'s own crate doc derives from the Python
  original's test suite (`_stop()`, used 100+ times), not `STOP #imm`
  (which appears once, in a doctest) and not a newly-invented
  pseudo-halt.
- `const_*` accepts the full 32-bit immediate range
  (`[i32::MIN, u32::MAX]`, matching `MOVE.L`'s 32-bit immediate field) —
  wider than ARM1's unrotated 8-bit `MOV`-immediate or the 6502's 8-bit
  `LDA`-immediate, since the 68000's `MOVE.L #imm, Dn` carries a full
  longword.
- **Security fix applied proactively**: the defensive "already
  terminated?" check uses an explicit `terminated: bool` flag (set
  `true` only when a real `ret_*`/`ret_void` arm pushes
  `encode_trap15()`, reset to `false` on every subsequent `const_*`) —
  NOT a trailing-byte/word-value comparison against the halt encoding.
  A prior lane (Intel 8051, commit `19e360d`) shipped exactly that
  unsound byte-comparison pattern and had to fix it after security
  review: `TRAP #15`'s low byte (`0x4F`) is also reachable as the low
  byte of a `MOVE.L #imm, D0` immediate (`const_i64 79`), which would
  have fooled a byte-value check into skipping the real terminator. A
  regression test
  (`const_ending_in_halt_low_byte_with_no_ret_still_appends_real_halt`)
  proves both the byte sequence and that the simulator actually halts
  for exactly this case.
- 17 unit/integration tests in `tests/test_backend.rs`, including one
  that loads the compiled bytes into `m68k-simulator`, runs them, and
  asserts `D0 == 42` and `halted == true` — byte-for-byte parity plus
  genuine execution, not just a hand-derived byte array.
- Wired into `lang-aot` as `--emit=m68k` (aliases `68000`, `mc68000`,
  `motorola68000`) — `compile_file_to_m68k_bin`, the
  `LangAotError::M68kBackendError` variant, and CLI help text. Manually
  verified end-to-end: `(define (main) 42)` compiled through the full
  IIR → CIR → `m68k-backend` pipeline produces the exact canonical byte
  sequence `[0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A, 0x4E, 0x4F, ...]`.

Eighth lane of the 9-architecture expansion following the pattern
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
