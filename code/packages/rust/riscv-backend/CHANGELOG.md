# Changelog — riscv-backend

## Unreleased

- Reworked mixed-width temporary allocation around three non-overlapping RV32
  register-pair slots. Scalars and full-width values can now interleave without
  constructing overlapping pairs; dead occupants are reclaimed and live scalar
  or pair values spill through the existing frame/reload paths. Added a direct
  simulator test that keeps interleaved scalar and 64-bit values live under
  register pressure.
- Added aligned stack spilling and pair-aware reloads for live 64-bit values in
  wide arithmetic and bitwise lowering, with a simulator fixture that exceeds
  the three-pair register pool.
- Added pair-spill reloads for wide shifts, including a parameterized simulator
  fixture that shifts a stack-resident full-width value.
- Added dedicated pair-spill reloads for signed and unsigned restoring division,
  with simulator coverage for stack-resident dividends and divisors.
- Added pair-spill reloads for wide comparisons and reserved a scalar register
  for mixed-width pressure when all pair temporaries are live.
- Added mixed-width pair allocation: a wide destination can spill live scalar
  words that occupy one of the three pair slots, then those scalars reload from
  the frame when consumed.
- Added scalar register spilling: live values evict to aligned `sp`-relative
  stack slots, reload through dedicated operand registers, and restore the
  frame on every return. The simulator fixture now executes seven live scalar
  values and returns their sum instead of failing after six temporaries.
- Added scalar RV32M `div` / `divu` / `rem` / `remu` lowering and pair-aware
  restoring `div_u64` / `mod_u64` lowering, including cross-word and
  zero-divisor simulator fixtures.
- Added signed pair `div_i64` / `mod_i64` lowering by normalizing magnitudes
  around the restoring loop, with simulator coverage for sign combinations,
  `i64::MIN / -1`, and zero divisors.
- Track remaining CIR value uses and let a wide right shift overwrite a dead
  left-hand register pair. This enables chained pair shifts within the
  six-register starter allocator; general spilling remains a later allocator
  milestone.

## v0.2.0 — 2026-08-13 — executable scalar core, plus a float refusal that says why

### Floating point now refuses with a reason instead of a shrug

`riscv-backend` reported `UnsupportedType("f64")` — "unsupported RV32I scalar
type" — for a double.  That reads like a lowering nobody has written yet, and
it is not: RV32I is the RISC-V **base integer** ISA.  Its entire architectural
state is 32 *integer* registers; there is no `f0`..`f31` bank and no `fadd.d`.
Single and double precision are separate optional standard extensions (`F` →
RV32F, `D` → RV32D).  An `f64` on RV32I is a value the target cannot hold, and
that has a completely different fix from a missing op: retarget the module, or
decompose the double into integer soft-float sequences before this backend
sees it.

So floats get their own error, `BackendError::UnsupportedFloat { site, ty }`.
It names *where* the float appeared (`op "const_f64"`, `parameter "mag"`) and
spells out that RV32I has no floating-point registers, naming RV32F/RV32D as
the extensions that would carry it.  Non-float types keep the generic
`UnsupportedType`, so the two cases stay distinguishable.

This surfaced through Dartmouth BASIC.  BASIC has exactly one numeric type and
it is REAL, so after the BA7 floating-point conversion even `10 PRINT 42` is
`const_f64 42.0` feeding `__basic_print_real(x : f64)`.  The `lang-aot`
BASIC → RV32I smoke test had been written to tolerate "lowering gap" messages
by substring, and its list did not include this one — so a genuine, correct
refusal read as a test failure.  The refusal is right; the expectation was
wrong.  Note what was *not* done: no truncation of `42.0` to an integer to
make bytes appear.  A loud refusal beats a silent wrong answer.

- Added `BackendError::UnsupportedFloat { site, ty }` and routed every type
  gate (parameters, `const_*`, `ret_*`, arithmetic, bitwise, unary, and
  comparisons) through a shared `unsupported_type_error` helper that picks the
  float refusal for `f16`/`f32`/`f64`/`f128` and the generic one otherwise.
- Type gates now receive the CIR op name so the message points at the exact
  instruction rather than "somewhere in this function".
- Added tests for a float constant, float arithmetic, a float comparison, a
  float return, an `f64` parameter, and a non-float type keeping the generic
  error.

### Executable scalar core (previously unreleased)

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
- Added pair-aware left, logical-right, and arithmetic-right shifts with
  correct zero, cross-word, and `>= 64` count handling.
- Added pair-aware signed and unsigned multiplication using the RV32M `mul`
  and `mulhu` instructions, including a source-level Nib simulator fixture.

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
