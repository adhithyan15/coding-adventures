# intel4004-backend

Intel 4004 backend for `jit-core` / `aot-core`.  Mirror of
`ge225-backend` / `aarch64-backend` for the **world's first
commercial microprocessor** (1971).

## What this crate is

- `Intel4004Backend` zero-sized struct implementing `Backend`.
- `compile` for function-local lowering and `compile_with_global_slots` for
  module-wide AOT global addressing.
- `pub enum BackendError` for diagnostic reporting.

## Covered CIR ops (v0.2.0)

| Family | Status |
|--------|--------|
| `const_*`, `ret_*`, `ret_void`, `mov_*` | ✓ |
| `global_store`, `global_load` | 320-nibble RAM via DCL/FIM/SRC + WRM/RDM/WR0..3/RD0..3 |
| Anything else | `None` (graceful fallback) |

## See also

- [`intel4004-encoder`](../intel4004-encoder)
- [`iir-to-intel4004`](../iir-to-intel4004) — deprecated predecessor
- [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md)
