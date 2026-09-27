# `z80-backend` spec

> **Status:** v0.1.0 — seventh lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

Zilog Z80 implementation of the `jit_core::backend::Backend` trait.
Mirror of `intel8080-backend` in shape and scope — the *minimal viable*
pattern every historical-arch lane uses (`armv7-backend` /
`ge225-backend` / `intel4004-backend` / `mips-r2000-backend` /
`arm1-backend`), not a fully-featured backend.

Lowers `Vec<CIRInstr>` (typed, monomorphised) to a `Vec<u8>` of Zilog
Z80 machine code via `z80-encoder`.

## Why the Z80, and why this shape?

The Z80 (1976) is the Intel 8080's direct architectural successor and a
full superset of its opcode set: still an 8-bit accumulator machine
with a real `HALT` opcode (`0x76` — the exact same bit pattern the 8080
uses for `HLT`, since the Z80 kept it verbatim). That means the
"`const_*` → load into the accumulator, `ret_*` → `HALT`" backend shape
that `intel8080-backend` already implements maps almost directly onto
the Z80, unlike MIPS R2000 (`JR $ra`) or ARM1 (SWI pseudo-halt), which
needed a different return-mechanism story.

Per the migration spec
([`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
consume typed **CIR** (not dynamically-typed IIR) via the shared
`Backend` trait, so `lang-aot --emit=z80` routes through the same
`aot_core::infer` + `aot_core::specialise` + `Backend::compile` pipeline
every other arch backend (including `aarch64-backend` /
`x86_64-backend`) uses. The Z80 never had an `iir-to-z80` predecessor to
migrate away from — like `mips-r2000-backend`/`arm1-backend`, this crate
starts at the correct layer from day one.

## Current scope — WORD02 arithmetic

| CIR op family | Lowering |
|----------------|----------|
| `const_u8`, `const_bool` | `LD A, imm` |
| `const_u16` | `LD HL, imm16` |
| `add`/`sub`/`and`/`or`/`xor` on `u8` and `u16` | native and synthesized ALU sequences over `A`/`B` or `HL`/`DE` |
| `not_u8`, `not_u16` | `CPL`, applied bytewise for `u16` |
| matching `ret_u8`, `ret_bool`, `ret_u16` | `HALT` (only if returning the current value at the matching width) |
| `ret_void` | `HALT` |
| Empty CIR body | `HALT` |
| Anything else | `UnsupportedOp` from `compile()`; `None` from the `Backend::compile` trait method |

The bounded allocator supports two same-width live values. Byte and bool
values occupy `A`/`B`; word values occupy `HL`/`DE`. Binary operations consume
both operands and retain their result in `A` or `HL`. A typed return can also
materialize the secondary value into the result location. More than two live
values and mixed-width live sets fall through to `UnsupportedOp`.

Full op coverage (comparisons, branches, calls, the
alternate register bank, `CB`-prefixed bit ops, IX/IY-relative
addressing) that a mature backend would carry is **intentionally not
ported** yet — `z80-simulator` already implements a substantial
subset of the ISA these could lower to (see that crate's README for the
full inventory and the deliberate `ED`-prefix scope cut), so a future
increment to `z80-backend` has comparatively little ISA groundwork left
to do, mostly CIR-to-encoder wiring.

## Wire format

Each instruction is a variable-length (1 to 4 byte) Z80 opcode
sequence, written in execution order with no endianness conversion at
this layer (`z80-encoder`'s `encode_*` helpers already place any 16-bit
operand little-endian within the instruction). Per-function byte
streams can be concatenated directly; `lang-aot` writes them straight
to disk as a flat `.bin`.

## Pinned byte sequence

| Program | CIR | Emitted bytes |
|---------|-----|----------------|
| Twig `42` | `const_i64 v=42; ret_i64 v` | `[0x3E, 0x2A, 0x76]` |
| Word `0x1234` | `const_u16 v=0x1234; ret_u16 v` | `[0x21, 0x34, 0x12, 0x76]` |
| `ret_void` only | `ret_void` | `[0x76]` |
| Empty CIR | (none) | `[0x76]` |

`LD A, 42` = `[0x3E, 0x2A]`; `HALT` = `[0x76]`. **Byte-for-byte
identical to `intel8080-backend`'s canonical output for the same
program** — this is the cross-architecture consistency check the
migration guidance calls for explicitly, since both chips share the
`LD A, n` / `HALT` (`MVI A, n` / `HLT`) encoding. Asserted in
`tests/test_backend.rs::z80_backend_matches_intel8080_backend_byte_for_byte`
against a literal constant (see the `Cargo.toml` `[dev-dependencies]`
note for why this isn't a live dependency on `intel8080-backend` in
this worktree — that lane hadn't merged to `origin/main` yet when this
one was branched).

## Backend trait surface

| Trait method | Behaviour |
|---------------|-----------|
| `name()` | returns `"z80"` |
| `compile(ir)` | returns `Some(bytes)` for supported CIR ops; `None` otherwise |
| `compile_function(ctx, ir)` | ignores `FunctionContext` (no parameter marshalling in v0.1.0); delegates to `compile` |
| `run(binary, args)` | **panics** with `"z80 backend is emit-only; load bytes into z80-simulator to execute"` — emit-only per the migration spec |

## Error variants

| `BackendError` variant | Trigger |
|--------------------------|---------|
| `UnsupportedOp(String)` | CIR operation outside `const_*`/`ret_*` |
| `InvalidOperand(String)` | Malformed CIR operands or missing `dest` |
| `UndefinedVariable(String)` | A typed return has no current value |
| `ImmediateOutOfRange(i64)` | A literal falls outside its selected unsigned width (`u8` or `u16`) |

## Termination-check convention (Intel 8051 bug class, avoided)

A prior lane (Intel 8051) shipped a real bug caught by security review:
a defensive "is the program already terminated?" check that compared
**trailing byte values** against the halt opcode instead of tracking
whether a real halt was actually emitted — which broke when a
`const_*` immediate's byte value happened to numerically equal the
sentinel.

`z80-backend`'s `ret_*`/`ret_void` lowering always emits a REAL `HALT`
opcode (never a pseudo-halt sentinel), and its termination logic uses an
explicit boolean set only when a real halt is emitted and reset by every
subsequent constant. It never infers instruction identity from trailing byte
values. Concretely: `LD A, 0x76` followed by the real
`HALT` (`[0x3E, 0x76, 0x76]`) is compiled and executed correctly — see
`tests/test_backend.rs::const_value_equal_to_halt_opcode_byte_is_not_misread`,
which loads that exact byte sequence into `z80-simulator` and asserts
`A == 0x76` after exactly two steps. This bug class does not directly
apply here (there is no separate pseudo-halt sentinel to confuse with a
real opcode byte), but the regression test exists so it stays that way
if the lowering logic changes.

## WORD01 execution proof

`tests/test_backend.rs::word_u16_result_executes_in_hl` runs
`LD HL,0x1234; HALT` in `z80-simulator` and inspects both `H == 0x12` and
`L == 0x34`. Companion tests execute the `u8` boundary in `A`, reject 256 as a
`u8`, and reject a `ret_u8` for a current `u16` value.

## WORD02 execution proof

The backend tests execute `u8(0xff) + u8(2) == 1`,
`u16(0xffff) + u16(2) == 1`, and
`u16(0x1234) ^ u16(0x00ff) == 0x12cb` in `z80-simulator`. Word results assert
both `H` and `L`. Companion tests cover subtraction and every bitwise operation
at both widths.

## Tests

22 unit/integration tests in `tests/test_backend.rs` (mirroring
`intel8080-backend`'s test shape, plus the cross-architecture and
halt-byte-value regression tests above) pin the canonical byte sequence
and edge cases (zero, 8-bit max, immediate overflow, bool, bounded allocation,
unsupported op, empty CIR, `ret_void`, `Backend::run`
panics, `Backend::compile` vs the free `compile` function agree).

Multiple tests additionally load the compiled bytes into `z80-simulator` and
inspect `A` or both halves of `HL` after execution — byte-for-byte parity is
necessary but not sufficient; the emitted bytes must actually execute.

## Backlog

1. [ ] Expand the bounded two-value allocator with spills and the alternate
   bank via `EX AF,AF'`/`EXX` when later Word programs require it.
2. [ ] Comparisons and conditional branches (`JP cc`/`JR cc`/`DJNZ`) —
   the simulator already implements all 8 condition codes plus the
   Z80-only relative-jump forms.
3. [ ] Direct calls (`CALL`/`RET` pairing) and a stack frame — the
   simulator already implements `CALL`/`RET`/`PUSH`/`POP`.
4. [ ] `CB`-prefixed bit test/manipulation ops as CIR bitwise
   primitives — the simulator already implements the full `BIT`/`RES`/
   `SET`/rotate-shift group.
5. [ ] IX/IY-relative addressing for stack-frame-like local variables —
   the simulator currently only ports the "IX/IY basics"
   (`LD IX/IY,nn`, `INC IX/IY`); full `(IX+d)` addressing is a
   simulator-side prerequisite before the backend could use it.
6. [ ] `Backend::run` wired to `z80-simulator` for JIT execution
   (best-effort per the migration spec — "no working JIT" is an
   acceptable outcome for a historical-arch target).
