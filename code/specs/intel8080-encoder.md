# `intel8080-encoder` spec

> **Status:** v0.1.0 — third lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

Pure-Rust encoder for the Intel 8080 (1974) instruction set — Intel's
first widely successful 8-bit microprocessor, direct successor to the
8008, and the CPU inside the Altair 8800 that launched the
personal-computer era. Has no IR knowledge — its job is to turn a
register/operand pair (plus an opcode mnemonic) into a byte sequence
that matches the ISA bit-for-bit.

Mirror of `mips-r2000-encoder` / `intel8008-encoder` / `armv7-encoder`.

## Why a new crate (not folding into `intel8008-encoder`)?

The Intel 8080 (1974) is the 8008's direct architectural successor:
same 8-bit accumulator model, same `HLT` halt convention, but a real
16-bit address space (64 KiB vs the 8008's 16 KiB), a RAM-based stack
(vs the 8008's internal 8-level push-down stack), 256 I/O ports (vs 8
in / 24 out), and a materially different — though related — opcode
layout (`HLT = 0x76` on both chips, but almost everything else moved).
Sharing bit patterns across two different ISAs in one crate would
force every `encode_*` call site to reason about which chip it's
targeting; a separate crate keeps each ISA's encoding table
self-contained, matching how `intel4004-encoder` and `intel8008-encoder`
already stay separate despite both being early Intel microprocessors.

This backend's `HLT`-based return mechanism is the **same real-hardware
convention** `intel8008-backend` already uses (unlike ARM1's SWI
pseudo-halt or MIPS's `JR $ra`) — the 8080 kept `MOV M,M`'s bit pattern
(`01_110_110` = `0x76`) as the halt sentinel, exactly as the 8008 did.

## Public surface

### Re-exported `encode_*` helpers (from `intel8080-simulator::encoding`)

`intel8080-simulator::encoding` is the canonical in-tree source of
truth for the Intel 8080 bit layout — it owns the opcode constants and
the byte-sequence packing routines for the full ISA. `intel8080-encoder`
re-exports only the subset `intel8080-backend` actually uses today:

| Mnemonic | Helper | Bytes |
|----------|--------|-------|
| `MVI A, n` | `encode_mvi_a` | 2 (`0x3E nn`) |
| (helper) `assemble` | `assemble` | `&[Vec<u8>]` → `Vec<u8>` (flatten; no endianness conversion at this layer) |

The full `encode_*` surface (MOV, LXI, INX/DCX/DAD, INR/DCR, ALU
reg/imm, all conditional jump/call/return forms, RST, PUSH/POP, I/O,
rotates, DAA, …) lives in `intel8080-simulator::encoding` for the
simulator's own test suite; this crate re-exports only what the
minimal-viable backend needs today. A future increment can widen the
re-export list alongside `intel8080-backend`'s op coverage.

### Opcode / register constants

| Constant | Value | Meaning |
|----------|-------|---------|
| `HLT` | `0x76` | halt — the universal Intel 8080 "return from entry function" |
| `RET` | `0xC9` | unconditional return (for non-entry functions, a future increment) |
| `REG_A`..`REG_M` | `0..7` | 3-bit register-field codes (B,C,D,E,H,L,M,A) |
| `MVI_MAX` | `255` | maximum unsigned 8-bit `MVI` immediate |

Unlike MIPS/ARM's numbered registers (`R0`..`R31`), the 8080 names its
working registers (A, B, C, D, E, H, L) — `intel8080-backend` addresses
the accumulator (`REG_A`) directly rather than through an indexed
register file.

## Tests (9 unit tests + 9 integration tests)

* `HLT == 0x76`, `RET == 0xC9`.
* Register-code constants match the 8080's documented 3-bit encoding
  (`REG_B=0` .. `REG_A=7`, `REG_M=6`).
* `encode_mvi_a(42) == [0x3E, 0x2A]` — first (and only) instruction of
  the Twig `42` lowering.
* `assemble(&[encode_mvi_a(42), vec![HLT]]) == [0x3E, 0x2A, 0x76]` —
  the exact byte sequence the `lang-aot` Intel 8080 e2e smoke test pins.
* `MVI_MAX == 255`.

Plus a doctest walking through the canonical `const 42; ret` byte
derivation.

## Out of scope

* MOV/LXI/ALU/branch/call/stack/I/O encoders — these exist in
  `intel8080-simulator::encoding` for the simulator's own test suite
  but are not yet re-exported here, since `intel8080-backend` v0.1.0
  does not use them. A future backend increment (real register
  allocator, arithmetic ops, control flow) re-exports the additional
  mnemonics it needs.
* Disassembly or simulation — `intel8080-simulator` handles both.
* Symbol resolution, linker relocations — that's `aot_core::link`
  territory.
