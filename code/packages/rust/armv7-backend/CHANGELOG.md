# Changelog — armv7-backend

## v0.1.0 — 2026-06-03 — Phase 5 of historical-arch backend migration

Initial release.  Minimal viable Backend trait impl for ARMv7-A
(A32).  Covers `const_*` + `ret_*` — enough to pass the existing
lang-aot ARMv7 e2e smoke test byte-for-byte
(`MOV r0, #42; BX LR`).

### Public API

- `pub struct Armv7Backend` with `impl Backend`:
  - `name() -> "armv7"`
  - `compile(&[CIRInstr]) -> Option<Vec<u8>>` (little-endian flattened)
  - `compile_function(&FunctionContext, &[CIRInstr]) -> Option<Vec<u8>>`
  - `run(_, _)` panics with "armv7 backend is emit-only"
- `pub fn compile(ctx, cir) -> Result<Vec<u8>, BackendError>`
- `pub enum BackendError` — 4 diagnostic variants.

### Covered CIR ops

- `const_*` (8-bit immediate range) → `MOV r0, #imm`
- `ret_*`, `ret_void` → `BX LR`
- Anything else → `None` (graceful AOT/JIT fallback)

### What's NOT in this PR

Full op coverage (add/sub/and/or/xor/adc/sbb/cmp/branches/calls)
from the deprecated `iir-to-armv7` v0.4.6 is intentionally out of
scope for the minimal-viable migration.  Future increments can
add them.

### Tests

10 unit tests pin the canonical `MOV r0, #42; BX LR` byte
sequence and edge cases (zero, max 8-bit, immediate overflow,
bool, multi-var fallthrough, unsupported op).
