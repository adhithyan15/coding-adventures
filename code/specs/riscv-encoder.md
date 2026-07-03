# `riscv-encoder` spec

> **Status:** v0.1.0 — Phase 7 (FINAL lane) of the historical-arch
> backend migration, 2026-06-03.

## Purpose

Pure-Rust encoder for the RV32I (RISC-V 32-bit integer) base ISA.
Has no IR knowledge — its job is to turn a register/operand triple
(plus an opcode mnemonic) into a 32-bit instruction word that
matches the RV32I spec bit-for-bit.

Mirror of `aarch64-encoder` / `x86_64-encoder` / `ge225-encoder` /
`intel4004-encoder` / `armv7-encoder` / `intel8008-encoder`.

## Public surface

### Re-exported `encode_*` helpers (from `riscv-simulator::encoding`)

`riscv-simulator::encoding` is the canonical in-tree source of
truth for the RV32I bit layout — it owns the funct3/funct7
constants and the imm-bit-fiddling routines.  `riscv-encoder`
re-exports the subset of `encode_*` helpers that `riscv-backend`
actually uses.

| Mnemonic | Helper | Encoding family |
|----------|--------|-----------------|
| `addi` `slti` `sltiu` `xori` `ori` `andi` | `encode_<mnemonic>` | I-type |
| `slli` `srli` `srai`                       | `encode_<mnemonic>` | I-type shift |
| `add` `sub` `sll` `slt` `sltu` `xor` `srl` `sra` `or` `and` | `encode_<mnemonic>` | R-type |
| `lb` `lh` `lw` `lbu` `lhu`                 | `encode_<mnemonic>` | I-type load |
| `sb` `sh` `sw`                             | `encode_<mnemonic>` | S-type store |
| `beq` `bne` `blt` `bge` `bltu` `bgeu`      | `encode_<mnemonic>` | B-type branch |
| `jal` `jalr`                               | `encode_<mnemonic>` | J-/I-type |
| `lui` `auipc`                              | `encode_<mnemonic>` | U-type |
| `ecall`                                    | `encode_ecall`      | system |
| (helper) `assemble`                        | `assemble`          | `Vec<u32>` → `Vec<u8>` (little-endian) |

### Register-index constants

| Constant | Value | Role |
|----------|-------|------|
| `X0_ZERO` | `0`  | hardwired zero |
| `X1_RA`   | `1`  | return address |
| `X2_SP`   | `2`  | stack pointer (16-byte aligned per psABI) |
| `A0`      | `10` | first argument / primary return-value register |
| `TEMP_REGISTERS` | `[5, 6, 7, 28, 29, 30, 31]` | t0..t6 pool, matches `iir-to-riscv` v0.3.3 |

### Canonical word constants

| Constant | Value | Meaning |
|----------|-------|---------|
| `RET_WORD` | `0x0000_8067` | `jalr x0, x1, 0` — universal RV32I return |

## Why this crate exists

Before Phase 7, `iir-to-riscv` reached directly into
`riscv-simulator::encoding`.  That coupled the IR-to-machine-code
lowering to the simulator crate.  After Phase 7, the new
`riscv-backend` depends on `riscv-encoder`, which re-exports the
encoding helpers — the indirection lets `riscv-backend` evolve
independently and matches the encoder/backend split the rest of
the historical-arch lanes use.

Re-exporting (rather than duplicating) the encode functions keeps
`riscv-simulator::encoding` as the single source of truth.  Any
future fix to a funct3/funct7 constant or an immediate-bit layout
lands in one place and propagates automatically.

## Tests (8 byte-pinned unit tests)

* Register constants match the psABI values.
* `TEMP_REGISTERS` matches `iir-to-riscv` v0.3.3 exactly (preserves
  byte-for-byte parity during the Phase 7 migration).
* `RET_WORD == 0x0000_8067` and matches `encode_jalr(0, 1, 0)`.
* `RET_WORD.to_le_bytes() == [0x67, 0x80, 0x00, 0x00]` — the
  little-endian tail the lang-aot RV32I e2e test pins.
* `encode_addi(5, X0_ZERO, 42) == 0x02A0_0293` — first instruction
  of the Twig `42` lowering.
* `encode_addi(A0, 5, 0) == 0x0002_8513` — the mv-to-a0 prologue.
* `encode_jalr` and `encode_ecall` produce distinct words.
* `assemble(&[…])` flattens to little-endian bytes per the
  `Vec<u32>::iter().flat_map(u32::to_le_bytes)` convention.

## Out of scope

* RV32M (multiply/divide), RV32A (atomics), F/D (floating-point)
  extensions — RV32I base only.
* Disassembly or simulation — `riscv-simulator` handles both.
* Symbol resolution, linker relocations — that's `aot_core::link`
  territory.
