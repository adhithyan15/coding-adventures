# `mips-r2000-encoder` spec

> **Status:** v0.1.0 — first lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

Pure-Rust encoder for the MIPS R2000 (1985) instruction set — the
first commercially successful RISC processor, designed by John
Hennessy's team at Stanford.  Has no IR knowledge — its job is to
turn a register/operand triple (plus an opcode mnemonic) into a
32-bit instruction word that matches the ISA bit-for-bit.

Mirror of `riscv-encoder` / `armv7-encoder` / `intel8008-encoder` /
`ge225-encoder`.

## Public surface

### Re-exported `encode_*` helpers (from `mips-r2000-simulator::encoding`)

`mips-r2000-simulator::encoding` is the canonical in-tree source of
truth for the MIPS R2000 bit layout — it owns the opcode/funct
constants and the R/I/J-format packing routines.  `mips-r2000-encoder`
re-exports the subset of `encode_*` helpers that `mips-r2000-backend`
actually uses today:

| Mnemonic | Helper | Encoding family |
|----------|--------|-----------------|
| `addiu`  | `encode_addiu` | I-type |
| `jr`     | `encode_jr`    | R-type |
| (helper) `assemble` | `assemble` | `Vec<u32>` → `Vec<u8>` (**big-endian**) |

The full `encode_*` surface (30+ mnemonics — R-type ALU/shifts,
I-type arithmetic/logic/loads/stores/branches, J-type jumps) lives in
`mips-r2000-simulator::encoding` for the simulator's own test suite;
this crate re-exports only what the minimal-viable backend needs
today.  A future increment can widen the re-export list alongside
`mips-r2000-backend`'s op coverage.

### Register-role constants

| Constant | Value | Role |
|----------|-------|------|
| `ZERO` | `0`  | hardwired zero |
| `V0`   | `2`  | return value (also syscall number on MIPS Linux) |
| `V1`   | `3`  | second return-value word |
| `A0`   | `4`  | first argument register |
| `SP`   | `29` | stack pointer |
| `RA`   | `31` | return address, set by `JAL`/`JALR` |
| `TEMP_REGISTERS` | `[8, 9, 10, 11, 12, 13, 14, 15]` | `$t0..$t7` pool for a future register allocator |

### Canonical word constants

| Constant | Value | Meaning |
|----------|-------|---------|
| `RET_WORD` | `0x03E0_0008` | `JR $ra` — universal MIPS R2000 return |

Unlike RISC-V's `jalr x0, x1, 0` (which carries an immediate field
that could in principle vary), MIPS R2000's `JR rs` has **no**
immediate — the word is fixed for a given `rs`, so `RET_WORD` is a
plain constant rather than requiring a function call to derive.

## Why this crate exists

Mirrors the encoder/backend split every other historical-arch lane
uses (see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
`mips-r2000-backend` (the `Backend` trait implementation over CIR)
depends on this crate rather than reaching directly into
`mips-r2000-simulator::encoding`, so the backend can evolve
independently of simulator internals and the simulator crate stays
a leaf dependency for exactly one direct consumer (this encoder).

Re-exporting (rather than duplicating) the encode functions keeps
`mips-r2000-simulator::encoding` the single source of truth.  Any
future fix to an opcode/funct constant or an immediate-bit layout
lands in one place and propagates automatically.

## Tests (8 unit tests)

* `RET_WORD == encode_jr(RA)` and `RET_WORD == 0x03E0_0008`.
* `RET_WORD.to_be_bytes() == [0x03, 0xE0, 0x00, 0x08]` — the
  big-endian tail the `lang-aot` MIPS R2000 e2e smoke test pins.
* Register constants match the documented convention (`ZERO=0`,
  `V0=2`, `V1=3`, `A0=4`, `SP=29`, `RA=31`).
* `TEMP_REGISTERS == [8..15]`.
* `encode_addiu(V0, ZERO, 42) == 0x2402_002A` — first instruction of
  the Twig `42` lowering.
* `assemble(&[RET_WORD])` flattens to big-endian bytes.

Plus a doctest walking through the canonical `const 42; ret` byte
derivation.

## Out of scope

* R-type ALU/shift encoders, I-type arithmetic/logic/load/store
  encoders, J-type `j`/`jal` encoders — these exist in
  `mips-r2000-simulator::encoding` for the simulator's own test
  suite but are not yet re-exported here, since `mips-r2000-backend`
  v0.1.0 does not use them.  A future backend increment (real
  register allocator, arithmetic ops, control flow) re-exports the
  additional mnemonics it needs.
* Disassembly or simulation — `mips-r2000-simulator` handles both.
* Symbol resolution, linker relocations — that's `aot_core::link`
  territory.
