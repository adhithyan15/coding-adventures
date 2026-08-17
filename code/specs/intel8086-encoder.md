# `intel8086-encoder` spec

> **Status:** v0.1.0 — ninth and **final** lane of the 9-architecture
> expansion, 2026-08-17.

## Purpose

Pure-Rust encoder for a curated core of the Intel 8086 (1978) instruction
set — the 16-bit extension of the 8080 architecture (NOT source- or
binary-compatible with it, despite the lineage) that introduced the
segmented memory model and the ModRM addressing byte. The IBM PC (1981)
shipped with the 8086's cheaper 8-bit-external-bus sibling, the 8088,
founding the "PC-compatible" industry — the Intel 8086 is the direct
architectural ancestor of every x86 CPU made today. Has no IR knowledge
— its job is to turn a mnemonic + operand into the raw byte sequence
that matches the ISA bit-for-bit.

Mirror of `mos6502-encoder` / `arm1-encoder` / `riscv-encoder`.

## Public surface

### Re-exported helpers (from `intel8086-simulator`)

`intel8086-simulator` is the canonical in-tree source of truth for this
lane's curated Intel 8086 opcode subset (`opcodes::lookup`) — it owns
the mnemonic-to-byte mapping. `intel8086-encoder` re-exports the subset
of `encode_*` helpers that `intel8086-backend` actually uses today, plus
a few more exercised by this crate's own tests:

| Item | Kind | Purpose |
|------|------|---------|
| `encode_mov_reg_imm16(reg, imm)` | fn | `MOV reg16,#imm16` — `[0xB8+reg, imm_lo, imm_hi]` (little-endian) |
| `encode_mov_reg_imm8(reg, imm)` | fn | `MOV reg8,#imm8` — `[0xB0+reg, imm]` |
| `encode_mov_reg_reg16(dest, src)` | fn | register-to-register `MOV reg16,r/m16` — `[0x8B, modrm]` (`mod=11`) |
| `encode_add_ax_imm16(imm)` / `encode_sub_ax_imm16(imm)` | fn | `ADD`/`SUB AX,#imm16` |
| `encode_inc_reg16(reg)` / `encode_dec_reg16(reg)` | fn | `INC`/`DEC reg16` |
| `encode_nop()` | fn | `NOP` — `[0x90]` |
| `encode_hlt()` | fn | the HALT sentinel — `[0xF4]` |
| `assemble(&[Vec<u8>])` | fn | concatenate per-instruction byte vectors (no fixed-width flattening needed) |

The full curated opcode table lives in `intel8086_simulator::opcodes`
for the decoder; this crate re-exports only the mnemonics
`intel8086-backend` needs today plus a handful more for test coverage.

### Canonical byte constant

| Constant | Value | Meaning |
|----------|-------|---------|
| `HALT_BYTE` | `0xF4` | `HLT` — the byte `intel8086-simulator::execute` intercepts to set `halted = true` |

### Register constant

| Constant | Value | Meaning |
|----------|-------|---------|
| `REG_AX` | `0` | the accumulator register index, re-exported from `intel8086_simulator::opcodes` so `intel8086-backend` doesn't need a direct `intel8086-simulator` dependency just to name its always-target register |

## Why `HLT`, not a pseudo-halt or repurposed opcode?

This is the **least-invented** halt-related decision in the entire
9-architecture expansion. Compare:

- **ARM1** (1985) has no real halt instruction at all — ARMv1 silicon
  can only spin or await an interrupt, so `arm1-backend` had to invent a
  pseudo-halt (`SWI #0x123456`, intercepted specially by
  `arm1-simulator`'s `execute_swi`).
- **MOS 6502** (1975) has `BRK`, technically a software-interrupt
  opcode — this repo's simulator stack (both the pre-existing Python
  original and its Rust port) *treats* it as HALT by an established
  convention, but it is not what the opcode does on genuine silicon in
  the general case.
- **Intel 8086** (1978) has `HLT` (opcode `0xF4`): a genuine, single-
  byte, no-operand hardware instruction whose *sole documented purpose*
  is to stop the CPU's fetch-decode-execute loop until the next
  interrupt (or `RESET`, on real silicon). This is real hardware
  behaviour, ported directly from `code/packages/python/
  intel-8086-simulator`'s `simulator.py`:
  `if op == 0xF4: self._halted = True; return "HLT"`.

No pseudo-halt invention, no repurposed-opcode convention — `ret_*`
lowering to `HLT` is the most direct choice of any lane in this
campaign.

## Quick start

```rust
use intel8086_encoder::{encode_mov_reg_imm16, encode_hlt, assemble, HALT_BYTE, REG_AX};

// const_i64 v=42 lowered to `MOV AX,42` -- the first instruction
// intel8086-backend emits for the canonical IIR `const 42; ret` program.
let bytes = assemble(&[encode_mov_reg_imm16(REG_AX, 42), encode_hlt()]);
assert_eq!(bytes, vec![0xB8, 42, 0x00, 0xF4]);
assert_eq!(HALT_BYTE, 0xF4);
```

## Why this crate exists

Mirrors the encoder/backend split every other historical-arch lane uses
(see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
`intel8086-backend` (the `Backend` trait implementation over CIR)
depends on this crate rather than reaching directly into
`intel8086_simulator`'s decode/execute machinery, so the backend can
evolve independently of simulator internals and the simulator crate
stays a leaf dependency for exactly one direct consumer (this encoder).

Re-exporting (rather than duplicating) the encode functions keeps
`intel8086_simulator` the single source of truth. Any future fix to the
opcode table lands in one place and propagates automatically.

## Tests (6 unit tests + 1 doctest)

* `HALT_BYTE == intel8086_simulator::opcodes::HLT_OPCODE` and
  `HALT_BYTE == 0xF4`.
* `HALT_BYTE` matches `encode_hlt()`'s single byte.
* `REG_AX == 0`.
* `encode_mov_reg_imm16(REG_AX, 42) == [0xB8, 42, 0x00]` — first
  instruction of the IIR `42` lowering.
* `assemble(&[encode_mov_reg_imm16(REG_AX, 42), encode_hlt()]) ==
  [0xB8, 42, 0x00, 0xF4]`.

Plus a doctest walking through the canonical `const 42; ret` byte
derivation.

## Out of scope

* Encoders for anything outside this lane's curated opcode subset —
  memory-operand addressing, string ops, stack ops, control flow,
  `MUL`/`DIV`, shift/rotate, BCD adjust, and more (see
  `intel8086-simulator`'s crate-level doc for the full "deferred" list).
* Disassembly or simulation — `intel8086-simulator` handles both.
* Symbol resolution, linker relocations — that's `aot_core::link`
  territory.
