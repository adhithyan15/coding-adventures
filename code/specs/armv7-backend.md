# `armv7-backend` — ARMv7-A (A32) backend for jit-core / aot-core

**Status:** v0.1.0 — Phase 5 of the historical-arch backend migration.
**Predecessor (deprecated):** `iir-to-armv7` v0.5.0.

Mirror of `ge225-backend` / `intel4004-backend` / `aarch64-backend` for ARMv7-A.

## v0.1.0 scope — minimal viable migration

Covers the AAPCS32-conforming trivial-ROM case (`const_*` immediate + `ret_*` → `BX LR`).  Enough to keep the existing lang-aot ARMv7 e2e smoke test (`MOV r0, #42; BX LR`) passing byte-for-byte through the new pipeline.

| CIR family | Status |
|------------|--------|
| `const_*` (8-bit immediate) | ✓ → `MOV r0, #imm` |
| `ret_*`, `ret_void` | ✓ → `BX LR` |
| Anything else | returns `None` (graceful AOT/JIT fallback) |

## What's intentionally NOT ported

The deprecated `iir-to-armv7` v0.4.6 had richer coverage (add/sub/and/or/xor/adc/sbb/cmp/cmp_*/branches/calls).  Porting all of that would balloon this PR; future increments to `armv7-backend` can add them.

Per the migration spec (GUIDING CONSTRAINT: JIT is best-effort), the architectural correctness win is delivered as soon as the AOT path is wired, regardless of op-set parity.  Larger CIR programs currently fall through to `Backend::compile` returning `None`, and the AOT pipeline reports a graceful compile failure — same behaviour as any other unsupported op.

## Why is `Backend::run` not implemented?

Emit-only target per the migration spec.  Bytes go to `arm-simulator`, `qemu-arm`, or `objcopy` + a phone-class Linux linker for a Cortex-A class SoC.  `Backend::run` panics with a clear message.

## Tests

10 unit tests pin canonical bytes:
- `MOV r0, #42; BX LR` = `[0x2A, 0x00, 0xA0, 0xE3, 0x1E, 0xFF, 0x2F, 0xE1]` (the e2e regression invariant)
- Edge cases: zero, max 8-bit (255), overflow (256 errors), bool
- Multi-var-ret falls through to `UnsupportedOp` (until a real allocator lands)
- Unsupported op (`add_i64`) returns `None`
- `Backend::run` panics with the documented message

## See also

- `armv7-encoder` — pure encoding tables
- `iir-to-armv7` — deprecated predecessor (v0.5.0)
- [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)
