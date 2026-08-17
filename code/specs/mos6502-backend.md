# `mos6502-backend` spec

> **Status:** v0.1.0 — fifth lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

MOS Technology 6502 (1975) implementation of the
`jit_core::backend::Backend` trait.  Mirror of `mips-r2000-backend` /
`arm1-backend` / `armv7-backend` / `intel8008-backend` (the *minimal
viable* shape).  The 6502 (Chuck Peddle, MOS Technology, 1975) sold for
$25 — versus the Intel 8080's $179 — and powered the Apple II (1977),
Commodore 64 (1982), Atari 2600/8-bit line (1977), BBC Micro (1981), and
— via the Ricoh 2A03 variant — the NES/Famicom (1983).  It remains one
of the most influential 8-bit CPUs ever made.

Lowers `Vec<CIRInstr>` (typed, monomorphised) to `Vec<u8>` of MOS 6502
machine code via `mos6502-encoder`.

## Why this crate exists

This is the fifth lane of a 9-architecture expansion that replicates the
pattern established by the historical-arch backend migration (see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
consume typed **CIR** (not dynamically-typed IIR) via the shared
`Backend` trait, so `lang-aot --emit=mos6502` routes through the same
`aot_core::infer` + `aot_core::specialise` + `Backend::compile` pipeline
every other arch backend (including `aarch64-backend` / `x86_64-backend`)
uses.  The MOS 6502 never had an `iir-to-mos6502` predecessor to migrate
away from — this crate starts at the correct layer from day one, same as
`mips-r2000-backend`/`arm1-backend`.

Unlike ARM1 (whose behavioral simulator, `arm1-simulator`, pre-existed
complete in-tree before this lane), the MOS 6502 needed a brand-new Rust
simulator alongside the encoder/backend pair — `mos6502-simulator`, a
full 151-opcode / 13-addressing-mode port of the pre-existing Python
simulator (`code/packages/python/mos6502-simulator`), same shape as
`mips-r2000-simulator`'s from-scratch port.

## Current scope — minimal viable

| CIR op family | Lowering |
|----------------|----------|
| `const_*` (unsigned 8-bit literal, `[0, 255]`) | `LDA #imm` |
| `ret_*` | `BRK` (only if returning the most recently `const_*`'d variable) |
| `ret_void` | `BRK` |
| Empty CIR body | `BRK` |
| Anything else | `UnsupportedOp` from `compile()`; `None` from the `Backend::compile` trait method |

There is **no real register allocator** — a trivial "last const var"
scheme tracks which single variable the most recent `const_*` wrote into
the accumulator (`A`); `ret_*` only succeeds if it returns exactly that
variable.  Programs needing more than one live value fall through to
`UnsupportedOp`.  AOT treats `None` as a per-function compile failure;
JIT keeps execution on the interpreter tier.

Full op coverage (arithmetic, comparisons, branches, calls — a mature
backend's worth) is **intentionally not wired into this backend** in
this PR, even though `mos6502-simulator` already implements every one of
those mnemonics in full.  Future increments can extend
`mos6502-backend::compile_to_bytes` to emit `ADC`/`CMP`/branches/`JSR`
using the encoder helpers `mos6502-encoder` already re-exports (or
widen that re-export list further) — the simulator-side work to execute
them is already done.

## Why `ret_*` lowers to `BRK`, not a pseudo-halt

Unlike ARM1/ARMv1 (1985, no real halt instruction — `arm1-backend`
invents a pseudo-halt via `SWI #0x123456` because ARM1 silicon has no
native way to stop), the MOS 6502 already has a genuine one-byte
instruction, `BRK`, that the **pre-existing, in-tree Python simulator**
this Rust port mirrors already treats as HALT.  See
`code/specs/mos6502-encoder.md`'s "Why `BRK`, not KIL/JAM or a self-jump
spin loop?" section for the full derivation and the two alternatives
(illegal-opcode lock, self-jump spin loop) considered and rejected.  The
short version: `code/packages/python/mos6502-simulator/src/
mos6502_simulator/simulator.py`'s own module docstring says *"BRK
(opcode 0x00) is treated as HALT ... This matches the convention used
throughout the simulator stack (HLT for 8080, TRAP for IBM 704, etc.)"*
— this is an **existing, established** convention for this specific ISA
in this repo, not a new design decision made for this lane.  Mirroring
it (rather than inventing a fresh pseudo-halt the way ARM1's lane had
to) is the correct choice.

## Wire format

Each instruction is 1-3 raw bytes — the MOS 6502 is a byte-oriented
ISA with **no word endianness**, unlike every fixed-32-bit-word target
in this repo (MIPS R2000, ARM1, RV32I, ARMv7).  Per-function byte
streams concatenate directly, with no flattening/endianness step;
`lang-aot` writes them straight to disk as a flat `.bin`.

## Pinned byte sequence

| Program | CIR | Emitted bytes |
|---------|-----|----------------|
| IIR `42` | `const_i64 v=42; ret_i64 v` | `[0xA9, 0x2A, 0x00]` |
| `ret_void` only | `ret_void` | `[0x00]` |
| Empty CIR | (none) | `[0x00]` |

`LDA #42` = `[0xA9, 0x2A]`; `BRK` = `[0x00]`.

## Backend trait surface

| Trait method | Behaviour |
|---------------|-----------|
| `name()` | returns `"mos6502"` |
| `compile(ir)` | returns `Some(bytes)` for supported CIR ops; `None` otherwise |
| `compile_function(ctx, ir)` | ignores `FunctionContext` (no parameter marshalling in v0.1.0); delegates to `compile` |
| `run(binary, args)` | **panics** with `"mos6502 backend is emit-only; load bytes into mos6502-simulator to execute"` — emit-only per the migration spec |

## Error variants

| `BackendError` variant | Trigger |
|--------------------------|---------|
| `UnsupportedOp(String)` | CIR operation outside `const_*`/`ret_*` |
| `InvalidOperand(String)` | Malformed CIR operands or missing `dest` |
| `UndefinedVariable(String)` | Reserved for a future register allocator (unused in v0.1.0's single-var scheme, where the "not the current accumulator var" case surfaces as `UnsupportedOp` instead) |
| `ImmediateOutOfRange(i64)` | A `const_*` literal falls outside `[0, 255]` — `LDA #imm`'s unsigned 8-bit immediate field (the 6502 accumulator is 8 bits wide, so there is no wider "unrotated"/"rotated" distinction the way ARM1's barrel shifter has) |

## Tests

14 unit/integration tests in `tests/test_backend.rs` (mirroring
`mips-r2000-backend`'s/`arm1-backend`'s test shape) pin the canonical
byte sequence and edge cases (zero, 8-bit range boundaries — negative
and `>255` — bool, multi-var fallthrough, unsupported op, empty CIR,
`ret_void`, `Backend::run` panics, `Backend::compile` vs the free
`compile` function agree).

One test additionally loads the compiled bytes into `mos6502-simulator`,
runs it, and asserts `A == 42` and `halted == true` after execution —
byte-for-byte parity is necessary but not sufficient; the emitted bytes
must actually execute correctly (and actually halt) in the new
simulator.

## Backlog

1. [ ] Real register allocator using the 6502's `X`/`Y` index registers
   and zero page as additional storage, removing the single-var
   limitation.
2. [ ] Arithmetic/logical CIR ops (`add`/`sub`/`and`/`or`/`xor`) via
   `ADC`/`SBC`/`AND`/`ORA`/`EOR` (already implemented in full by
   `mos6502-simulator`; only the backend-side lowering + wider
   `mos6502-encoder` re-export list are missing).
3. [ ] Comparisons and conditional branches, using `CMP`/`CPX`/`CPY`
   plus the 8 conditional branch instructions (`BEQ`/`BNE`/etc.) —
   the 6502 has no per-instruction condition-code field the way ARM1
   does, so this needs an explicit compare-then-branch pairing, closer
   to `mips-r2000-backend`'s eventual branch story than ARM1's.
4. [ ] Direct calls (`JSR`/`RTS` pairing) and a stack frame — once this
   lands, `ret_*` could switch from `BRK` to `RTS` for called functions
   (the `BRK` halt would remain for the outermost program-exit case,
   matching how ARM1's backlog item 5 plans to keep its pseudo-halt for
   program exit even after adding real calls).
5. [ ] `Backend::run` wired to `mos6502-simulator` for JIT execution
   (best-effort per the migration spec — "no working JIT" is an
   acceptable outcome for a historical-arch target).
