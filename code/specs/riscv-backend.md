# `riscv-backend` spec

> **Status:** v0.1.0 — Phase 7 (FINAL lane) of the historical-arch
> backend migration, 2026-06-03.

## Purpose

RV32I implementation of the `jit_core::backend::Backend` trait.
Mirror of `aarch64-backend` / `x86_64-backend` / `ge225-backend` /
`intel4004-backend` / `armv7-backend` / `intel8008-backend`.

Lowers `Vec<CIRInstr>` (typed, monomorphised) to a little-endian
`Vec<u8>` of RV32I machine code via `riscv-encoder`.

## Why this crate exists (Phase 7 of the historical-arch backend migration)

RV32I was the **original** historical-arch lane (the A1+ cascade
from May 2026).  It shipped at the wrong layer — `iir-to-riscv`
consumed dynamic IIR (`add a b` with unknown operand types) and
emitted bytes directly.  That bypassed `aot_core::infer` and the
shared `Backend` trait — every other arch backend would have to
redo type inference and re-implement the registry hookup.

Phase 7 corrects that.  `riscv-backend` consumes CIR (typed:
`add_i64`, `cmp_lt_u32`) and plugs into the same registry as
`aarch64-backend` / `x86_64-backend`.  The architectural
correctness win — every arch backend uses the same
`aot_core::infer` + `aot_core::specialise` + `Backend::compile`
pipeline — is delivered as Phase 7 lands, closing the historical
migration.

## v0.1.0 scope — minimal viable

Per the GUIDING CONSTRAINT of the migration spec, "minimal-viable
backend op coverage (only what the e2e test pins) is acceptable".
v0.1.0 ships the smallest op set needed to keep the existing
lang-aot RV32I e2e smoke test passing byte-for-byte:

| CIR op family | Lowering |
|---------------|----------|
| `const_*` (12-bit signed immediate) | `addi rd, x0, n` |
| `ret_*` (value matches the last `const_*` dest) | `addi a0, src_reg, 0` + `jalr x0, x1, 0` |
| `ret_void` | `jalr x0, x1, 0` |
| Empty CIR body | `jalr x0, x1, 0` |

Anything else returns `BackendError::UnsupportedOp(op)` from the
inherent `compile()`, or `None` from `Backend::compile` (the trait
method).  AOT treats `None` as a per-function compile failure
(same error path as any backend); JIT treats it as "stay on the
interpreter tier".

## Wire format

Each instruction is a 32-bit RV32I word, flattened to little-
endian bytes per the RISC-V spec.  Per-function byte streams can
be concatenated directly — `lang-aot` writes them straight to disk
as a flat `.bin`.

## Pinned byte sequences

| Program | CIR | Emitted bytes |
|---------|-----|---------------|
| Twig `42` | `const_i64 v=42; ret_i64 v` | `[0x93, 0x02, 0xA0, 0x02, 0x13, 0x85, 0x02, 0x00, 0x67, 0x80, 0x00, 0x00]` |
| `ret_void` only | `ret_void` | `[0x67, 0x80, 0x00, 0x00]` |
| Empty CIR | (none) | `[0x67, 0x80, 0x00, 0x00]` |
| BASIC `PRINT 42` | `… call_builtin_print_i64 … ret_void` | (returns `UnsupportedOp` — test treats as expected gap) |

## Backend trait surface

| Trait method | Behaviour |
|--------------|-----------|
| `name()` | returns `"riscv32"` |
| `compile(ir)` | returns `Some(bytes)` for supported CIR ops; `None` otherwise |
| `compile_function(ctx, ir)` | identical to `compile(ir)` — v0.1.0 doesn't yet use the `FunctionContext` |
| `run(binary, args)` | **panics** with `"riscv32 backend is emit-only…"` — per the GUIDING CONSTRAINT, JIT execution is best-effort and a future increment can wire this to `riscv-simulator::Simulator::run` |

## Error variants

| `BackendError` variant | Trigger |
|------------------------|---------|
| `UnsupportedOp(String)` | CIR op outside v0.1.0's coverage |
| `InvalidOperand(String)` | `ret_*` srcs[0] isn't a `Var`, `const_*` missing a dest, etc. |
| `UndefinedVariable(String)` | `ret_*` references a name never seen via `const_*` |
| `ImmediateOutOfRange(i64)` | `const_*` value outside `[-2048, 2047]` (the 12-bit signed `addi` window) |
| `OutOfRegisters` | Linear allocator exhausted the 7-temp pool (`TEMP_REGISTERS`) |

## Tests (11 byte-pinned unit tests)

* Empty CIR emits the canonical 4-byte ret.
* Backend name is `"riscv32"`.
* `Backend::run` panics with the documented message.
* Twig `42` canonical produces the 12-byte sequence above.
* `const_i64 0` lowers to `addi t0, x0, 0`.
* `const_bool true` acts as immediate-1.
* Negative immediates in range work (`-1` → 0xFFF in 12-bit two's complement).
* Out-of-range immediate (2048) reports `ImmediateOutOfRange`.
* `ret_void`-only program is just `jalr x0, x1, 0`.
* Unsupported op (`add_i64`) reports `UnsupportedOp`.
* Two distinct `const_*` vars use `TEMP_REGISTERS[0..1]` (t0, t1).
* 8 distinct vars triggers `OutOfRegisters`.
* `ret_*` on an undefined name reports `UndefinedVariable`.

## Out of scope (future increments)

* Arithmetic, comparison, branches, calls, locals — everything
  `iir-to-riscv` v0.3.3 had can be ported back in if richer RV32I
  coverage is wanted.
* `ecall print_i64` lowering — the IIR primitive for printing
  integers via the RISC-V syscall ABI.
* Stack-spilling allocator — current backend caps at 7
  simultaneous vars.
* i64 register-pair support — RV32I is 32-bit; `i64` values that
  don't fit in 12-bit immediates trigger `ImmediateOutOfRange`
  today.
* Real JIT execution via `riscv-simulator` (per the GUIDING
  CONSTRAINT, this is best-effort and `Backend::run` panics for
  now).
