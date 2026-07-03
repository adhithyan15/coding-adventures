# ge225-encoder

Pure-Rust GE-225 instruction encoder.  Mirror of `aarch64-encoder`
and `x86_64-encoder` in shape and intent.

## What's in it

- Opcode-nibble constants (`LDA_OPCODE_NIBBLE`, …, `BMI_OPCODE_NIBBLE`)
- Canonical word constants (`HALT_WORD`, `RTS_WORD`)
- Capacity constants (`GP_REGISTER_COUNT`, `LDA_MAX_SIGNED`, …)
- `encode_*` helpers (one per opcode that takes an operand)
- `decode_word(...)` for symmetric round-tripping

No IR knowledge.  No `jit-core` dependency.  Consumed by
`ge225-backend` (Phase 2 of the migration) and re-exported by
`iir-to-ge225` for backwards compatibility.

## Why does this crate exist?

The historical-arch IIR-level crates (`iir-to-riscv`,
`iir-to-intel8008`, `iir-to-armv7`, `iir-to-intel4004`,
`iir-to-ge225`) were initially built at the wrong layer in the
compiler pipeline.  They consumed dynamically-typed IIR instead of
monomorphised CIR, and bypassed the `jit_core::backend::Backend`
trait that hooks both `aot-core` and `jit-core`.

This crate is Phase 1 of fixing that — see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md)
for the full plan.

## Status

- **v0.1.0** — initial carve-out from `iir-to-ge225` v0.9.0.  Same
  byte sequences, same opcode assignments, just at the right
  architectural layer.

## See also

- `ge225-backend` (Phase 2) — consumes CIR via the `Backend` trait.
- `aarch64-encoder` — the model this crate follows.
- `code/specs/iir-to-ge225.md` — the original (now-deprecated) IIR-level spec.
