# `m68k-backend` spec

> **Status:** v0.1.0 — eighth lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

Motorola 68000 (1979) implementation of the `jit_core::backend::Backend`
trait. Mirror of `mos6502-backend` / `arm1-backend` / `armv7-backend` /
`intel8008-backend` (the *minimal viable* shape). The 68000 is the
landmark 16/32-bit processor that powered the original Apple Macintosh
(1984), Commodore Amiga (1985), Atari ST (1985), early Sun-1/Sun-2
workstations, and the Sega Genesis/Mega Drive (1988) — the CPU
clean-ISA advocates point to as "what the 8086 should have been," with
8 general-purpose 32-bit data registers, 8 address registers, and 14
genuinely orthogonal addressing modes.

Lowers `Vec<CIRInstr>` (typed, monomorphised) to `Vec<u8>` of big-endian
Motorola 68000 machine code via `m68k-encoder`.

## Why this crate exists

Eighth lane of a 9-architecture expansion that replicates the pattern
established by the historical-arch backend migration (see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
consume typed **CIR** (not dynamically-typed IIR) via the shared
`Backend` trait, so `lang-aot --emit=m68k` routes through the same
`aot_core::infer` + `aot_core::specialise` + `Backend::compile` pipeline
every other arch backend (including `aarch64-backend` / `x86_64-backend`)
uses. The 68000 never had an `iir-to-m68k` predecessor to migrate away
from — this crate starts at the correct layer from day one, same as
`mos6502-backend`/`arm1-backend`.

Unlike ARM1 (whose behavioral simulator, `arm1-simulator`, pre-existed
complete in-tree before this lane), the 68000 needed a brand-new Rust
simulator alongside the encoder/backend pair — `m68k-simulator`, a
substantial port of the pre-existing Python simulator
(`code/packages/python/motorola-68000-simulator`), covering a genuinely
useful subset of the ISA (not just the trivial-ROM opcodes) — see
`m68k-simulator`'s own README for the full instruction/addressing-mode
coverage table.

## Current scope — minimal viable

| CIR op family | Lowering |
|----------------|----------|
| `const_*` (32-bit literal, `[i32::MIN, u32::MAX]`) | `MOVE.L #imm, D0` |
| `ret_*` | `TRAP #15` (only if returning the most recently `const_*`'d variable) |
| `ret_void` | `TRAP #15` |
| Empty CIR body | `TRAP #15` |
| Anything else | `UnsupportedOp` from `compile()`; `None` from the `Backend::compile` trait method |

There is **no real register allocator** — a trivial "last const var"
scheme tracks which single variable the most recent `const_*` wrote into
data register `D0`; `ret_*` only succeeds if it returns exactly that
variable. Programs needing more than one live value fall through to
`UnsupportedOp`. AOT treats `None` as a per-function compile failure;
JIT keeps execution on the interpreter tier.

Full op coverage (arithmetic, comparisons, branches, calls — a mature
backend's worth) is **intentionally not wired into this backend** in
this PR, even though `m68k-simulator` already implements a useful
subset of those mnemonics (`ADD`/`SUB`/`AND`/`OR`/`EOR`/`CMP`,
`Bcc`/`DBcc`/`Scc`, `JSR`/`JMP`/`RTS`). Future increments can extend
`m68k-backend::compile_to_bytes` to emit them using the encoder helpers
`m68k-encoder` already re-exports (or widen that re-export list
further) — the simulator-side work to execute them is already done.

## Why `ret_*` lowers to `TRAP #15`, not `STOP #imm`

The 68000 has **two** genuinely real halting instructions — `STOP #imm`
(architecturally the more literal "halt": loads an immediate into the
status register and stops until an interrupt) and `TRAP #15`
(architecturally a software-interrupt call). The pre-existing, in-tree
Python simulator's own `state.py` documents *both* as valid: *"halted:
True after STOP or TRAP #15 executes."*

Since neither is objectively "more real" per that doc, this lane follows
this repo's own established rule for such ties (see
`code/specs/mos6502-encoder.md`'s "Why `BRK`, not KIL/JAM or a self-jump
spin loop?" section for the same reasoning applied to a different ISA):
**mirror whatever the pre-existing reference already does, don't invent
a fresh convention.** The Python original's own test suite settles which
one that is —
`code/packages/python/motorola-68000-simulator/tests/
test_instructions.py`'s `_stop()` helper (*"TRAP #15 — halts simulation
without modifying SR"*) is used across 100+ of that file's test programs
(plus 18 more in `test_programs.py`); `STOP #imm` appears exactly once,
in a module-level doctest example. `TRAP #15` is the dominant,
already-established idiom — mirroring it here is the correct choice,
the same way `mos6502-backend`'s `BRK` mirrors the 6502 Python
original's own documented convention rather than reaching for either
alternative that lane's spec considered and rejected (an illegal-opcode
lock, or a self-jump spin loop).

Unlike ARM1/ARMv1 (1985, no real halt instruction at all — `arm1-backend`
had to invent a pseudo-halt via `SWI #0x123456` because ARM1 silicon has
no native way to stop), the 68000 already has *two* genuine halting
instructions to choose between; no pseudo-instruction invention was
needed for this lane, only a choice between two real ones, settled by
which one the existing reference's own test suite already treats as
canonical.

## Wire format

Each instruction is 2-6 raw bytes, big-endian — the 68000's native byte
order. Unlike `arm1-backend` (which flattens little-endian ARM1 words
via `to_le_bytes()`), there is no endianness-conversion step here:
`m68k-encoder`'s bytes are already the wire format. Per-function byte
streams concatenate directly; `lang-aot` writes them straight to disk as
a flat `.bin`.

## Pinned byte sequence

| Program | CIR | Emitted bytes |
|---------|-----|----------------|
| IIR `42` | `const_i64 v=42; ret_i64 v` | `[0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A, 0x4E, 0x4F]` |
| `ret_void` only | `ret_void` | `[0x4E, 0x4F]` |
| Empty CIR | (none) | `[0x4E, 0x4F]` |

`MOVE.L #42, D0` = `[0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A]`; `TRAP #15` =
`[0x4E, 0x4F]`. This exact sequence is independently verified against
the pre-existing Python simulator's own test suite, which uses the same
`0x203C`/`0x223C`/... `MOVE.L #imm, Dn` opword family (see
`test_instructions.py`'s `_w(0x203C) + _l(imm)` idiom, used 15+ times).

## Backend trait surface

| Trait method | Behaviour |
|---------------|-----------|
| `name()` | returns `"m68k"` |
| `compile(ir)` | returns `Some(bytes)` for supported CIR ops; `None` otherwise |
| `compile_function(ctx, ir)` | ignores `FunctionContext` (no parameter marshalling in v0.1.0); delegates to `compile` |
| `run(binary, args)` | **panics** with `"m68k backend is emit-only; load bytes into m68k-simulator to execute"` — emit-only per the migration spec |

## Error variants

| `BackendError` variant | Trigger |
|--------------------------|---------|
| `UnsupportedOp(String)` | CIR operation outside `const_*`/`ret_*` |
| `InvalidOperand(String)` | Malformed CIR operands or missing `dest` |
| `UndefinedVariable(String)` | Reserved for a future register allocator (unused in v0.1.0's single-var scheme, where the "not the current D0 var" case surfaces as `UnsupportedOp` instead) |
| `ImmediateOutOfRange(i64)` | A `const_*` literal falls outside `[i32::MIN, u32::MAX]` — `MOVE.L #imm, D0`'s 32-bit immediate field (the widest immediate any lane in this expansion supports, since the 68000's data registers are a full 32 bits wide) |

## Security: termination check uses an explicit `bool`

`compile_to_bytes` tracks an explicit `terminated: bool` — set `true`
only when a real `ret_*`/`ret_void` arm pushes `encode_trap15()`, reset
to `false` whenever a further `const_*` is emitted — rather than
comparing the trailing emitted byte(s) against the halt sentinel's
encoding.

This matters concretely: `TRAP #15`'s low byte is `0x4F`, and `0x4F` is
also reachable as the low byte of a `MOVE.L #imm, D0` immediate (e.g.
`const_i64 79`, whose encoding ends `..., 0x00, 0x4F`). A byte-value
comparison would see that trailing `0x4F`, wrongly conclude a halt was
already emitted, and skip appending the real terminator — exactly the
unsound pattern a prior lane (Intel 8051) shipped and had to fix after
security review (commit `19e360d`, *"track real HALT emission instead of
comparing trailing byte value"*). `m68k-backend` was written with the
`terminated: bool` fix applied from the start; see
`tests/test_backend.rs::const_ending_in_halt_low_byte_with_no_ret_still_appends_real_halt`
for the regression test proving both the byte sequence and that the
simulator actually halts (not just runs out its step budget) for
exactly this case.

## Tests

17 unit/integration tests in `tests/test_backend.rs` (mirroring
`mos6502-backend`'s/`arm1-backend`'s test shape) pin the canonical byte
sequence and edge cases (zero, 32-bit range boundaries — below
`i32::MIN` and above `u32::MAX` — bool, multi-var fallthrough,
unsupported op, empty CIR, `ret_void`, `Backend::run` panics,
`Backend::compile` vs the free `compile` function agree, and the
halt-lookalike-byte security regression above).

One test additionally loads the compiled bytes into `m68k-simulator`,
runs it, and asserts `D0 == 42` and `halted == true` after execution —
byte-for-byte parity is necessary but not sufficient; the emitted bytes
must actually execute correctly (and actually halt) in the new
simulator.

## Backlog

1. [ ] Real register allocator using the 68000's other 7 data registers
   and 8 address registers.
2. [ ] Arithmetic/logical CIR ops (`add`/`sub`/`and`/`or`/`xor`) via
   `ADD`/`SUB`/`AND`/`OR`/`EOR` (already implemented by
   `m68k-simulator`; only the backend-side lowering + wider
   `m68k-encoder` re-export list are missing).
3. [ ] Comparisons and conditional branches, using `CMP` plus the 14
   conditional `Bcc` variants (already implemented by `m68k-simulator`
   — this is a smaller lift than `mos6502-backend`'s equivalent backlog
   item, since the 68000's per-instruction condition-code field works
   the same way ARM1's does).
4. [ ] Direct calls (`JSR`/`RTS` pairing) and a stack frame (`LINK`/
   `UNLK`, already implemented by `m68k-simulator`) — once this lands,
   `ret_*` could switch from `TRAP #15` to `RTS` for called functions
   (the `TRAP #15` halt would remain for the outermost program-exit
   case, matching how ARM1's and MOS 6502's equivalent backlog items
   plan to keep their own halt instructions for program exit even after
   adding real calls).
5. [ ] `Backend::run` wired to `m68k-simulator` for JIT execution
   (best-effort per the migration spec — "no working JIT" is an
   acceptable outcome for a historical-arch target).
6. [ ] The 3 deferred addressing modes (`d8(An,Xn.sz)`, `d16(PC)`,
   `d8(PC,Xn.sz)`) in `m68k-simulator`, needed before any backend
   increment that wants indexed or PC-relative operand access.
