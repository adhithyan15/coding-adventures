# `sparc-v8-encoder` spec

> **Status:** v0.1.0 — sixth lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

Pure-Rust encoder for the SPARC V8 (1987) instruction set — the first
**open** RISC instruction-set standard, designed by Sun Microsystems
and later powering Sun SPARCstation workstations and Solaris servers
for two decades.  Has no IR knowledge — its job is to turn a
register/opcode/immediate tuple into a 32-bit instruction word that
matches the ISA bit-for-bit.

Mirror of `mips-r2000-encoder` / `arm1-encoder` / `intel8008-encoder`.

## Public surface

### Re-exported helpers (from `sparc-v8-simulator`)

`sparc-v8-simulator` is the canonical in-tree source of truth for the
SPARC V8 bit layout — it owns the op/op2/op3 field packing logic in
`encoding.rs`.  `sparc-v8-encoder` re-exports the subset that
`sparc-v8-backend` actually uses today:

| Item | Kind | Purpose |
|------|------|---------|
| `encode_add_imm(rd, rs1, simm13)` | fn | `ADD rd, rs1, simm13` (sign-extended 13-bit immediate) |
| `assemble(&[u32])` | fn | flatten instruction words to big-endian bytes |
| `HALT_WORD` | const (`0x91D0_2000`) | `ta 0` — trap always, software trap #0, this simulator's HALT sentinel |

The full `encode_*` surface (all Format 1/2/3 ALU/memory/branch
encoders — see `sparc-v8-simulator`'s own module docs) lives in
`sparc_v8_simulator::encoding` for the simulator's own test suite;
this crate re-exports only what the minimal-viable backend needs
today.  A future increment can widen the re-export list alongside
`sparc-v8-backend`'s op coverage (e.g. `encode_sethi` for wider
constants, `encode_bicc` for control flow, `encode_save`/
`encode_restore` for real function calls).

### Register-role constants

| Constant | Value | Role |
|----------|-------|------|
| `G0` | `0` | `%g0` — hardwired zero, used as `rs1` for `ADD %g0, imm, rd` |
| `O0` | `8` | `%o0` — the SPARC calling-convention return-value register |

### Why `%o0`, not a `%g` register, for the return value?

Real SPARC ABI convention (the "C calling convention" documented in
the SPARC V8 manual and every SunOS/Solaris ABI doc) returns integer
values in `%o0` — the register that becomes `%i0` in the *caller's*
view once the callee's `RESTORE` rotates the window back.  `%g1`-`%g7`
are explicitly reserved as scratch/library-private registers, not the
return-value slot — using one of them would be architecturally
inauthentic, even though it would have sidestepped the
register-window question entirely.

`%o0` (virtual register 8) is **not** one of the 8 CWP-independent
globals — it is a windowed register, physically `8 + CWP*16` when
read or written.  This lane's v0.1.0 scope never executes `SAVE`/
`RESTORE`, so CWP is always `0` for the lifetime of a compiled
program: `%o0` therefore always resolves to the same fixed physical
register (index 8), exactly as if it were a global, with zero risk of
window-rotation surprises.  Using `%o0` does not reintroduce the
windowing complexity the task explicitly permits sidestepping — the
window literally never moves in this lane's v0.1.0 programs.

### Canonical word constant

| Constant | Value | Meaning |
|----------|-------|---------|
| `HALT_WORD` | `0x91D0_2000` | `ta 0` — trap always, software trap #0 — the HALT sentinel `sparc-v8-simulator`'s executor intercepts to stop the fetch-decode-execute loop |

Derivation (Format 3i, `op=OP_ALU`, `op3=OP3_TICC`, `rd`/cond field
`= COND_BA = 8`, `rs1=0`, `i=1`, `simm13=0`):

```text
(OP_ALU << 30) | (COND_BA << 25) | (OP3_TICC << 19) | (1 << 13)
  = (0b10 << 30) | (0x8 << 25) | (0x3A << 19) | (1 << 13)
  = 0x91D0_2000
```

This matches `sparc_v8_simulator::opcodes::HALT_WORD`, which in turn
matches the Python original
(`code/packages/python/sparc-v8-simulator/src/sparc_v8_simulator/state.py`'s
`HALT_WORD`) bit-for-bit.

## Why `ta 0`, not `RESTORE` + `JMPL`?

A real SPARC subroutine returns via `RESTORE %g0, %g0, %g0` (undo the
register window) followed by `JMPL %i7+8, %g0` (return to the caller,
skipping the two-instruction CALL-annotation slot).  Both require a
live caller context (a `%i7` set by a preceding `CALL`) that the
minimal-viable `const_*`/`ret_*` scope never establishes — there is no
caller for a trivial ROM.

The Python `sparc-v8-simulator` reference already defines exactly the
right primitive for "the program is done": `ta 0` (trap always,
software trap #0), matching the SPARC/Solaris `ta 0`-as-debugger-
breakpoint idiom (distinct from `ta 1` = `sys_exit` on real SPARC
Linux/SunOS ABIs).  This is a simulator-level halt convention already
established in the existing Python reference — not invented for this
lane — parallel to `arm1-backend`'s pseudo-halt `SWI #0x123456` for
ARM1 (which also predates a clean return-from-subroutine convention
for a caller-less program).

## SPARC V8 instruction formats (background)

```text
Format 1 (op=01):    [op:2][disp30:30]                        -- CALL
Format 2 (op=00):    [op:2][rd:5][op2:3][imm22:22]             -- SETHI, Bicc, NOP
Format 3r (op=10/11): [op:2][rd:5][op3:6][rs1:5][0][asi:8][rs2:5]  -- register operand
Format 3i (op=10/11): [op:2][rd:5][op3:6][rs1:5][1][simm13:13]     -- 13-bit sign-extended immediate
```

`op=10` (`OP_ALU`) selects the ALU op3 family (arithmetic/logic/
shift/`SAVE`/`RESTORE`/`Ticc`/…); `op=11` (`OP_MEM`) selects the
memory op3 family (`LD`/`ST`/…).  `sparc-v8-encoder`'s
`encode_add_imm` builds a Format 3i word with `op3 = OP3_ADD = 0x00`.

## `sparc-v8-simulator`: the full-ISA behavioral port this encoder wraps

`sparc-v8-simulator` implements every Format 1/2/3 instruction the
Python original (`code/packages/python/sparc-v8-simulator`) supports,
including the complete overlapping-register-window machinery
(`SAVE`/`RESTORE`, `virt_to_phys`).  `sparc-v8-encoder` re-exports only
the narrow slice `sparc-v8-backend` v0.1.0 needs; see that crate's
own README for the full instruction inventory.

## Why this crate exists

Mirrors the encoder/backend split every other historical-arch lane
uses (see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
`sparc-v8-backend` (the `Backend` trait implementation over CIR)
depends on this crate rather than reaching directly into
`sparc_v8_simulator`'s decode/execute machinery, so the backend can
evolve independently of simulator internals and the simulator crate
stays a leaf dependency for exactly one direct consumer (this
encoder).

Re-exporting (rather than duplicating) the encode functions keeps
`sparc_v8_simulator` the single source of truth.  Any future fix to an
op/op2/op3 field layout lands in one place and propagates
automatically.

## Tests (6 unit tests + 1 doctest)

* `HALT_WORD == encode_ta(0)` and `HALT_WORD == 0x91D0_2000`.
* `HALT_WORD.to_be_bytes() == [0x91, 0xD0, 0x20, 0x00]` — the
  big-endian tail the `lang-aot` SPARC V8 e2e smoke test pins.
* `G0 == 0`, `O0 == 8`.
* `encode_add_imm(O0, G0, 42) == 0x9000_202A` — first instruction of
  the Twig `42` lowering.
* Big-endian byte layout of the canonical `const 42` word.

Plus a doctest walking through the canonical `const 42; ret` word
derivation.

## Out of scope

* `SETHI`/`Bicc`/`SAVE`/`RESTORE`/register-register ALU encoders —
  these exist in `sparc_v8_simulator::encoding` for the simulator's
  own test suite but are not yet re-exported here, since
  `sparc-v8-backend` v0.1.0 does not use them.  A future backend
  increment (real register allocator, wider constants via `SETHI`,
  arithmetic ops, control flow, real function calls via `SAVE`/
  `RESTORE`) re-exports the additional helpers it needs.
* Disassembly or simulation — `sparc-v8-simulator` handles both.
* Symbol resolution, linker relocations — that's `aot_core::link`
  territory.
