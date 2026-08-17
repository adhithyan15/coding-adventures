# `arm1-encoder` spec

> **Status:** v0.1.0 — second lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

Pure-Rust encoder for the ARM1 (ARMv1, 1985) instruction set —
Sophie Wilson and Steve Furber's original Acorn RISC Machine, the
first commercially successful RISC chip, and the architectural
ancestor of the already-migrated `armv7-backend` lane (ARMv7-A,
2 decades newer).  Has no IR knowledge — its job is to turn a
register/condition/immediate tuple into a 32-bit instruction word
that matches the ISA bit-for-bit.

Mirror of `mips-r2000-encoder` / `armv7-encoder` / `intel8008-encoder`
/ `ge225-encoder`.

## Public surface

### Re-exported helpers (from `arm1-simulator`)

`arm1-simulator` is the canonical in-tree source of truth for the
ARM1 bit layout — it owns the condition/opcode field packing logic
in `encode_data_processing`, plus the specialised wrappers built on
top of it.  `arm1-encoder` re-exports the subset that
`arm1-backend` actually uses today:

| Item | Kind | Purpose |
|------|------|---------|
| `encode_mov_imm(cond, rd, imm8)` | fn | `MOV Rd, #imm8` (unrotated 8-bit immediate) |
| `encode_halt()` | fn | the pseudo-halt `SWI #0x123456`, `AL`-conditioned |
| `COND_AL` | const (`0xE`) | "always execute" condition code |

The full `encode_*` surface (data processing, load/store, block
transfer, branch — see `arm1-simulator`'s own module docs) lives in
`arm1_simulator` for the simulator's own test suite; this crate
re-exports only what the minimal-viable backend needs today.  A
future increment can widen the re-export list alongside
`arm1-backend`'s op coverage (e.g. `encode_alu_reg` for
arithmetic, `encode_branch` for control flow).

### Register-role constant

| Constant | Value | Role |
|----------|-------|------|
| `R0` | `0` | return-value register — `const_*` writes here; the caller reads it back via `ARM1::read_register(0)` after the pseudo-halt stops execution |

ARM1/ARMv1 predates the AAPCS calling-convention documents, but `R0`
is the register every hand-written example in
`code/specs/07e-arm1-simulator.md` (and `arm1-simulator`'s own
`test_mov_imm_and_halt`) uses to carry a computed value — the same
role AAPCS32 later formalised for ARMv7, and the same role
`armv7-backend`'s `r0` / `mips-r2000-backend`'s `$v0` play in their
respective lanes.

### Canonical word constant

| Constant | Value | Meaning |
|----------|-------|---------|
| `HALT_WORD` | `0xEF12_3456` | `SWI #0x123456`, `AL`-conditioned — the pseudo-halt `arm1-simulator::ARM1::execute_swi` intercepts to stop the fetch-decode-execute loop |

`encode_halt()` takes no arguments, so its result is fixed
regardless of caller context — `HALT_WORD` is a plain constant
rather than requiring a function call to derive, mirroring
`mips_r2000_encoder::RET_WORD`.

## Why `SWI`, not `BX LR`?

`armv7-backend` returns via `BX LR`, the link-register-return
convention every modern ARM ABI uses.  ARM1/ARMv1 predates that
convention entirely: there is no `BX` instruction, and the era's
subroutine-return idiom (`MOVS PC, R14`) needs a live `R14` set by a
preceding `BL`, which the minimal-viable `const_*`/`ret_*` scope
never establishes — there is no caller in a trivial ROM.

`arm1-simulator` already defines a pseudo-halt for exactly this
situation: `SWI #0x123456`.  Its `execute_swi` method special-cases
the 24-bit SWI comment field — when it equals `HALT_SWI`
(`0x123456`), the simulator sets its internal `halted` flag
(observable via `ARM1::halted()`) and stops, instead of entering
Supervisor mode as a genuine SWI would.  This is a simulator-level
convention (parallel to the Intel 8008 backend's `HLT` byte or the
GE-225 backend's `HLT` word), not real ARM1 silicon behaviour — real
ARM1 silicon has no way to "halt"; a real program would branch to
itself or await an interrupt.  Since `ARM1::halted()` is the only
externally observable "the program is done" signal, lowering
`ret_*`/`ret_void` to this pseudo-halt is the semantically correct
choice for the minimal-viable backend.

## `arm1-simulator`: behavioral Rust port, distinct from the gate-level sim

`code/specs/07e-arm1-simulator.md` documents the **behavioral**
ARM1 simulator this crate wraps — it executes ARMv1 machine code
directly using host-language arithmetic (no gate-level modelling).
`code/specs/07e2-arm1-gatelevel.md` documents a **separate**,
gate-level ARM1 simulation (routes every operation through actual
logic-gate primitives) — that spec is unrelated to this lane; do not
confuse the two when reading `code/packages/rust/arm1-simulator/`
(behavioral) vs. `code/packages/rust/arm1-gatelevel/` (gate-level).
`07e-arm1-simulator.md` predates this crate's existence and
describes the behavioral simulator's design at the spec level (its
`Public API` section is written in a Python-flavoured pseudo-class
form since the spec is shared across the project's per-language
simulator ports); `code/packages/rust/arm1-simulator/src/lib.rs` is
the concrete, already-complete Rust port this encoder depends on.

## Why this crate exists

Mirrors the encoder/backend split every other historical-arch lane
uses (see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
`arm1-backend` (the `Backend` trait implementation over CIR) depends
on this crate rather than reaching directly into `arm1_simulator`'s
decode/execute machinery, so the backend can evolve independently of
simulator internals and the simulator crate stays a leaf dependency
for exactly one direct consumer (this encoder).

Re-exporting (rather than duplicating) the encode functions keeps
`arm1_simulator` the single source of truth.  Any future fix to a
condition/opcode field layout lands in one place and propagates
automatically.

## Tests (7 unit tests + 1 doctest)

* `HALT_WORD == encode_halt()` and `HALT_WORD == 0xEF12_3456`.
* `HALT_WORD.to_le_bytes() == [0x56, 0x34, 0x12, 0xEF]` — the
  little-endian tail the `lang-aot` ARM1 e2e smoke test pins.
* `R0 == 0`.
* `COND_AL == 0xE`.
* `encode_mov_imm(COND_AL, R0, 42) == 0xE3A0_002A` — first
  instruction of the Twig `42` lowering.
* Little-endian byte layout of the canonical `const 42` word.

Plus a doctest walking through the canonical `const 42; ret` word
derivation.

## Out of scope

* R-type ALU/register-shift encoders, load/store, block-transfer,
  and branch encoders — these exist in `arm1_simulator` for the
  simulator's own test suite but are not yet re-exported here, since
  `arm1-backend` v0.1.0 does not use them.  A future backend
  increment (real register allocator, arithmetic ops, control flow)
  re-exports the additional helpers it needs.
* Disassembly or simulation — `arm1-simulator` handles both.
* Symbol resolution, linker relocations — that's `aot_core::link`
  territory.
