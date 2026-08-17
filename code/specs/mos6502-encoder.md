# `mos6502-encoder` spec

> **Status:** v0.1.0 — fifth lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

Pure-Rust encoder for the MOS Technology 6502 (1975) instruction set —
Chuck Peddle's $25 chip (versus the Intel 8080's $179) that powered the
Apple II, Commodore 64, Atari 2600/8-bit line, BBC Micro, and — via the
Ricoh 2A03 variant — the NES/Famicom.  Has no IR knowledge — its job is
to turn a mnemonic + operand into the raw byte sequence that matches the
ISA bit-for-bit.

Mirror of `mips-r2000-encoder` / `arm1-encoder` / `armv7-encoder` /
`intel8008-encoder`.

## Public surface

### Re-exported helpers (from `mos6502-simulator`)

`mos6502-simulator` is the canonical in-tree source of truth for the MOS
6502 opcode table (`opcodes::lookup`) — it owns the mnemonic-to-byte
mapping for all 151 official opcodes.  `mos6502-encoder` re-exports the
subset of `encode_*` helpers that `mos6502-backend` actually uses today,
plus a few more exercised by this crate's own tests:

| Item | Kind | Purpose |
|------|------|---------|
| `encode_lda_imm(imm)` | fn | `LDA #imm` (immediate-mode load accumulator) — `[0xA9, imm]` |
| `encode_ldx_imm(imm)` | fn | `LDX #imm` — `[0xA2, imm]` |
| `encode_ldy_imm(imm)` | fn | `LDY #imm` — `[0xA0, imm]` |
| `encode_sta_zp(zp)` | fn | `STA $zp` — `[0x85, zp]` |
| `encode_adc_imm(imm)` | fn | `ADC #imm` — `[0x69, imm]` |
| `encode_sbc_imm(imm)` | fn | `SBC #imm` — `[0xE9, imm]` |
| `encode_clc()` / `encode_sec()` | fn | `CLC`/`SEC` — `[0x18]`/`[0x38]` |
| `encode_nop()` | fn | `NOP` — `[0xEA]` |
| `encode_brk()` | fn | the HALT sentinel — `[0x00]` |
| `assemble(&[Vec<u8>])` | fn | concatenate per-instruction byte vectors (trivial — no endianness conversion) |

The full 151-opcode table (all 13 addressing modes) lives in
`mos6502_simulator::opcodes` for the decoder; this crate re-exports only
the mnemonics `mos6502-backend` needs today.  A future increment can
widen the re-export list alongside the backend's op coverage (the
simulator already implements every mnemonic — only the *encoder*
re-export list is intentionally narrow).

### Canonical byte constant

| Constant | Value | Meaning |
|----------|-------|---------|
| `HALT_BYTE` | `0x00` | `BRK` — the byte `mos6502-simulator::execute` intercepts to set `halted = true` |

Unlike `mips_r2000_encoder::RET_WORD` (a 32-bit word constant) or
`arm1_encoder::HALT_WORD` (same), `HALT_BYTE` is a single byte — the
6502 has no word endianness to speak of.

## Why `BRK`, not KIL/JAM or a self-jump spin loop?

Three halt-related possibilities exist for a halt-less-in-silicon-sense
8-bit CPU backend:

1. **A real halt-like instruction the existing in-tree simulator already
   treats specially.**  This is what `mos6502-encoder`/`mos6502-backend`
   use: `BRK` (opcode `0x00`).
2. **An illegal/undocumented opcode that locks the CPU** (`KIL`/`JAM`,
   e.g. `0x02`) — some real-world 6502 test-suite emulators treat this
   as "halted" since the chip stops fetching.  Considered and
   **rejected** for this lane.
3. **A self-targeting `JMP $addr` spin loop** — the convention a
   different halt-less architecture elsewhere in this 9-architecture
   expansion uses.  Also considered and **rejected** here.

The deciding factor: `mos6502-simulator` is a **direct Rust port** of an
**existing, in-tree Python simulator**
(`code/packages/python/mos6502-simulator`) that already documents its
own halt convention — quoting that package's `simulator.py` module
docstring verbatim:

> *"Halt condition: BRK (opcode 0x00) is treated as HALT — the simulator
> stops and sets `halted=True` in the state. This matches the convention
> used throughout the simulator stack (HLT for 8080, TRAP for IBM 704,
> etc.)."*

This is unlike ARM1 (1985), whose Rust `arm1-simulator` (also an in-tree
port, but of an ISA with **no real halt instruction at all**) had to
*invent* a pseudo-halt (`SWI #0x123456`) because ARMv1 silicon has no
native way to stop — a real ARMv1 program spins or awaits an interrupt.
The MOS 6502 is different: `BRK` is a real, single-byte, already-decoded
instruction, and the **pre-existing** simulator this crate ports already
gives it HALT semantics.  Choosing anything else (KIL/JAM, a spin loop)
would mean this backend's compiled programs stop meaning "halted" the
moment they're loaded into any *other* in-tree 6502 tooling that already
assumes `BRK`-means-stop — every test program in
`code/specs/07j-mos6502-simulator.md`'s "Test Programs" section ends in
`BRK`.  Mirroring the **established, existing** semantics for this
specific ISA in this repo is unambiguously correct here; it is not a
judgment call the way ARM1's pseudo-halt invention was.

## `mos6502-simulator`: full behavioral port, not a narrowed subset

Unlike `arm1-encoder` (whose backing simulator, `arm1-simulator`,
pre-existed complete in-tree before this expansion), `mos6502-simulator`
is new — but it ports `code/packages/python/mos6502-simulator`'s
**entire** 151-opcode / 13-addressing-mode instruction set (including
BCD decimal-mode `ADC`/`SBC` and the documented indirect-`JMP`
page-wrap silicon bug), not just the `LDA #imm` + `BRK` sequence this
encoder's backend consumer needs.  This mirrors how the Intel 8080 lane
ported its full ISA when the Python reference was already fully worked
out — see `code/specs/mos6502-backend.md` and
`code/packages/rust/mos6502-simulator/README.md` for the full
simulator writeup; this encoder crate documents only the byte-level
encoding surface it re-exports.

## Why this crate exists

Mirrors the encoder/backend split every other historical-arch lane uses
(see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
`mos6502-backend` (the `Backend` trait implementation over CIR) depends
on this crate rather than reaching directly into `mos6502_simulator`'s
decode/execute machinery, so the backend can evolve independently of
simulator internals and the simulator crate stays a leaf dependency for
exactly one direct consumer (this encoder).

Re-exporting (rather than duplicating) the encode functions keeps
`mos6502_simulator` the single source of truth.  Any future fix to the
opcode table lands in one place and propagates automatically.

## Tests (10 unit tests + 1 doctest)

* `HALT_BYTE == mos6502_simulator::opcodes::BRK_OPCODE` and
  `HALT_BYTE == 0x00`.
* `HALT_BYTE` matches `encode_brk()`'s single byte.
* `encode_lda_imm(42) == [0xA9, 42]` — first instruction of the IIR `42`
  lowering.
* `assemble(&[encode_lda_imm(42), encode_brk()]) == [0xA9, 42, 0x00]`.

Plus a doctest walking through the canonical `const 42; ret` byte
derivation.

## Out of scope

* Encoders for the remaining ~140 opcodes not listed above — these exist
  in `mos6502_simulator::opcodes`/`encoding` for the simulator's own
  test suite but are not yet re-exported here, since `mos6502-backend`
  v0.1.0 does not use them.  A future backend increment (real register
  allocator, arithmetic ops, control flow, addressing modes beyond
  immediate/zero-page) re-exports the additional helpers it needs.
* Disassembly or simulation — `mos6502-simulator` handles both.
* Symbol resolution, linker relocations — that's `aot_core::link`
  territory.
