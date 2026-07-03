# `intel4004-backend` — Intel 4004 backend for jit-core / aot-core

**Status:** v0.1.0 — Phase 4 of the historical-arch backend migration.
**Migration plan:** [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md).
**Predecessor (deprecated):** `iir-to-intel4004` v0.4.0.

Mirror of [`ge225-backend`](ge225-backend.md) — same shape, same
`Backend` trait, just for a different historical arch (the 1971
Intel 4004, the world's first commercial microprocessor).

## Public surface (v0.1.0)

```rust
pub struct Intel4004Backend;

impl jit_core::backend::Backend for Intel4004Backend {
    fn name(&self) -> &str { "intel4004" }
    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>>;
    fn compile_function(&self, ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>>;
    fn run(&self, _: &[u8], _: &[Value]) -> Value;
        // panics — emit-only target
}

pub fn compile(ctx, cir) -> Result<Vec<u8>, BackendError>;
pub enum BackendError { /* 5 variants */ }
```

## CIR ops covered

| Family | CIR mnemonics | Status |
|--------|---------------|--------|
| Constants | `const_*` (every int type + bool) | ✓ |
| Move | `mov_*` | ✓ |
| Returns | `ret_*`, `ret_void` | ✓ |
| Anything else | — | returns `None` |

Same op set as `iir-to-intel4004` v0.3.0.  All trivial-ROM byte
sequences from the lang-aot intel4004 e2e smoke test
(`[0xD5, 0x40, 0x00]` for `const_i64 v=5; ret_i64 v`) reproduce
byte-for-byte through the new pipeline.

## Why is `Backend::run` not implemented?

Per the migration spec, the historical-arch backends are
**emit-only**.  Bytes go to a downstream simulator
(`intel4004-simulator`), the in-tree `intel-4004-assembler` for
round-trip disassembly, or an EPROM burner for a 4004 dev board.
`Backend::run` panics with a clear message.

## Tests (17 unit tests)

- 3 Backend trait basics (`name()`, `compile` returns `Some`/`None`,
  `run` panics).
- 1 empty-CIR HALT_LOOP fallback.
- 5 trivial-ROM regressions: const 0/5/15/-1/bool=true → exact bytes.
- 1 out-of-range immediate → `ImmediateOutOfRange`.
- 1 two-const eviction trace (5 bytes).
- 1 ret-of-evicted-var-emits-LD trace (6 bytes).
- 1 ret_void-only → exact 2-byte HALT_LOOP.
- 1 undefined-var-in-ret → `UndefinedVariable`.
- 1 `mov_i64` byte trace (7 bytes).
- 1 unsupported-op (`add_i64`) → `UnsupportedOp`.

All pass on first try.

## Non-goals (v0.1.0)

- No `run` execution (the panic is the contract).
- No cross-function `call` support.
- No `jit-core` registry registration — JIT is best-effort per
  GUIDING CONSTRAINT.
- No arithmetic, comparison, or branch ops — out of scope for the
  current op-set parity.  Future increments can add them as
  needed.
