# Changelog — ge225-backend

## v0.1.0 — 2026-06-03 — Phase 2 of historical-arch backend migration

Initial release.  Implements `jit_core::backend::Backend` for the
GE-225, consuming monomorphised CIR.  Mirror of `aarch64-backend`
/ `x86_64-backend` in shape and intent.

### Added

- `pub struct Ge225Backend` with `impl Backend`:
  - `name() -> "ge225"`.
  - `compile(&[CIRInstr]) -> Option<Vec<u8>>`.
  - `compile_function(&FunctionContext, &[CIRInstr]) -> Option<Vec<u8>>`.
  - `run(_, _) -> Value` panics with `"ge225 backend is emit-only;
    load bytes into a ge225 simulator to execute"`.  Per the
    migration spec, JIT support is best-effort for historical
    arches; this satisfies the trait so the backend can plug into
    the `jit-core` registry, but `run` is intentionally not
    implemented.
- `pub fn compile(ctx, cir) -> Result<Vec<u8>, BackendError>` — the
  direct AOT entry point (same shape as `aarch64_backend::compile`).
- `pub enum BackendError` — structured diagnostics: `UnsupportedOp`,
  `InvalidOperand`, `UndefinedVariable`, `ImmediateOutOfRange`,
  `OutOfRegisters`, `UndefinedLabel`, `BranchTargetOutOfRange`.

### Covered CIR ops

- `const_*` (every int type + bool), `mov_*`, `add_*`, `sub_*`,
  `neg_*`, `cmp_{lt,gt,eq,ne,le,ge}_*`, `label`, `jmp`,
  `jmp_if_true`, `jmp_if_false`, `ret_*`, `ret_void`,
  `call_builtin` (no-op).
- `call` (cross-function) returns `Err(UnsupportedOp)` until
  Phase 3 adds module-level relocation support via `aot-core`.

### Byte-for-byte parity with iir-to-ge225 v0.9.0

The 24 unit tests pin the same trivial-ROM byte sequences (6-byte
const+ret, 21-byte add/sub, 33-byte cmp_lt, 15-byte neg), just
built from CIR instead of IIR.

### Source

Lowering algorithm carved from `iir-to-ge225` v0.9.0 and adapted
to consume `CIRInstr` (typed) instead of `IIRInstr` (dynamic).
The CIR ops dispatch via prefix-strip (`op.strip_prefix("const_")`
etc.) — much cleaner than the IIR-level approach that needed
explicit per-type arms.

### Reference

- Spec: `code/specs/ge225-backend.md`
- Migration plan: `code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md`
