# Changelog — intel4004-backend

## v0.1.0 — 2026-06-03 — Phase 4 of historical-arch backend migration

Initial release.  Implements `jit_core::backend::Backend` for the
Intel 4004, consuming monomorphised CIR.  Mirror of `ge225-backend`
/ `aarch64-backend` in shape and intent.

### Added

- `Intel4004Backend` struct with `impl Backend`:
  - `name() -> "intel4004"`
  - `compile(&[CIRInstr]) -> Option<Vec<u8>>`
  - `compile_function(&FunctionContext, &[CIRInstr]) -> Option<Vec<u8>>`
  - `run(_, _)` panics with `"intel4004 backend is emit-only;
    load bytes into an Intel 4004 simulator to execute"`.
- `pub fn compile(ctx, cir) -> Result<Vec<u8>, BackendError>` —
  direct AOT entry point.
- `pub enum BackendError` with 5 diagnostic variants.

### Covered CIR ops

`const_*`, `ret_*`, `ret_void`, `mov_*`.  Anything else returns
`None`.

### Byte-for-byte parity with iir-to-intel4004 v0.3.0

15 tests pin the same byte sequences (3-byte canonical ROM
`[0xD5, 0x40, 0x00]` for `const v=5; ret v`, etc.).

### Reference

- Spec: `code/specs/intel4004-backend.md`
- Migration plan: `code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md`
