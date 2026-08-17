# `mos6502-backend`

MOS 6502 implementation of the `jit_core::backend::Backend` trait. Mirror
of `mips-r2000-backend` / `arm1-backend` / `armv7-backend` /
`intel8008-backend` (the *minimal viable* shape). Lowers `Vec<CIRInstr>`
(typed, monomorphised CIR) to `Vec<u8>` of MOS 6502 machine code via
[`mos6502-encoder`](../mos6502-encoder).

Fifth lane of the 9-architecture expansion — see
[`code/specs/mos6502-backend.md`](../../../specs/mos6502-backend.md) and
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## Scope (v0.1.0 — minimal viable)

| CIR op | Lowering |
|--------|----------|
| `const_*` (8-bit literal, `[0, 255]`) | `LDA #imm` |
| `ret_*`, `ret_void` | `BRK` |
| Anything else | `None` from `Backend::compile` |

No real register allocator — a trivial "last const var" scheme tracks
which single variable the most recent `const_*` wrote into the
accumulator; `ret_*` only succeeds if it returns exactly that variable.

## Why `BRK`, not a pseudo-halt?

Unlike ARM1 (no real halt instruction — `arm1-backend` invents a
pseudo-halt via `SWI`), the MOS 6502 already has `BRK`, and
`mos6502-simulator` already treats it as HALT — a convention ported
directly from the pre-existing Python simulator, not invented for this
lane. See the crate's module doc for the full derivation.

## `Backend::run`

Panics with `"mos6502 backend is emit-only; load bytes into
mos6502-simulator to execute"` — emit-only per the migration spec.

## Tests

14 unit/integration tests in `tests/test_backend.rs` (mirroring
`mips-r2000-backend`'s/`arm1-backend`'s test shape) pin the canonical
byte sequence and edge cases (zero, 8-bit range boundaries, bool,
multi-var fallthrough, unsupported op, empty CIR, `ret_void`,
`Backend::run` panics, `Backend::compile` vs the free `compile` function
agree). One test additionally loads the compiled bytes into
`mos6502-simulator`, runs it, and asserts `A == 42` and `halted == true`.
