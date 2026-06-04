# riscv-backend

RV32I backend for `jit-core` / `aot-core`.  Phase 7 (the FINAL
lane) of the historical-arch backend migration — see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## What's inside

* `Backend` trait impl (`Riscv32Backend`) plugging RV32I into
  the same registry as `aarch64-backend` and `x86_64-backend`.
* CIR → `Vec<u8>` lowering via `riscv-encoder`.
* Output format: little-endian 32-bit instruction words.
  Per-function bytes can be concatenated directly — `lang-aot`
  flattens them straight into a `.bin`.

## v0.1.0 scope — minimal viable

Same scope as `intel8008-backend` v0.1.0 and `armv7-backend`
v0.1.0: just enough to keep the existing lang-aot RV32I e2e smoke
tests passing byte-for-byte.

| CIR family | Status |
|------------|--------|
| `const_*` (12-bit signed immediate, single-var case) | ✓ → `addi rd, x0, n` |
| `ret_*` | ✓ → `addi a0, rs, 0` + `jalr x0, x1, 0` |
| `ret_void` | ✓ → `jalr x0, x1, 0` |
| Anything else | returns `None` (AOT reports the gap; JIT stays on the interpreter tier) |

Per the GUIDING CONSTRAINT, the architectural correctness win
(IIR → CIR via Backend trait) is what Phase 7 delivers.  Future
increments to `riscv-backend` can port the richer op coverage
that `iir-to-riscv` v0.3.3 had (add/sub/cmp/branches/calls/ecall
print_i64).

## Why this is the FINAL lane

The RV32I backend was the **original** target the A1+ cascade
shipped at the wrong layer (IIR-direct) in May 2026.  It's the
mistake that started the whole pattern Phases 1–6 corrected; the
migration spec docs (`HISTORICAL-ARCH-BACKEND-MIGRATION.md`)
describe RV32I as "last (the original mistake from A1+ that
started this whole pattern)".  Phase 7 closes the loop.
