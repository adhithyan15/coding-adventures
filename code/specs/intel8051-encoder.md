# `intel8051-encoder` spec

> **Status:** v0.1.0 — fourth lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

Pure-Rust encoder for the Intel 8051 (MCS-51, 1980) instruction set —
the most-manufactured CPU architecture in history (20+ billion units,
still fabricated today by Atmel/Microchip, NXP, Silicon Labs, and
others). Has no IR knowledge — its job is to turn an opcode +
immediate/register tuple into the exact byte sequence the ISA defines.

Mirror of `arm1-encoder` / `mips-r2000-encoder` / `intel8008-encoder`
/ `ge225-encoder`.

## Public surface

### Re-exported helpers (from `intel8051-simulator::encoding`)

`intel8051-simulator` is the canonical in-tree source of truth for the
8051 bit layout — its `opcodes`/`decode`/`execute` modules own the
instruction semantics, and `encoding` owns the byte-layout logic.
`intel8051-encoder` re-exports the subset that `intel8051-backend`
actually uses today:

| Item | Kind | Purpose |
|------|------|---------|
| `encode_mov_a_imm(n)` | fn | `MOV A, #n` — 2 bytes: `[0x74, n]` |
| `encode_mov_rn_imm(n, imm)` | fn | `MOV Rn, #imm` — 2 bytes: `[0x78 + (n & 7), imm]` |
| `encode_halt()` | fn | the HALT sentinel — 1 byte: `0xA5` |
| `MOV_A_IMM` | const (`0x74`) | `MOV A, #imm` opcode byte |
| `HALT_OPCODE` | const (`0xA5`) | the HALT sentinel opcode byte |
| `IMM8_MAX` | const (`255`) | maximum unsigned 8-bit immediate |

The full `encode_*`/opcode surface (every data-transfer, arithmetic,
logic, bit-manipulation, jump, and subroutine instruction — see
`intel8051-simulator`'s own module docs) lives in
`intel8051_simulator::opcodes`/`encoding` for the simulator's own test
suite; this crate re-exports only what the minimal-viable backend
needs today. A future increment can widen the re-export list
alongside `intel8051-backend`'s op coverage.

## Why `MOV A, #imm` + the HALT sentinel?

The 8051's accumulator (`A`) is the implicit destination/source for
almost every arithmetic and data-transfer instruction — the same "one
working register" role the Intel 8008's `A` plays in
`intel8008-backend`. `MOV A, #imm` (opcode `0x74`, 2 bytes: an opcode
byte plus an immediate byte) is the natural "materialise a constant"
instruction, exactly mirroring `intel8008-backend`'s `MVI A, n`.

## Why the HALT sentinel, not self-jump (`SJMP $`) detection?

**There is no real HALT instruction on the 8051.** A genuinely
running 8051 program that has finished its work spins forever (`SJMP
$`, jump-to-self) or waits for the next interrupt — the chip has
nothing to hand control back to, unlike the Intel 8080/8008 (which
have a real `HLT` opcode) or a hosted architecture with an OS to
return to. Self-jump detection — recognise a fixed `SJMP $` pattern
(`[0x80, 0xFE]`) and treat it as "halted" the way a real in-circuit
debugger notices the PC has stopped advancing — is the historically
idiomatic 8051 convention, and was seriously considered for this
lane.

It was **not** used, for a concrete reason, not an aesthetic one: this
architecture already has a tested, shipped HALT convention. The
existing Python behavioral reference this Rust simulator was ported
from (`intel8051_simulator.state.HALT_OPCODE`,
`code/packages/python/intel8051-simulator`, spec 07p) already defines
opcode `0xA5` — reserved/undefined in every MCS-51 opcode map — as a
HALT sentinel, and `code/specs/07p-intel-8051-simulator.md`'s "HALT
convention" section documents it explicitly:

> The real 8051 has no HALT instruction. In this simulator, executing
> opcode `0xA5` (undefined/reserved on real hardware) is the HALT
> sentinel that stops execution and sets `halted=True`.

`intel8051-simulator::opcodes::HALT_OPCODE` ports this constant
unchanged. Inventing a second, different halt convention for the same
architecture in the same codebase — self-jump detection alongside an
already-established sentinel opcode — would fracture parity between
the Python and Rust simulators for no benefit: both now agree
byte-for-byte on what "the program is done" means, and any consumer
that already knows to look for `0xA5` (a debugger, a disassembler, a
test harness written against the Python simulator) keeps working
unchanged against the Rust port.

The sentinel is also strictly simpler to detect: `halted()` becomes a
single opcode-equality check in `intel8051_simulator::execute`, rather
than the simulator having to recognise "the *next* fetch will
re-execute the same two-byte `SJMP` at the same address" — real,
avoidable complexity for a target whose only current job is
materialising one constant and stopping. Self-jump detection remains
available as a documented option for a future increment that grows
real subroutine calls (where `ret_*` would need to mean "return to
caller", and the sentinel's "whole program is done" semantics would no
longer be quite right).

## `intel8051-simulator`: behavioral Rust port of the Python reference

`code/specs/07p-intel-8051-simulator.md` documents the **behavioral**
8051 simulator this crate wraps, originally written in Python
(`code/packages/python/intel8051-simulator`) — it executes 8051
machine code directly using host-language arithmetic (no gate-level
modelling); a separate gate-level simulation
(`code/packages/rust/intel8051-gatelevel`) is unrelated to this lane.
`intel8051-simulator`'s Rust port (`code/packages/rust/
intel8051-simulator/src/{opcodes,encoding,decode,execute,simulator}.rs`)
is a mechanical, instruction-by-instruction translation of the Python
source, preserving every reset value, flag-computation truth table,
and the HALT sentinel convention described above.

## Why this crate exists

Mirrors the encoder/backend split every other historical-arch lane
uses (see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
`intel8051-backend` (the `Backend` trait implementation over CIR)
depends on this crate rather than reaching directly into
`intel8051_simulator`'s decode/execute machinery, so the backend can
evolve independently of simulator internals and the simulator crate
stays a leaf dependency for exactly one direct consumer (this
encoder).

Re-exporting (rather than duplicating) the encode functions and
opcode constants keeps `intel8051_simulator` the single source of
truth. Any future fix to a byte-layout detail lands in one place and
propagates automatically.

## Tests (7 unit tests + 1 doctest)

* `HALT_OPCODE == 0xA5` and `encode_halt() == HALT_OPCODE`.
* `MOV_A_IMM == 0x74` and `IMM8_MAX == 255`.
* `encode_mov_a_imm(42) == [0x74, 0x2A]` — first instruction of the
  Twig `42` lowering.
* `encode_mov_a_imm(42)` followed by `encode_halt()` ==
  `[0x74, 0x2A, 0xA5]` — the full canonical byte sequence.
* `encode_mov_rn_imm(0, 7) == [0x78, 7]` — the re-exported register
  form.

Plus a doctest walking through the canonical `const 42; ret` byte
derivation.

## Out of scope

* Every other opcode's `encode_*` helper (data transfer beyond `MOV
  A, #imm`/`MOV Rn, #imm`, arithmetic, logic, bit manipulation,
  jumps, subroutines) — these exist as raw opcode constants and
  decode/execute logic in `intel8051_simulator` for the simulator's
  own test suite, but are not yet re-exported as `encode_*` helpers
  here, since `intel8051-backend` v0.1.0 does not use them. A future
  backend increment (real register allocator, arithmetic ops, control
  flow) re-exports the additional helpers it needs.
* Disassembly or simulation — `intel8051-simulator` handles both.
* Symbol resolution, linker relocations — that's `aot_core::link`
  territory.
