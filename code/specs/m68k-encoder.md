# `m68k-encoder` spec

> **Status:** v0.1.0 — eighth lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

Pure-Rust encoder for the Motorola 68000 (1979) instruction set — the
landmark 16/32-bit processor that powered the original Apple Macintosh
(1984), Commodore Amiga (1985), Atari ST (1985), early Sun-1/Sun-2
workstations, and the Sega Genesis/Mega Drive (1988). Has no IR
knowledge — its job is to turn a register/immediate tuple into raw
big-endian bytes that match the ISA bit-for-bit.

Mirror of `mos6502-encoder` / `arm1-encoder` / `armv7-encoder` /
`intel8008-encoder`.

## Public surface

### Re-exported helpers (from `m68k-simulator`)

`m68k-simulator` is the canonical in-tree source of truth for the
68000's opword-field packing logic (`encoding.rs`, which its own tests
also exercise). `m68k-encoder` re-exports the subset that
`m68k-backend` actually uses today:

| Item | Kind | Purpose |
|------|------|---------|
| `encode_move_l_imm_to_dn(dn, imm)` | fn | `MOVE.L #imm, Dn` — 32-bit immediate into a data register |
| `encode_trap15()` | fn | the HALT sentinel `TRAP #15` |
| `encode_moveq(dn, imm8)` | fn | `MOVEQ #imm8, Dn` — 8-bit sign-extended immediate |
| `encode_nop()` / `encode_rts()` | fn | `NOP` / `RTS`, exercised by this crate's own tests |
| `assemble(&[Vec<u8>])` | fn | concatenate per-instruction byte vectors (no endianness conversion needed — every helper already returns big-endian bytes) |

The full `encode_*` surface `m68k-simulator` could in principle expose
(one per ported mnemonic) is **not** re-exported here — only the
handful `m68k-backend`'s minimal-viable scope and this crate's tests
need. A future increment can widen the re-export list alongside
`m68k-backend`'s op coverage.

### Register-role constant

| Constant | Value | Role |
|----------|-------|------|
| `D0` | `0` | return-value/scratch register — `const_*` writes here; the caller reads it back via `sim.d[0]` after `TRAP #15` stops execution |

`D0`/`D1` (and `A0`/`A1`) are the 68000's own documented
scratch/return-value convention — see
`code/packages/python/motorola-68000-simulator/src/
motorola_68000_simulator/simulator.py`'s module doc: *"D0-D1, A0-A1 —
scratch / return values"*. The same role `arm1-encoder`'s `R0` and
`mips_r2000_encoder`'s `V0` play in their respective lanes.

### Canonical byte constant

| Constant | Value | Meaning |
|----------|-------|---------|
| `HALT_BYTES` | `[0x4E, 0x4F]` | `TRAP #15` — the HALT sentinel `m68k-simulator::execute`'s line-4 dispatcher intercepts to stop the fetch-decode-execute loop |

Big-endian, matching `encode_trap15()`'s own byte order — unlike
`arm1_encoder::HALT_WORD` (which stores ARM1's little-endian words),
there is no endianness flip between this constant and the encoder's
output.

## Why `TRAP #15`, not `STOP #imm`?

The 68000 has two genuine, silicon-real halting instructions, and the
pre-existing Python simulator's own `state.py` documents both: *"halted:
True after STOP or TRAP #15 executes."* `STOP #imm` is architecturally
the more literal "halt" — a privileged instruction that loads an
immediate into the status register and stops the CPU until an interrupt
occurs. `TRAP #15` is architecturally a software-interrupt/trap-vector
call (trap number 15 of 16); the Python simulator special-cases it as a
halt rather than modelling trap-vector dispatch.

Both are equally "real" per `state.py`'s own docs, so this lane follows
this repo's own established rule for such ties (see `mos6502-encoder`'s
and `arm1-encoder`'s own crate docs for the same reasoning applied to
their ISAs): **mirror whatever the pre-existing reference already
does, don't invent a fresh convention.** Inspecting the Python
original's own test suite settles it —
`code/packages/python/motorola-68000-simulator/tests/
test_instructions.py` defines a `_stop()` helper (*"TRAP #15 — halts
simulation without modifying SR"*) that is `_w(0x4E4F)`, i.e.
`TRAP #15`, and every one of that file's 100+ test programs (plus
`test_programs.py`'s 18) ends its program with that helper. `STOP #imm`
appears exactly once, in the module-level doctest example, and nowhere
else — a curiosity, not the established idiom. `TRAP #15` is therefore
the dominant, already-established halt convention this port mirrors.

`STOP #imm` is still ported faithfully in `m68k-simulator::execute`
(any program that happens to use it directly still halts correctly) —
it just isn't the convention `m68k-backend` emits.

## `m68k-simulator`: from-scratch Rust port, distinct from the gate-level sim

`code/specs/07n-motorola-68000-simulator.md` documents the
**behavioral** 68000 simulator this crate wraps — it executes 68000
machine code directly using host-language arithmetic (no gate-level
modelling). `code/specs/07n2-motorola68k-gatelevel.md` documents a
**separate**, gate-level 68000 simulation (routes every operation
through actual logic-gate primitives) — that spec is unrelated to this
lane; do not confuse the two when reading
`code/packages/rust/m68k-simulator/` (behavioral, this lane) vs.
`code/packages/rust/motorola68k-gatelevel/` (gate-level, pre-existing,
unrelated).

Unlike ARM1 (whose behavioral simulator, `arm1-simulator`, pre-existed
complete in-tree before its lane started) or MOS 6502 (whose Python
reference this repo already had, but whose Rust simulator did not),
this lane needed a brand-new Rust simulator alongside the encoder/
backend pair — `m68k-simulator`, a substantial-but-bounded port of the
pre-existing Python simulator (`code/packages/python/
motorola-68000-simulator`) covering a genuinely useful subset of the
ISA rather than a narrow stub. See `code/specs/m68k-backend.md` and
`m68k-simulator`'s own README for exactly which instructions/addressing
modes are ported vs. deferred.

## Why this crate exists

Mirrors the encoder/backend split every other historical-arch lane uses
(see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
`m68k-backend` (the `Backend` trait implementation over CIR) depends on
this crate rather than reaching directly into `m68k_simulator`'s
decode/execute machinery, so the backend can evolve independently of
simulator internals and the simulator crate stays a leaf dependency for
exactly one direct consumer (this encoder).

Re-exporting (rather than duplicating) the encode functions keeps
`m68k_simulator` the single source of truth. Any future fix to an
opword-field layout lands in one place and propagates automatically.

## Tests (5 unit tests + 1 doctest)

* `HALT_BYTES.to_vec() == encode_trap15()` and `HALT_BYTES == [0x4E, 0x4F]`.
* `D0 == 0`.
* `encode_move_l_imm_to_dn(D0, 42) == [0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A]`
  — first instruction of the IIR `42` lowering.
* `assemble([encode_move_l_imm_to_dn(D0, 42), encode_trap15()])` matches
  the full canonical byte sequence.

Plus a doctest walking through the canonical `const 42; ret` byte
derivation.

## Out of scope

* Encoders for every other `m68k-simulator`-ported mnemonic (`ADD`,
  `SUB`, branches, etc.) — these exist as raw bit-field arithmetic
  inline in `m68k-simulator::execute` (there's no flat opcode table to
  wrap the way `mos6502_simulator::opcodes::lookup` is), but are not
  yet given dedicated `encode_*` helpers here, since `m68k-backend`
  v0.1.0 does not use them. A future backend increment (real register
  allocator, arithmetic ops, control flow) adds the encoders it needs.
* Disassembly or simulation — `m68k-simulator` handles both.
* Symbol resolution, linker relocations — that's `aot_core::link`
  territory.
