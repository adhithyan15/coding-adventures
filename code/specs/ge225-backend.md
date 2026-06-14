# `ge225-backend` — GE-225 backend for jit-core / aot-core

**Status:** v0.1.0 — Phase 2 of the historical-arch backend migration.
**Migration plan:** [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md).
**Predecessor (deprecated in Phase 3):** [`iir-to-ge225`](iir-to-ge225.md).

## Why this crate exists

The original `iir-to-ge225` (and its A1–A4 siblings: `iir-to-riscv`,
`iir-to-intel8008`, `iir-to-armv7`, `iir-to-intel4004`) was hooked
at the wrong layer in the compiler stack: it consumed
**dynamically-typed IIR** directly and bypassed the
`jit_core::backend::Backend` trait that `aarch64-backend` and
`x86_64-backend` use to plug into **both** AOT and JIT in one shot.

This crate is the architectural fix for the GE-225 lane:

```text
IIR (dynamic-typed)
  ↓ aot_core::infer
  ↓ aot_core::specialise
CIR (typed: add_i64, cmp_lt_u32, neg_i16)
  ↓ Ge225Backend::compile_function     ← this crate
  ↓ ge225_encoder::encode_*
Vec<u8>  (20-bit GE-225 words, packed 3 bytes each)
  ├──→ aot_core::link → AOT executable bytes
  └──→ jit_core::JITCore  → JIT cache (in-process execution NOT supported — see below)
```

## Public surface (v0.1.0)

```rust
pub struct Ge225Backend;

impl Ge225Backend {
    pub fn new() -> Self;
}

impl jit_core::backend::Backend for Ge225Backend {
    fn name(&self) -> &str;
    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>>;
    fn compile_function(&self, ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>>;
    fn run(&self, _: &[u8], _: &[vm_core::value::Value]) -> vm_core::value::Value;
        // panics — see "Backend::run is intentionally not implemented" below
}

pub fn compile(
    ctx: &FunctionContext<'_>,
    cir: &[CIRInstr],
) -> Result<Vec<u8>, BackendError>;

pub enum BackendError { ... }   // 7 variants — see crate CHANGELOG
```

## Covered CIR ops

| Family | CIR mnemonics | Lowering shape |
|--------|---------------|----------------|
| Constants | `const_i8`/`const_i16`/`const_i32`/`const_i64`, `const_u8`/`u16`/`u32`/`u64`, `const_bool` | `(STA r_evict)?` + `LDA n` |
| Move | `mov_*` (any width) | `(STA r_evict_src)?` + `LD r_src` + `STA r_dest` |
| Arith | `add_*`, `sub_*` | (3-step eviction prep) + `LD r_lhs` + `ADD r_rhs` |
| Negate | `neg_*` | (evict) + `LDA 0` + `SUB r_src` |
| Cmp | `cmp_{lt,gt,eq,ne,le,ge}_*` (signed & unsigned) | SUB-then-test boolean materialisation |
| Control flow | `label`, `jmp`, `jmp_if_true`, `jmp_if_false` | `BR` / `BNZ` / `BZ` with per-function backpatching |
| Returns | `ret_*`, `ret_void` | `(LD r_var)?` + `HLT` |
| Builtin | `call_builtin` | no-op (with `LDA 0` placeholder if dest bound) |
| Cross-function call | `call` | **Err(UnsupportedOp)** — Phase 3 will add module-level relocations |
| Float, mul, div, shifts, bitwise, globals, type_assert, send, properties | — | **None** (graceful AOT/JIT fallback) |

## Why is `Backend::run` intentionally not implemented?

Per the [migration spec](HISTORICAL-ARCH-BACKEND-MIGRATION.md#what-about-backendrun--and-jit-in-general):

> **JIT support for the historical arches is explicitly
> best-effort, and "no working JIT" is an acceptable outcome for
> any individual arch.**

The GE-225 has no in-process simulator in this crate (the
in-workspace `ge225-simulator` crate exists but isn't wired here),
so `run` panics with a clear "emit-only; use a downstream
simulator" message.  The trait is satisfied so the backend plugs
cleanly into the `jit-core` registry; nobody should reach `run`.

A future increment could:
- Wire `run` to forward to `ge225-simulator`.
- Or skip `jit-core` registration entirely and only expose this
  backend through `aot-core`.

Both are fine — neither blocks the migration.

## Trivial-ROM regressions (byte-for-byte parity with iir-to-ge225 v0.9.0)

| Program | CIR | Bytes |
|---------|-----|-------|
| 6-byte ROM | `const_i64 v=5; ret_i64 v` | `[0x01, 0x00, 0x05, 0x00, 0x00, 0x00]` |
| 21-byte ROM | `const a=3; const b=4; add c, a, b; ret c` | `LDA 3 + STA r0 + LDA 4 + STA r1 + LD r0 + ADD r1 + HLT` |
| 15-byte ROM | `const v=5; neg w, v; ret w` | `LDA 5 + STA r0 + LDA 0 + SUB r0 + HLT` |
| 33-byte ROM | `const a=2; const b=5; cmp_lt c, a, b; ret c` | `LDA 2 + STA r0 + LDA 5 + STA r1 + LD r0 + SUB r1 + BMI 27 + LDA 0 + BR 30 + LDA 1 + HLT` |

## What's NOT in this crate (yet)

Out of scope for Phase 2; covered by Phase 3 (wiring) or future
increments:

- **Cross-function `call`** — needs module-level relocations via
  `aot_core::link`.  Phase 3 will add `compile_with_relocs` /
  `compile_with_globals` style entry points alongside `compile`.
- **Multi-function lowering / linking** — Phase 3.
- **lang-aot --emit=ge225 wiring** — currently goes through the
  legacy `iir-to-ge225` crate; Phase 3 re-routes it through
  `aot_core::link` + `ge225-backend`.
- **`iir-to-ge225` deprecation** — Phase 3 marks it
  `#[deprecated]`.

## Tests

24 unit tests in `tests/test_backend.rs`:

- `empty_cir_emits_halt`, `backend_name_is_ge225`,
  `backend_compile_returns_some_on_valid_input`,
  `backend_compile_returns_none_on_unsupported_op`,
  `backend_run_panics_per_spec` (the `#[should_panic]` regression).
- 4 trivial-ROM byte-for-byte pins (const+ret, add, sub, neg).
- 1 type-parametric test that runs the trivial ROM for every
  `const_*` variant: i8, u8, i16, u16, i32, u32, i64, u64.
- 4 comparison byte-shape tests (cmp_lt full byte trace, cmp_eq
  BZ position, cmp_gt operand-swap, cmp_le double-test).
- 3 control-flow tests (forward jmp backpatch, undefined-label
  error, jmp_if_true skip-LD).
- 3 call_builtin tests (no-dest, with-dest, cross-function `call`
  returns UnsupportedOp).
- 3 error-case tests (out-of-range immediate, undefined var in
  add, unsupported op).

All 24 pass without modification across cargo test runs.

## Non-goals (v0.1.0)

- No `run` execution (the panic is the contract).
- No cross-function `call` support — Phase 3.
- No registration with `jit-core::default_backends()` or similar —
  Phase 3 decides whether to register based on whether the
  registry's contract is satisfiable.
- No multiplication, division, bitwise, shift, or floating-point
  ops — out of scope for the historical-arch lane.
