# Motorola 68000 Simulator (Rust)

Behavioral simulator for the Motorola 68000 (1979) — the landmark 16/32-bit
processor that powered the original Apple Macintosh (1984), Commodore Amiga
(1985), Atari ST (1985), early Sun-1/Sun-2 workstations, and the Sega
Genesis/Mega Drive (1988). Rust port of
`code/packages/python/motorola-68000-simulator` (Layer 07n); see
[`code/specs/07n-motorola-68000-simulator.md`](../../../specs/07n-motorola-68000-simulator.md)
for the full ISA writeup.

Eighth lane of the [9-architecture expansion](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## Supported instructions

Bit-field-decoded, not a flat opcode table (see "Architecture" below for
why) — organised by the Python original's own "line" grouping:

- **Move**: `MOVE.B/W/L`, `MOVEA.W/L`, `MOVEQ`
- **Arithmetic**: `ADD`/`ADDA`/`ADDQ`, `SUB`/`SUBA`/`SUBQ` (`ADDX`/`SUBX`
  deferred)
- **Logic**: `AND`, `OR`, `EOR` (register and memory forms), `NOT`, `CLR`,
  `NEG`, `TST`
- **Compare**: `CMP`, `CMPA`
- **Register ops**: `EXG`, `SWAP`, `EXT.W`, `EXT.L`
- **Shift/rotate**: `ASL`/`ASR`, `LSL`/`LSR`, `ROXL`/`ROXR`, `ROL`/`ROR`
  (register form only — see below)
- **Branches**: `BRA`, `BSR`, all 14 conditional `Bcc`, `DBcc`, `Scc`
- **Calls/jumps**: `JSR`, `JMP`, `RTS`, `RTR`
- **Stack frame**: `LINK`, `UNLK`
- **Addressing**: `LEA`
- **Halt/misc**: `TRAP #n` (the halt convention — see below), `STOP #imm`
  (real 68000 semantics, faithfully ported, but not this port's chosen
  convention), `NOP`, `RESET` (stub)

**Deferred** (return a descriptive `Err`, never silently mis-execute):
`ORI`/`ANDI`/`SUBI`/`ADDI`/`EORI`/`CMPI` (the whole line-0 immediate
group), `BTST`/`BCHG`/`BCLR`/`BSET`, `DIVU`/`DIVS`, `MULU`/`MULS`,
`ADDX`/`SUBX`, `NEGX`, `PEA`, `MOVE SR`/`MOVE CCR`, memory-operand
shift/rotate.

## Addressing modes: 8 of 11 ported

| Mode | Syntax | Ported? |
|------|--------|---------|
| Data/address register direct | `Dn` / `An` | ✅ |
| Indirect | `(An)` | ✅ |
| Postincrement | `(An)+` | ✅ |
| Predecrement | `-(An)` | ✅ |
| 16-bit displacement | `d16(An)` | ✅ |
| Indexed | `d8(An,Xn.sz)` | ❌ deferred |
| Absolute short | `(abs).W` | ✅ |
| Absolute long | `(abs).L` | ✅ |
| PC-relative | `d16(PC)` | ❌ deferred |
| PC-relative indexed | `d8(PC,Xn.sz)` | ❌ deferred |
| Immediate | `#imm` | ✅ |

The 3 deferred modes are exactly the ones that need a second extension
word carrying an index-register selector, a Dn-vs-An bit, and a
word/long size bit — the most intricate addressing-mode machinery on the
chip, and not needed by `m68k-backend`'s minimal-viable
`MOVE.L #imm, D0` scope. `decode::decode_ea` returns `Err` for them
rather than misinterpreting the extension-word stream.

## Architecture

```
opcodes.rs   -- shared size-code tables, condition-code predicates, the
                HALT sentinel, masks/sign-extension helpers
decode.rs    -- effective-address classification/resolution + the
                PC-relative fetch helpers every instruction shares
flags.rs     -- N/Z/V/C/X computation (direct port of flags.py)
execute.rs   -- one function per opword "line" (the top 4 bits),
                over &mut M68kSimulator
simulator.rs -- top-level M68kSimulator with fetch-decode-execute
encoding.rs  -- encode_* helpers (subset used by tests / m68k-encoder)
```

Unlike MOS 6502/Intel-8080-family CPUs (a flat opcode → mnemonic table)
or MIPS R2000/ARM1 (fixed 3-format decode), the 68000's 16-bit opword
groups instructions by their top 4 bits ("line 0" through "line F" —
even the Python original's own module doc calls this "a rough category,
not a complete opcode"), and each line further branches on its own bit
sub-fields. There is no single table to port; `decode.rs`/`execute.rs`
mirror the Python original's per-line dispatch methods directly.

## What differs from every other simulator in this repo

- **Big-endian**, unlike MIPS R2000/ARM1/RV32I/MOS 6502 (all
  little-endian or byte-oriented). `decode.rs`'s `mem_read`/`mem_write`
  and `fetch_word`/`fetch_long` all assemble bytes most-significant-byte
  first, matching the Python original exactly.
- **A real 24-bit address bus (16 MiB)** — every computed effective
  address is masked to `0x00FFFFFF`; the backing `Memory` a caller
  constructs may be smaller (tests routinely use a few KiB).
- **16 uniform 32-bit GPRs** in two orthogonal banks (`D0`-`D7`,
  `A0`-`A7`), unlike the 6502's three small, irregular, individually
  named registers — `M68kSimulator` exposes `pub d: [u32; 8]` /
  `pub a: [u32; 8]` arrays rather than 16 discrete named fields.
- **`TRAP #15` is the HALT sentinel**, not `STOP #imm` — see below.

## Halt convention: `TRAP #15`, not `STOP #imm`

The 68000 has two genuine, silicon-real halting instructions, and the
pre-existing Python simulator's own `state.py` documents both: *"halted:
True after STOP or TRAP #15 executes."* Both are equally "real" per
that doc — so this port follows this repo's own rule for such ties:
mirror whatever the pre-existing reference already *does*, don't invent
a new convention. Inspecting the Python original's own test suite
settles it: `test_instructions.py` defines a `_stop()` helper (*"TRAP
#15 — halts simulation without modifying SR"*) used across 100+ test
programs, while `STOP #imm` appears exactly once, in a module-level
doctest. `TRAP #15` is the dominant, already-established idiom this
port mirrors. `STOP #imm` is still ported faithfully (any program using
it directly still halts correctly) — it's just not the convention
`m68k-backend` emits.

## Usage

```rust
use m68k_simulator::M68kSimulator;
use m68k_simulator::encoding::{assemble, encode_move_l_imm_to_dn, encode_trap15};

let mut sim = M68kSimulator::new(65536);
sim.run(&assemble(&[
    encode_move_l_imm_to_dn(0, 42), // MOVE.L #42, D0
    encode_trap15(),                 // TRAP #15 (halt)
]));
assert_eq!(sim.d[0], 42);
assert!(sim.halted);
```
