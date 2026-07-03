# ge225-backend

GE-225 backend for `jit-core` / `aot-core`.  Mirror of
`aarch64-backend` / `x86_64-backend` for the 1959 General Electric
mainframe where Dartmouth BASIC was designed in 1964.

## What this crate is

- A `Ge225Backend` zero-sized struct that implements
  `jit_core::backend::Backend`.
- A public `compile(ctx, cir) -> Result<Vec<u8>, BackendError>`
  function for direct AOT use (the same shape as
  `aarch64_backend::compile`).
- A structured `BackendError` enum for diagnostic reporting.

## What it does

Lowers a `Vec<CIRInstr>` (typed, monomorphised compiler-IR
produced by `aot_core::specialise`) into 20-bit GE-225 instruction
words packed 3 bytes each.  Uses `ge225-encoder` for the actual
byte construction.

## Backed CIR ops (Phase 2, v0.1.0)

| Family | CIR mnemonics | Status |
|--------|---------------|--------|
| Constants | `const_i8` … `const_i64`, `const_u8` … `const_u64`, `const_bool` | ✓ |
| Move | `mov_*` | ✓ |
| Add / Sub | `add_*`, `sub_*` | ✓ |
| Neg | `neg_*` | ✓ |
| Compare | `cmp_{lt,gt,eq,ne,le,ge}_*` | ✓ |
| Control flow | `label`, `jmp`, `jmp_if_true`, `jmp_if_false` | ✓ |
| Returns | `ret_*`, `ret_void` | ✓ |
| Built-in call | `call_builtin` | ✓ (no-op) |
| Cross-function call | `call` | Returns `None` until Phase 3 wires module-level relocations |
| Mul / Div / Shifts / Bitwise / Float / Globals / Type guards | — | Returns `None` (graceful AOT/JIT fallback) |

## Why is `Backend::run` not implemented?

The GE-225 has no in-process simulator in this crate.  Per the
[migration spec](../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md),
the historical-arch backends are **emit-only** — bytes go to a
downstream simulator (`ge225-simulator`) or a custom decoder.
`Backend::run` panics with a clear message.

## Byte-for-byte parity with `iir-to-ge225` v0.9.0

The 24 unit tests in `tests/test_backend.rs` pin the same trivial-
ROM byte sequences `iir-to-ge225` did, just built from CIR
(`const_i64`/`add_i64`/etc.) instead of IIR (`const`/`add`):

| Program | Bytes |
|---------|-------|
| `const v=N; ret v` | 6 (LDA + HLT) |
| `const a; const b; add c, a, b; ret c` | 21 |
| `const a; const b; cmp_lt c, a, b; ret c` | 33 |
| `const v; neg w, v; ret w` | 15 |

## See also

- [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md) — phase plan.
- [`ge225-encoder`](../ge225-encoder) — pure byte-encoding tables this crate consumes.
- [`iir-to-ge225`](../iir-to-ge225) — the older IIR-level crate this PR migrates away from.  Will be deprecated in Phase 3.
- [`aarch64-backend`](../aarch64-backend) — the architectural model this crate follows.
