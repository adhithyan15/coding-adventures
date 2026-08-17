# `arm1-backend` spec

> **Status:** v0.1.0 — second lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

ARM1 (ARMv1, 1985) implementation of the `jit_core::backend::Backend`
trait.  Mirror of `mips-r2000-backend` / `armv7-backend` /
`intel8008-backend` / `ge225-backend` (the *minimal viable* shape).
ARM1 is architecturally the direct ancestor of the already-migrated
`armv7-backend` lane — Sophie Wilson and Steve Furber's original
Acorn RISC Machine, designed in 1985, the first commercially
successful RISC chip, and the chip family ARM/Acorn later grew into
ARMv7-A (deployed in billions of Cortex-A-class SoCs).

Lowers `Vec<CIRInstr>` (typed, monomorphised) to a little-endian
`Vec<u8>` of ARM1 machine code via `arm1-encoder`.

## Why this crate exists

This is the second lane of a 9-architecture expansion that
replicates the pattern established by the historical-arch backend
migration (see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
consume typed **CIR** (not dynamically-typed IIR) via the shared
`Backend` trait, so `lang-aot --emit=arm1` routes through the same
`aot_core::infer` + `aot_core::specialise` + `Backend::compile`
pipeline every other arch backend (including `aarch64-backend` /
`x86_64-backend`) uses.  ARM1 never had an `iir-to-arm1` predecessor
to migrate away from — this crate starts at the correct layer from
day one, same as `mips-r2000-backend`.

Unlike `mips-r2000-backend` (which needed a brand-new
`mips-r2000-simulator` alongside it), ARM1's behavioral simulator
(`arm1-simulator`, 2270 lines) already existed, complete, in-tree —
this lane only needed the `{arch}-encoder` + `{arch}-backend` split
on top of it.

## Current scope — minimal viable

| CIR op family | Lowering |
|----------------|----------|
| `const_*` (unrotated 8-bit literal, `[0, 255]`) | `MOV R0, #imm` |
| `ret_*` | pseudo-halt `SWI #0x123456` (only if returning the most recently `const_*`'d variable) |
| `ret_void` | pseudo-halt `SWI #0x123456` |
| Empty CIR body | pseudo-halt `SWI #0x123456` |
| Anything else | `UnsupportedOp` from `compile()`; `None` from the `Backend::compile` trait method |

There is **no real register allocator** — a trivial "last const var"
scheme tracks which single variable the most recent `const_*` wrote
into `R0`; `ret_*` only succeeds if it returns exactly that
variable.  Programs needing more than one live value fall through to
`UnsupportedOp`.  AOT treats `None` as a per-function compile
failure; JIT keeps execution on the interpreter tier.

Full op coverage (arithmetic, comparisons, branches, calls) that a
mature backend would carry is **intentionally not ported** in this
PR — future increments can extend `compile_to_words` using ARM1's
other 14 general-purpose registers and the barrel shifter's rotated-
immediate form (which would also widen `const_*`'s literal range
past `[0, 255]`).

## Why `ret_*` lowers to a pseudo-halt, not `BX LR`

`armv7-backend` (ARMv7-A, 2 decades newer) returns via `BX LR` — the
link-register-return convention every modern ARM ABI uses.  ARM1/
ARMv1 predates that convention entirely: there is no `BX`
instruction, and the era's subroutine-return idiom, `MOVS PC, R14`,
requires a live `R14` that only a preceding `BL` sets up (i.e. it
needs a *caller*).  The minimal-viable `const_*`/`ret_*` scope
compiles a whole program's worth of CIR with no caller in the
picture — the trivial ROM just needs to compute a value and stop.

`arm1-simulator` already defines exactly this: a pseudo-halt
instruction, `SWI #0x123456` (`arm1_simulator::HALT_SWI`), that its
`execute_swi` intercepts specially — when the SWI's 24-bit comment
field equals `HALT_SWI`, the simulator sets its internal `halted`
flag (observable via `ARM1::halted()`) instead of entering
Supervisor mode like a genuine SWI would.  This is a simulator-level
halt convention (parallel to the Intel 8008 backend's `HLT` byte or
the GE-225 backend's `HLT` word), not real ARM1 silicon behaviour.
Lowering `ret_*`/`ret_void` to this pseudo-halt is the semantically
correct choice: it is the only instruction that actually stops the
fetch-decode-execute loop, leaving the computed value in `R0` for
the caller to read via `read_register(0)`.

## Wire format

Each instruction is a 32-bit ARM1 word, flattened to
**little-endian** bytes — ARM1's byte order (see
`arm1_simulator::ARM1::read_word`/`write_word`, which use
`u32::from_le_bytes`/`to_le_bytes`).  Per-function byte streams can
be concatenated directly; `lang-aot` writes them straight to disk as
a flat `.bin`.

## Pinned byte sequence

| Program | CIR | Emitted bytes |
|---------|-----|----------------|
| Twig `42` | `const_i64 v=42; ret_i64 v` | `[0x2A, 0x00, 0xA0, 0xE3, 0x56, 0x34, 0x12, 0xEF]` |
| `ret_void` only | `ret_void` | `[0x56, 0x34, 0x12, 0xEF]` |
| Empty CIR | (none) | `[0x56, 0x34, 0x12, 0xEF]` |

`MOV R0, #42` = `0xE3A0_002A`; `SWI #0x123456` (`AL`) = `0xEF12_3456`.

## Backend trait surface

| Trait method | Behaviour |
|---------------|-----------|
| `name()` | returns `"arm1"` |
| `compile(ir)` | returns `Some(bytes)` for supported CIR ops; `None` otherwise |
| `compile_function(ctx, ir)` | ignores `FunctionContext` (no parameter marshalling in v0.1.0); delegates to `compile` |
| `run(binary, args)` | **panics** with `"arm1 backend is emit-only; load bytes into arm1-simulator to execute"` — emit-only per the migration spec |

## Error variants

| `BackendError` variant | Trigger |
|--------------------------|---------|
| `UnsupportedOp(String)` | CIR operation outside `const_*`/`ret_*` |
| `InvalidOperand(String)` | Malformed CIR operands or missing `dest` |
| `UndefinedVariable(String)` | Reserved for a future register allocator (unused in v0.1.0's single-var scheme, where the "not the current `R0` var" case surfaces as `UnsupportedOp` instead) |
| `ImmediateOutOfRange(i64)` | A `const_*` literal falls outside `[0, 255]` — `MOV Rd, #imm8`'s unrotated immediate field; wider or negative values need the barrel shifter's rotated-immediate form (or `MVN`), out of scope for v0.1.0 |

## Tests

14 unit/integration tests in `tests/test_backend.rs` (mirroring
`mips-r2000-backend`'s/`armv7-backend`'s test shape) pin the
canonical byte sequence and edge cases (zero, 8-bit range boundaries
— negative and `>255` — bool, multi-var fallthrough, unsupported op,
empty CIR, `ret_void`, `Backend::run` panics, `Backend::compile` vs
the free `compile` function agree).

One test additionally loads the compiled bytes into `arm1-simulator`,
runs it, and asserts `R0 == 42` and `halted() == true` after
execution — byte-for-byte parity is necessary but not sufficient;
the emitted bytes must actually execute correctly (and actually
halt) in the existing simulator.

## Backlog

1. [ ] Real register allocator over ARM1's other 14 general-purpose
   registers, removing the single-var limitation.
2. [ ] Rotated-immediate `const_*` (widens the `[0, 255]` literal
   range using the barrel shifter's 4-bit rotate field).
3. [ ] Arithmetic/bitwise CIR ops (`add`/`sub`/`and`/`or`/`xor`) via
   `encode_alu_reg` (already re-exportable from `arm1_simulator`).
4. [ ] Comparisons and conditional branches, using ARM1's per-
   instruction condition codes (`encode_data_processing`'s `cond`
   parameter) rather than separate compare-then-branch pairs.
5. [ ] Direct calls (`BL`/`MOVS PC, R14` pairing) and a stack frame
   — once this lands, `ret_*` could switch from the pseudo-halt to
   the historically authentic ARMv1 return idiom for called
   functions (the pseudo-halt would remain for the outermost
   program-exit case).
6. [ ] `Backend::run` wired to `arm1-simulator` for JIT execution
   (best-effort per the migration spec — "no working JIT" is an
   acceptable outcome for a historical-arch target).
