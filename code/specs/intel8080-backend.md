# `intel8080-backend` spec

> **Status:** v0.1.0 — third lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

Intel 8080 implementation of the `jit_core::backend::Backend` trait.
Mirror of `intel8008-backend` in shape and scope — the *minimal
viable* pattern every historical-arch lane uses (`armv7-backend` /
`ge225-backend` / `intel4004-backend` / `mips-r2000-backend`), not a
fully-featured backend.

Lowers `Vec<CIRInstr>` (typed, monomorphised) to a `Vec<u8>` of Intel
8080 machine code via `intel8080-encoder`.

## Why the Intel 8080, and why this shape?

The Intel 8080 (1974) is the 8008's direct architectural successor:
still an 8-bit accumulator machine with a real `HLT` opcode
(`0x76` — the same bit pattern the 8008 uses, since the 8080 kept
`MOV M,M`'s encoding as the halt sentinel). That means the
"`const_*` → load into the accumulator, `ret_*` → `HLT`" backend shape
that `intel8008-backend` already implements maps almost directly onto
the 8080, unlike MIPS R2000 (`JR $ra`) or ARM1 (SWI pseudo-halt), which
needed a different return-mechanism story.

Per the migration spec
([`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
consume typed **CIR** (not dynamically-typed IIR) via the shared
`Backend` trait, so `lang-aot --emit=intel8080` routes through the
same `aot_core::infer` + `aot_core::specialise` + `Backend::compile`
pipeline every other arch backend (including `aarch64-backend` /
`x86_64-backend`) uses. The Intel 8080 never had an `iir-to-intel8080`
predecessor to migrate away from — like `mips-r2000-backend` and
`arm1-backend`, this crate starts at the correct layer from day one.

## Current scope — minimal viable

| CIR op family | Lowering |
|----------------|----------|
| `const_*` (8-bit unsigned literal) | `MVI A, imm` |
| `ret_*` | `HLT` (only if returning the most recently `const_*`'d variable) |
| `ret_void` | `HLT` |
| Empty CIR body | `HLT` |
| Anything else | `UnsupportedOp` from `compile()`; `None` from the `Backend::compile` trait method |

There is **no real register allocator** — a trivial "last const var"
scheme tracks which single variable the most recent `const_*` wrote
into the accumulator (A); `ret_*` only succeeds if it returns exactly
that variable. Programs needing more than one live value fall through
to `UnsupportedOp`.

Full op coverage (arithmetic, comparisons, branches, calls) that a
mature backend would carry is **intentionally not ported** in this PR
— `intel8080-simulator` already implements the entire ISA these could
lower to, so a future increment to `intel8080-backend` has no ISA
groundwork left to do, only CIR-to-encoder wiring.

## Wire format

Each instruction is a variable-length (1, 2, or 3 byte) Intel 8080
opcode sequence, written in execution order with no endianness
conversion at this layer (`intel8080-encoder`'s `encode_*` helpers
already place any 16-bit operand little-endian within the
instruction). Per-function byte streams can be concatenated directly;
`lang-aot` writes them straight to disk as a flat `.bin`.

## Pinned byte sequence

| Program | CIR | Emitted bytes |
|---------|-----|----------------|
| Twig `42` | `const_i64 v=42; ret_i64 v` | `[0x3E, 0x2A, 0x76]` |
| `ret_void` only | `ret_void` | `[0x76]` |
| Empty CIR | (none) | `[0x76]` |

`MVI A, 42` = `[0x3E, 0x2A]`; `HLT` = `[0x76]`. Byte-for-byte identical
to `intel8008-backend`'s canonical output for the same program, since
both chips share the `MVI A, n` / `HLT` encoding.

## Backend trait surface

| Trait method | Behaviour |
|---------------|-----------|
| `name()` | returns `"intel8080"` |
| `compile(ir)` | returns `Some(bytes)` for supported CIR ops; `None` otherwise |
| `compile_function(ctx, ir)` | ignores `FunctionContext` (no parameter marshalling in v0.1.0); delegates to `compile` |
| `run(binary, args)` | **panics** with `"intel8080 backend is emit-only; load bytes into intel8080-simulator to execute"` — emit-only per the migration spec |

## Error variants

| `BackendError` variant | Trigger |
|--------------------------|---------|
| `UnsupportedOp(String)` | CIR operation outside `const_*`/`ret_*` |
| `InvalidOperand(String)` | Malformed CIR operands or missing `dest` |
| `UndefinedVariable(String)` | Reserved for a future register allocator (unused in v0.1.0's single-var scheme, where the "not the current accumulator var" case surfaces as `UnsupportedOp` instead) |
| `ImmediateOutOfRange(i64)` | A `const_*` literal falls outside `[0, 255]` — `MVI`'s 8-bit immediate field |

## Tests

12 unit/integration tests in `tests/test_backend.rs` (mirroring
`intel8008-backend`'s/`mips-r2000-backend`'s test shape) pin the
canonical byte sequence and edge cases (zero, 8-bit max, immediate
overflow, bool, multi-var fallthrough, unsupported op, empty CIR,
`ret_void`, `Backend::run` panics, `Backend::compile` vs the free
`compile` function agree).

One test additionally loads the compiled bytes into
`intel8080-simulator`, runs it, and asserts the accumulator equals 42
after execution — byte-for-byte parity is necessary but not
sufficient; the emitted bytes must actually execute correctly in the
new simulator.

## Backlog

1. [ ] Real register allocator using the 8080's B/C/D/E/H/L temp
   registers, removing the single-var limitation.
2. [ ] Arithmetic/bitwise CIR ops (`add`/`sub`/`and`/`or`/`xor`) —
   `intel8080-simulator` already implements ADD/SUB/ANA/XRA/ORA/CMP,
   so this is CIR-to-encoder wiring only.
3. [ ] Comparisons and conditional branches — the simulator already
   implements all 8 condition codes.
4. [ ] Direct calls (`CALL`/`RET` pairing) and a stack frame — the
   simulator already implements `CALL`/`RET`/`PUSH`/`POP`.
5. [ ] `Backend::run` wired to `intel8080-simulator` for JIT execution
   (best-effort per the migration spec — "no working JIT" is an
   acceptable outcome for a historical-arch target).
