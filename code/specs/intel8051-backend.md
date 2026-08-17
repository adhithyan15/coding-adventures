# `intel8051-backend` spec

> **Status:** v0.1.0 — fourth lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

Intel 8051 (MCS-51, 1980) implementation of the
`jit_core::backend::Backend` trait. Mirror of `intel8008-backend` /
`arm1-backend` / `mips-r2000-backend` (the *minimal viable* shape).
The 8051 is Intel's first single-chip microcontroller — CPU, RAM,
ROM, timers, a UART, and I/O ports on one die — and by unit count the
most-manufactured CPU architecture in history: over 20 billion units,
still fabricated today (Atmel/Microchip AT89, NXP 80C51, Silicon Labs
EFM8, and more).

Lowers `Vec<CIRInstr>` (typed, monomorphised) to a `Vec<u8>` of Intel
8051 machine code via `intel8051-encoder`.

## Why this crate exists

This is the fourth lane of a 9-architecture expansion that replicates
the pattern established by the historical-arch backend migration (see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
consume typed **CIR** (not dynamically-typed IIR) via the shared
`Backend` trait, so `lang-aot --emit=intel8051` routes through the
same `aot_core::infer` + `aot_core::specialise` + `Backend::compile`
pipeline every other arch backend (including `aarch64-backend` /
`x86_64-backend`) uses. The 8051 never had an `iir-to-intel8051`
predecessor to migrate away from — this crate starts at the correct
layer from day one.

Unlike ARM1 (whose behavioral simulator already existed complete
in-tree before its `{arch}-encoder`/`{arch}-backend` split), the 8051
needed a brand-new Rust simulator: `intel8051-simulator`
(`code/packages/rust/intel8051-simulator`), ported from the existing
Python behavioral reference
(`code/packages/python/intel8051-simulator`, spec 07p) and
module-split into `opcodes`/`encoding`/`decode`/`execute`/`simulator`,
mirroring `mips-r2000-simulator`'s shape.

## Current scope — minimal viable

| CIR op family | Lowering |
|----------------|----------|
| `const_*` (unsigned 8-bit literal, `[0, 255]`) | `MOV A, #imm` |
| `ret_*` | the HALT sentinel `0xA5` (only if returning the most recently `const_*`'d variable) |
| `ret_void` | the HALT sentinel `0xA5` |
| Empty CIR body | the HALT sentinel `0xA5` |
| Anything else | `UnsupportedOp` from `compile()`; `None` from the `Backend::compile` trait method |

There is **no real register allocator** — a trivial "last const var"
scheme (identical to `arm1-backend`'s and `intel8008-backend`'s)
tracks which single variable the most recent `const_*` wrote into the
accumulator (`A`); `ret_*` only succeeds if it returns exactly that
variable. Programs needing more than one live value fall through to
`UnsupportedOp`. AOT treats `None` as a per-function compile failure;
JIT keeps execution on the interpreter tier.

Full op coverage (arithmetic, comparisons, branches, calls) is
**intentionally not ported** in this PR — future increments can
extend `compile_single_function` using the 8051's `R0`-`R7` working
registers, direct/indirect RAM addressing, and the bit-addressable
region.

## Why `ret_*` lowers to the HALT sentinel, and not self-jump (`SJMP $`) detection

**There is no real HALT instruction on the 8051.** Unlike the Intel
8080/8008 (a genuine `HLT` opcode) or a modern hosted architecture (an
OS to return control to), a real, running 8051 program that's done
working spins forever (`SJMP $`, jump-to-self) or waits for the next
interrupt — the chip simply has nothing to hand control back to. This
is the historically idiomatic 8051 "the program is done" convention,
and it was seriously considered for this backend: detect a fixed
`SJMP $` self-loop pattern (`[0x80, 0xFE]`) as a pseudo-halt a
simulator recognises, the same way a real 8051 in-circuit debugger
would notice the PC has stopped advancing between polls.

It was **not** used, for a concrete reason: this architecture already
has a tested, shipped, documented HALT convention. The Python
behavioral reference `intel8051-simulator` was ported from
(`intel8051_simulator.state.HALT_OPCODE`, spec 07p) already defines
opcode `0xA5` — reserved/undefined in every MCS-51 opcode map — as a
HALT sentinel: executing it sets `halted = true` and stops the
fetch-decode-execute loop, exactly like a PDP-11 program of this
codebase's other historical-arch lanes terminating on a reserved trap
value rather than a real "power off" instruction.
`intel8051_simulator::opcodes::HALT_OPCODE` ports this constant
unchanged.

Inventing a *second*, different halt convention (self-jump detection)
for the same architecture, in the same codebase, would fracture
parity between the Python and Rust simulators for no benefit — both
now agree byte-for-byte on what "the program is done" means, and any
consumer that already knows to look for `0xA5` (a debugger, a
disassembler, a test harness written against the Python simulator)
keeps working unchanged against the Rust port. It is also strictly
simpler for an *emit-only, minimal-viable* backend to produce and for
a simulator to detect: one opcode-equality check in
`intel8051_simulator::execute`, versus pattern-matching "is the next
fetch going to re-execute the same two-byte `SJMP` at the same
address" — real, avoidable complexity for a target whose only current
job is materialising one constant and stopping.

Self-jump detection remains available as a documented fallback for a
future increment that grows real subroutine calls and needs `ret_*`
to mean "return to caller" rather than "the whole program is done" —
at that point the sentinel's whole-program-exit semantics and a
per-function return would need to coexist, and self-jump (or a real
`RET`/stack-based return, which the simulator already implements)
becomes the more honest choice for the *per-function* case while the
sentinel remains the program's outermost exit.

## Wire format

Each instruction is written byte-for-byte, exactly as
`intel8051-encoder` emits it — no endianness conversion, since every
8051 instruction is a byte sequence rather than a fixed-width word
(the same convention `intel8008-backend`/`intel4004-backend` use, and
unlike ARM1/ARMv7/RV32I's 32-bit little-endian words). Per-function
byte streams can be concatenated directly; `lang-aot` writes them
straight to disk as a flat `.bin`.

## Pinned byte sequence

| Program | CIR | Emitted bytes |
|---------|-----|----------------|
| Twig `42` | `const_i64 v=42; ret_i64 v` | `[0x74, 0x2A, 0xA5]` |
| `ret_void` only | `ret_void` | `[0xA5]` |
| Empty CIR | (none) | `[0xA5]` |

`MOV A, #42` = `[0x74, 0x2A]`; the HALT sentinel = `[0xA5]`.

## Backend trait surface

| Trait method | Behaviour |
|---------------|-----------|
| `name()` | returns `"intel8051"` |
| `compile(ir)` | returns `Some(bytes)` for supported CIR ops; `None` otherwise |
| `compile_function(ctx, ir)` | ignores `FunctionContext` (no parameter marshalling in v0.1.0); delegates to `compile` |
| `run(binary, args)` | **panics** with `"intel8051 backend is emit-only; load bytes into intel8051-simulator to execute"` — emit-only per the migration spec |

## Error variants

| `BackendError` variant | Trigger |
|--------------------------|---------|
| `UnsupportedOp(String)` | CIR operation outside `const_*`/`ret_*` |
| `InvalidOperand(String)` | Malformed CIR operands or missing `dest` |
| `UndefinedVariable(String)` | Reserved for a future register allocator (unused in v0.1.0's single-var scheme, where the "not the current accumulator var" case surfaces as `UnsupportedOp` instead) |
| `ImmediateOutOfRange(i64)` | A `const_*` literal falls outside `[0, 255]` — `MOV A, #imm`'s plain unsigned 8-bit operand; negative literals need a two's-complement encoding step, out of scope for v0.1.0 |

## Tests

15 unit/integration tests in `tests/test_backend.rs` (mirroring
`arm1-backend`'s/`intel8008-backend`'s test shape) pin the canonical
byte sequence and edge cases (zero, 8-bit range boundaries — negative
and `>255` — bool, multi-var fallthrough, unsupported op, empty CIR,
`ret_void`, `Backend::run` panics, `Backend::compile` vs the free
`compile` function agree).

Two tests specifically exercise the HALT-sentinel design decision:

1. One loads the compiled canonical bytes into `intel8051-simulator`,
   runs it with a bounded step limit, and asserts `acc() == 42`,
   `halted() == true`, and `steps == 2` — byte-for-byte parity is
   necessary but not sufficient; the emitted bytes must actually
   execute correctly (and actually halt, within a bounded step count)
   in the simulator.
2. A converse test loads a raw `SJMP $` self-loop (bypassing the
   backend entirely) and proves it does **not** halt and **does**
   exhaust the step budget — confirming the first test's
   `halted == true` assertion is meaningful rather than vacuously
   true of any bounded run.

## Backlog

1. [ ] Real register allocator over the 8051's `R0`-`R7` working
   registers and direct/indirect RAM addressing, removing the
   single-accumulator limitation.
2. [ ] Arithmetic/bitwise CIR ops (`add`/`sub`/`and`/`or`/`xor`) via
   `ADD A,#imm`/`ANL A,#imm`/etc. (already implemented in
   `intel8051-simulator::execute`, not yet re-exported as `encode_*`
   helpers).
3. [ ] Comparisons and conditional branches, using `CJNE`/`JZ`/`JNZ`/
   `JC`/`JNC`/`JB`/`JNB`.
4. [ ] Direct calls (`LCALL`/`RET` pairing, or `ACALL` for
   same-2KB-page targets) and a stack frame — once this lands,
   per-function `ret_*` could use the 8051's real `RET` instruction
   (already implemented in the simulator) while the HALT sentinel
   remains reserved for the outermost program-exit case, and
   self-jump detection becomes available as an alternative outermost
   convention if a future increment prefers it.
5. [ ] `Backend::run` wired to `intel8051-simulator` for JIT execution
   (best-effort per the migration spec — "no working JIT" is an
   acceptable outcome for a historical-arch target).
