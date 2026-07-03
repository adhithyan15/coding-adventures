# Changelog — riscv-encoder

## v0.1.0 — 2026-06-03 — initial carve-out

Phase 7 (FINAL lane) of the historical-arch backend migration.

Re-exports the `encode_*` helpers from `riscv-simulator::encoding`
— the canonical in-tree source of truth for RV32I bit layout —
and adds register-index constants and canonical word constants
(`RET_WORD = 0x0000_8067`) that `riscv-backend` consumes.

The original RV32I encoding logic was carved out of
`iir-to-riscv` v0.3.3 (which itself used `riscv-simulator::encoding`)
during the migration's correctness pass: a separate `riscv-encoder`
crate exists so that `riscv-backend` (Backend trait over CIR) has
a focused, IR-agnostic encoding dependency.  Mirror of the same
encoder/backend split used by `aarch64-encoder` + `aarch64-backend`
and `x86_64-encoder` + `x86_64-backend`.
