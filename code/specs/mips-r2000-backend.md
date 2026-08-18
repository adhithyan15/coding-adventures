# `mips-r2000-backend` spec

> **Status:** v0.1.0 — first lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

MIPS R2000 implementation of the `jit_core::backend::Backend` trait.
Mirror of `armv7-backend` / `intel8008-backend` / `ge225-backend` /
`intel4004-backend` (the *minimal viable* shape, not `riscv-backend`,
which grew a full executable scalar core far beyond minimal).

Lowers `Vec<CIRInstr>` (typed, monomorphised) to a big-endian
`Vec<u8>` of MIPS R2000 machine code via `mips-r2000-encoder`.

## Why this crate exists

This is the first lane of a 9-architecture expansion that replicates
the pattern established by the historical-arch backend migration
(see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
consume typed **CIR** (not dynamically-typed IIR) via the shared
`Backend` trait, so `lang-aot --emit=mips-r2000` routes through the
same `aot_core::infer` + `aot_core::specialise` + `Backend::compile`
pipeline every other arch backend (including `aarch64-backend` /
`x86_64-backend`) uses.  MIPS R2000 never had an `iir-to-mips`
predecessor to migrate away from — this crate starts at the correct
layer from day one.

## Current scope — minimal viable

| CIR op family | Lowering |
|----------------|----------|
| `const_*` (16-bit signed literal) | `ADDIU $v0, $zero, imm` |
| `ret_*` | `JR $ra` (only if returning the most recently `const_*`'d variable) |
| `ret_void` | `JR $ra` |
| Empty CIR body | `JR $ra` |
| Anything else | `UnsupportedOp` from `compile()`; `None` from the `Backend::compile` trait method |

There is **no real register allocator** — a trivial "last const var"
scheme tracks which single variable the most recent `const_*` wrote
into `$v0`; `ret_*` only succeeds if it returns exactly that
variable.  Programs needing more than one live value fall through to
`UnsupportedOp`.  AOT treats `None` as a per-function compile
failure; JIT keeps execution on the interpreter tier.

Full op coverage (arithmetic, comparisons, branches, calls) that a
mature backend would carry is **intentionally not ported** in this
PR — future increments can extend `compile_single_function` using
the `TEMP_REGISTERS` pool `mips-r2000-encoder` already declares.

## Wire format

Each instruction is a 32-bit MIPS R2000 word, flattened to
**big-endian** bytes — MIPS R2000's default byte order (unlike
RISC-V/ARMv7/x86, which are little-endian).  Per-function byte
streams can be concatenated directly; `lang-aot` writes them straight
to disk as a flat `.bin`.

## Pinned byte sequence

| Program | CIR | Emitted bytes |
|---------|-----|----------------|
| Twig `42` | `const_i64 v=42; ret_i64 v` | `[0x24, 0x02, 0x00, 0x2A, 0x03, 0xE0, 0x00, 0x08]` |
| `ret_void` only | `ret_void` | `[0x03, 0xE0, 0x00, 0x08]` |
| Empty CIR | (none) | `[0x03, 0xE0, 0x00, 0x08]` |

`ADDIU $v0, $zero, 42` = `0x2402_002A`; `JR $ra` = `0x03E0_0008`.

## Backend trait surface

| Trait method | Behaviour |
|---------------|-----------|
| `name()` | returns `"mips-r2000"` |
| `compile(ir)` | returns `Some(bytes)` for supported CIR ops; `None` otherwise |
| `compile_function(ctx, ir)` | ignores `FunctionContext` (no parameter marshalling in v0.1.0); delegates to `compile` |
| `run(binary, args)` | **panics** with `"mips-r2000 backend is emit-only; load bytes into mips-r2000-simulator to execute"` — emit-only per the migration spec |

## Error variants

| `BackendError` variant | Trigger |
|--------------------------|---------|
| `UnsupportedOp(String)` | CIR operation outside `const_*`/`ret_*` |
| `InvalidOperand(String)` | Malformed CIR operands or missing `dest` |
| `UndefinedVariable(String)` | Reserved for a future register allocator (unused in v0.1.0's single-var scheme, where the "not the current `$v0` var" case surfaces as `UnsupportedOp` instead) |
| `ImmediateOutOfRange(i64)` | A `const_*` literal falls outside `[-32768, 32767]` — `ADDIU`'s 16-bit signed immediate field; wider values need a `lui`+`ori` pair, out of scope for v0.1.0 |

## Tests

11 unit/integration tests in `tests/test_backend.rs` (mirroring
`armv7-backend`'s/`intel8008-backend`'s test shape) pin the canonical
byte sequence and edge cases (zero, 16-bit signed range boundaries,
immediate overflow, bool, multi-var fallthrough, unsupported op,
empty CIR, `ret_void`, `Backend::run` panics, `Backend::compile` vs
the free `compile` function agree).

One test additionally loads the compiled bytes into
`mips-r2000-simulator`, steps the simulator, and asserts `$v0 == 42`
after execution — byte-for-byte parity is necessary but not
sufficient; the emitted bytes must actually execute correctly in the
new simulator.

## Backlog

1. [ ] Real register allocator using the `TEMP_REGISTERS` pool
   (`$t0..$t7`), removing the single-var limitation.
2. [ ] Arithmetic/bitwise CIR ops (`add`/`sub`/`and`/`or`/`xor`/shifts).
3. [ ] Comparisons and conditional branches.
4. [ ] Direct calls (`JAL`/`JR $ra` pairing) and a stack frame.
5. [ ] `Backend::run` wired to `mips-r2000-simulator` for JIT
   execution (best-effort per the migration spec — "no working JIT"
   is an acceptable outcome for a historical-arch target).
