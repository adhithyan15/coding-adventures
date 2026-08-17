# MOS 6502 Simulator (Rust)

Behavioral simulator for the MOS Technology 6502 (1975) — designed by
Chuck Peddle's team after leaving Motorola, sold for $25 (versus the Intel
8080's $179), and the CPU behind the Apple II, Commodore 64, Atari
2600/8-bit line, BBC Micro, and — via the Ricoh 2A03 variant — the
NES/Famicom. Rust port of `code/packages/python/mos6502-simulator` (Layer
07j); see
[`code/specs/07j-mos6502-simulator.md`](../../../specs/07j-mos6502-simulator.md)
for the full ISA writeup.

## Supported instructions

All 151 official NMOS 6502 opcodes across all 13 addressing modes:

- **Load/store**: LDA, LDX, LDY, STA, STX, STY
- **Register transfers**: TAX, TAY, TXA, TYA, TSX, TXS
- **Stack**: PHA, PLA, PHP, PLP
- **Arithmetic**: ADC, SBC (binary and BCD/decimal mode)
- **Increment/decrement**: INC, INX, INY, DEC, DEX, DEY
- **Logical**: AND, ORA, EOR, BIT
- **Shift/rotate**: ASL, LSR, ROL, ROR (accumulator and memory forms)
- **Compare**: CMP, CPX, CPY
- **Branches**: BCC, BCS, BEQ, BNE, BPL, BMI, BVC, BVS
- **Jumps/calls**: JMP (absolute and indirect, with the page-wrap bug), JSR, RTS, RTI
- **Flags**: CLC, SEC, CLD, SED, CLI, SEI, CLV
- **Halt**: BRK (the pre-existing convention from the Python original — see below)
- **Other**: NOP

## Architecture

```
opcodes.rs   -- the 151-opcode table (mnemonic, addressing mode) + BRK_OPCODE
decode.rs    -- fetch + addressing-mode resolution (combined -- see its
                module doc for why the 6502's variable-length encoding
                makes this inseparable, unlike fixed-width MIPS/ARM1)
flags.rs     -- N/Z/V computation, P-byte pack/unpack, BCD add/sub
execute.rs   -- instruction executor (methods over &mut Mos6502Simulator)
simulator.rs -- top-level Mos6502Simulator with fetch-decode-execute
encoding.rs  -- encode_* helpers (subset used by tests / mos6502-encoder)
```

## What differs from fixed-width ISAs in this repo (MIPS R2000, ARM1, RV32I)

- **Variable-length instructions (1-3 bytes).**  Every other Rust ISA
  simulator in this repo is fixed-width (32-bit words).  The 6502's
  addressing mode determines instruction length, so `decode.rs` combines
  fetch and address-resolution into one step instead of keeping them
  separate the way `mips-r2000-simulator::decode` can.
- **A small, irregular register file.**  `A`/`X`/`Y` (8-bit), `S` (8-bit
  stack pointer), `PC` (16-bit) — no uniform GPR bank, so
  `Mos6502Simulator` exposes plain typed fields instead of reusing
  `cpu_simulator::RegisterFile`.
- **BCD decimal-mode arithmetic.**  `ADC`/`SBC` with the `D` flag set
  perform BCD correction.  Ported faithfully from the Python original,
  including the NMOS gotcha where `N`/`V`/`Z` reflect the *binary* result
  computed before BCD correction (the 65C02 fixes this; this simulator,
  like the Python original, models NMOS).
- **The indirect `JMP` page-wrap bug.**  `JMP ($10FF)` reads its high byte
  from `$1000`, not `$1100` — replicated exactly, as documented NMOS
  silicon behaviour.
- **`BRK` is the HALT sentinel.**  This is the Python original's
  *pre-existing, documented* convention (`simulator.py`'s module doc:
  *"BRK (opcode 0x00) is treated as HALT ... matches the convention used
  throughout the simulator stack (HLT for 8080, TRAP for IBM 704,
  etc.)"*), not a new choice made for this Rust port or for
  `mos6502-backend`.  `mos6502-encoder`/`mos6502-backend` mirror it
  rather than inventing a KIL/JAM-opcode or self-jump-loop convention.

## Usage

```rust
use mos6502_simulator::Mos6502Simulator;
use mos6502_simulator::encoding::*;

let mut sim = Mos6502Simulator::new(65536);
sim.run(&assemble(&[
    encode_lda_imm(42), // LDA #42
    encode_brk(),        // BRK (halt)
]));
assert_eq!(sim.a, 42);
assert!(sim.halted);
```
