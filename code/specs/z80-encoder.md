# `z80-encoder` spec

> **Status:** v0.1.0 — seventh lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

Pure-Rust encoder for the Zilog Z80 (1976) instruction set — one of the
most widely produced microprocessors ever, powering the TRS-80, ZX
Spectrum, MSX, the original Game Boy (via a variant core), and
countless CP/M machines. Has no IR knowledge — its job is to turn a
register/operand pair (plus an opcode mnemonic) into a byte sequence
that matches the ISA bit-for-bit.

Mirror of `mips-r2000-encoder` / `intel8080-encoder`.

## Why a new crate (not folding into `intel8080-encoder`)?

The Z80 (1976) is the Intel 8080's direct architectural successor and a
full superset of its opcode set: every valid 8080 instruction is a
valid Z80 instruction with identical semantics and identical byte
encoding. But the Z80 also adds a genuinely larger opcode space on top
— an alternate register bank, index registers IX/IY, and four
prefix-byte families (`CB`/`ED`/`DD`/`FD`) that open secondary
instruction spaces the 8080 has no equivalent of. Sharing bit patterns
across two different (if related) ISAs in one crate would force every
`encode_*` call site to reason about which chip it's targeting, and
would tangle the 8080's flat single-space decode with the Z80's
prefix-dispatch decode; a separate crate keeps each ISA's encoding
table self-contained — the same reasoning that already kept
`intel4004-encoder` and `intel8008-encoder` separate despite both being
early Intel microprocessors.

This backend's `HALT`-based return mechanism is the **same real-hardware
convention** `intel8080-backend` already uses (unlike ARM1's SWI
pseudo-halt or MIPS's `JR $ra`) — the Z80 kept the 8080's `HLT` bit
pattern (`01_110_110` = `0x76`) verbatim as its own `HALT` opcode.

## Byte-identical to `intel8080-encoder` for the shared subset

Per the GUIDING CONSTRAINT called out for this lane: the Z80's
`LD A, n` / `HALT` — the only two instructions the minimal-viable
`z80-backend` emits — are **byte-identical** to the 8080's
`MVI A, n` / `HLT`:

| Zilog mnemonic | Intel mnemonic | Bytes | Byte-identical? |
|-----------------|------------------|-------|:---:|
| `LD A, n` | `MVI A, n` | `0x3E nn` | ✓ |
| `HALT` | `HLT` | `0x76` | ✓ |

`z80-encoder::encode_ld_a_n(42) == [0x3E, 0x2A] == intel8080_encoder::encode_mvi_a(42)`,
and `z80_encoder::HALT == 0x76 == intel8080_encoder::HLT`. This is
proven directly in `z80-encoder`'s own test suite
(`canonical_const_42_matches_intel8080_encoder_bytes`) and in
`z80-backend`'s cross-architecture test
(`z80_backend_matches_intel8080_backend_byte_for_byte`) — see that
crate's spec for why the comparison is pinned as a literal constant
rather than a live dependency on `intel8080-backend` in this worktree
snapshot.

More broadly, every unprefixed 8080-legacy opcode this crate's parent
simulator (`z80-simulator`) implements shares its byte encoding with
`intel8080-simulator`'s opcode table — `LD r,r'`/`MOV`, `LD rp,nn`/
`LXI`, the ALU groups, `JP`/`JMP`, `CALL`, `RET`, `PUSH`/`POP`, `IN`/
`OUT`, `EI`/`DI`, and the rotate/misc-accumulator group (`RLCA`/`RRCA`/
`RLA`/`RRA`/`DAA`/`CPL`/`SCF`/`CCF`, which are Zilog's renamed but
byte-identical `RLC`/`RRC`/`RAL`/`RAR`/`DAA`/`CMA`/`STC`/`CMC`) all
carry over unchanged. Only the Z80-only unprefixed bytes (`EX AF,AF'`
`0x08`, `EXX` `0xD9`, `DJNZ` `0x10`, `JR` + its 4 conditional forms
`0x18`/`0x20`/`0x28`/`0x30`/`0x38`) and the four prefix bytes (`CB`/
`ED`/`DD`/`FD`) are new — and Zilog deliberately chose bytes that are
**undefined/reserved on a stock 8080** for every one of them, so a
period 8080 program never collides with a Z80-only opcode.

## Public surface

### Re-exported `encode_*` helpers (from `z80-simulator::encoding`)

`z80-simulator::encoding` is the canonical in-tree source of truth for
the Z80 bit layout — it owns the opcode constants and the byte-sequence
packing routines for the full ported ISA. `z80-encoder` re-exports only
the subset `z80-backend` actually uses today:

| Mnemonic | Helper | Bytes |
|----------|--------|-------|
| `LD A, n` | `encode_ld_a_n` | 2 (`0x3E nn`) — byte-identical to `intel8080_encoder::encode_mvi_a` |
| (helper) `assemble` | `assemble` | `&[Vec<u8>]` → `Vec<u8>` (flatten; no endianness conversion at this layer) |

The full `encode_*` surface (`LD r,r'`, `LD rp,nn`, ALU reg/imm, all
conditional jump/call/return forms, `RST`, `PUSH`/`POP`, I/O, rotates,
`DAA`, plus the Z80-only `EX AF,AF'`/`EXX`/`DJNZ`/`JR`-family/`CB`-
prefixed bit ops/`DD`-`FD` IX-IY basics) lives in
`z80-simulator::encoding` for the simulator's own test suite; this
crate re-exports only what the minimal-viable backend needs today. A
future increment can widen the re-export list alongside
`z80-backend`'s op coverage.

### Opcode / register constants

| Constant | Value | Meaning |
|----------|-------|---------|
| `HALT` | `0x76` | halt — the universal Z80 "return from entry function"; byte-identical to `intel8080_encoder::HLT` |
| `RET` | `0xC9` | unconditional return (for non-entry functions, a future increment) |
| `REG_A` | `7` | 3-bit register-field code for the accumulator |
| `LD_A_N_MAX` | `255` | maximum unsigned 8-bit `LD A,n` immediate |

Like the 8080, the Z80 names its working registers (A, B, C, D, E, H,
L) rather than numbering them — `z80-backend` addresses the accumulator
(`REG_A`) directly rather than through an indexed register file.

## Tests (8 unit tests)

* `HALT == 0x76`, `RET == 0xC9`.
* `REG_A == 7`.
* `encode_ld_a_n(42) == [0x3E, 0x2A]` — first (and only) instruction of
  the Twig `42` lowering.
* `canonical_const_42_matches_intel8080_encoder_bytes` — the direct
  byte-identity assertion against the literal 8080-legacy encoding.
* `assemble(&[encode_ld_a_n(42), vec![HALT]]) == [0x3E, 0x2A, 0x76]` —
  the exact byte sequence the `lang-aot` Z80 e2e smoke test pins.
* `LD_A_N_MAX == 255`.

Plus a doctest walking through the canonical `const 42; ret` byte
derivation.

## Out of scope

* `LD r,r'`/`LD rp,nn`/ALU/branch/call/stack/I/O/Z80-only encoders —
  these exist in `z80-simulator::encoding` for the simulator's own test
  suite but are not yet re-exported here, since `z80-backend` v0.1.0
  does not use them. A future backend increment (real register
  allocator, arithmetic ops, control flow, alternate-bank swaps,
  bit-manipulation ops, IX/IY-relative addressing) re-exports the
  additional mnemonics it needs.
* Disassembly or simulation — `z80-simulator` handles both.
* Symbol resolution, linker relocations — that's `aot_core::link`
  territory.
* The `ED`-prefix opcode space — not ported anywhere in this lane; see
  `z80-simulator`'s spec/README for the deliberate scope cut.
