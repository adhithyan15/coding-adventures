# Changelog — riscv-backend

## Unreleased

- Added executable RV32I scalar lowering for typed CIR constants, arithmetic,
  bitwise operations, shifts, unary operations, and signed/unsigned comparisons.
- Added `run_binary`, which executes a flat function binary on the in-tree
  `riscv-simulator` and reports its `a0` return value and instruction count.
- Preserved the canonical Twig `42` byte sequence while proving that the
  emitted bytes execute and return `42` in the simulator.
- Added two-pass control-flow lowering for CIR `label`, `jmp`,
  `jmp_if_true`, and `jmp_if_false`, including a Nib conditional
  source-to-simulator fixture.
- Permit `i64`/`u64` comparisons only for constant values proven to fit in one
  RV32 register, keeping arbitrary wide values unsupported.
- Added low/high register-pair lowering for full-width `i64`/`u64` constants,
  addition, subtraction, and returns; `RunResult` now exposes the returned
  `a1` high word for simulator assertions.
- Added pair-aware signed and unsigned `eq`, `ne`, `lt`, `le`, `gt`, and `ge`
  comparisons, including a numeric Nib conditional simulator fixture.
- Added pair-aware `and`, `or`, `xor`, and `not` lowering for `i64`/`u64`,
  including a Nib bitwise source-to-simulator fixture.

## v0.1.0 — 2026-06-03 — Phase 7 (FINAL lane) of historical-arch backend migration

Initial release.  Minimal viable Backend trait impl over CIR.

Covers `const_*` + `ret_*` + `ret_void` — enough to keep the
existing lang-aot RV32I e2e smoke tests passing byte-for-byte:

* Twig `42` → `[addi t0, x0, 42; addi a0, t0, 0; jalr x0, x1, 0]`
  = `[0x02A0_0293, 0x0002_8513, 0x0000_8067]` as little-endian
  bytes = `[0x93, 0x02, 0xA0, 0x02, 0x13, 0x85, 0x02, 0x00,
  0x67, 0x80, 0x00, 0x00]`.
* BASIC `PRINT 42` → returns `UnsupportedOp("call_builtin_print_i64")`,
  which the e2e test treats as an expected gap (skipped with
  `eprintln!`).  Phase 8+ can port the `ecall print_i64` lowering
  from `iir-to-riscv` v0.3.3 if richer RV32I coverage is wanted.

11 unit tests pin every byte sequence.

This crate closes the historical-arch backend migration: every
arch backend now consumes typed CIR via the `Backend` trait
rather than dynamic IIR via a bespoke entry point.
