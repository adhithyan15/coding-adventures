# intel4004-backend

Intel 4004 backend for `jit-core` / `aot-core`.  Mirror of
`ge225-backend` / `aarch64-backend` for the **world's first
commercial microprocessor** (1971).

## What this crate is

- `Intel4004Backend` zero-sized struct implementing `Backend`.
- `pub fn compile(ctx, cir) -> Result<Vec<u8>, BackendError>` for
  direct AOT use.
- `pub enum BackendError` for diagnostic reporting.

## Covered CIR ops (v0.1.0)

| Family | Status |
|--------|--------|
| `const_*`, `ret_*`, `ret_void`, `mov_*` | ✓ |
| Anything else | `None` (graceful fallback) |

Same op set as the deprecated `iir-to-intel4004` v0.3.0 — just at
the right architectural layer.

## See also

- [`intel4004-encoder`](../intel4004-encoder)
- [`iir-to-intel4004`](../iir-to-intel4004) — deprecated predecessor
- [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md)
