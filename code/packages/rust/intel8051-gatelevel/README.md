# intel8051-gatelevel

Gate-level Intel 8051 (1980) simulator in Rust. Every arithmetic and
logical operation routes through AND, OR, XOR, NOT gates and a ripple-carry
adder — no native integer arithmetic in the data path.

## Architecture

The Intel 8051 was introduced in 1980 as Intel's first member of its MCS-51
microcontroller family. Fabricated in HMOS-II at roughly 3.5-micron feature
size, it contains approximately 68,000 transistors in a 40-pin DIP package.
The 8051 became one of the most widely used microcontrollers in history;
its ISA is still produced today in dozens of variants.

Unlike general-purpose CPUs, the 8051 is a **microcontroller**: it integrates
CPU, RAM, ROM, timers, I/O ports, and a serial port on a single chip, designed
for embedded control applications.

### Harvard memory model

The 8051 uses a **Harvard architecture** — program and data occupy completely
separate address spaces, accessed by different instructions:

| Space   | Size  | Access |
|---------|-------|--------|
| Code    | 64 KB | MOVC, implicit fetch |
| IRAM    | 256 B | MOV (direct/Rn/@Ri), PUSH/POP |
| XDATA   | 64 KB | MOVX |

### Internal RAM (IRAM) layout

```text
0x00-0x1F  Register banks 0-3  (R0-R7 per bank, selected by PSW.RS1:RS0)
0x20-0x2F  Bit-addressable area (128 individually addressable bits)
0x30-0x7F  General-purpose scratchpad RAM
0x80-0xFF  Special Function Registers (SFRs)
```

### Key SFRs

| SFR  | Addr  | Purpose |
|------|-------|---------|
| P0   | 0x80  | Port 0 latch (init 0xFF) |
| SP   | 0x81  | Stack Pointer (init 0x07) |
| DPL  | 0x82  | Data Pointer low byte |
| DPH  | 0x83  | Data Pointer high byte |
| P1   | 0x90  | Port 1 latch |
| P2   | 0xA0  | Port 2 latch |
| P3   | 0xB0  | Port 3 latch |
| PSW  | 0xD0  | Program Status Word |
| ACC  | 0xE0  | Accumulator |
| B    | 0xF0  | B register (MUL/DIV helper) |

### Program Status Word (PSW)

```text
Bit 7  CY  — Carry flag (also used as borrow for SUBB)
Bit 6  AC  — Auxiliary carry (nibble carry; used by DA A)
Bit 5  F0  — User-defined flag
Bit 4  RS1 — Register bank select bit 1
Bit 3  RS0 — Register bank select bit 0
Bit 2  OV  — Overflow flag (signed overflow for ADD/SUBB)
Bit 1  —   — (reserved, always 0)
Bit 0  P   — Parity: 1 when ACC has an ODD number of 1-bits
```

P is **odd parity** — the opposite sense from Intel 8086's PF (even parity).
P is recomputed after every instruction that modifies ACC.

### Bit-addressable space

The 8051 can read, set, clear, or test individual bits in:
- IRAM bytes 0x20-0x2F (bit addresses 0x00-0x7F)
- Certain SFRs (bit addresses 0x80-0xFF, aligned to SFR byte × 8)

```text
Bit addr 0x00-0x7F  →  byte = 0x20 + (bit_addr >> 3),  bit = bit_addr & 7
Bit addr 0x80-0xFF  →  byte = bit_addr & 0xF8,          bit = bit_addr & 7
```

Example: bit address 0xD7 is PSW.CY (byte 0xD0, bit 7).

### Gate-level data path

```text
bits.rs
  int_to_bits8/16 → LSB-first bit vectors
  add_8bit_full   → 8-stage ripple-carry adder → (result, carries[8])
  add_16bit_full  → 16-stage ripple-carry adder → (result, carry_out)
  invert_8bit     → 8 NOT gates in parallel
  compute_parity  → 7-gate XOR tree (ODD parity for PSW.P)
  compute_zero    → NOR tree

alu.rs
  AluResult8051 { result, cy, ac, ov, parity }
  add8(a, b, cy_in)     — carries[7]=CY, carries[3]=AC, XOR(c6,c7)=OV
  subb8(a, b, borrow)   — A + NOT(B) + NOT(borrow); CY=NOT(carry_out)
  anl8/orl8/xrl8(a, b)  — 8 AND/OR/XOR gates; cy=ac=ov=0
  inc8/dec8(a)           — gate-level ±1; cy=ac=ov=0 (never set by INC/DEC)
  rl8/rr8(a)             — circular rotate without carry; CY=exiting bit
  rlc8/rrc8(a, cy)       — 9-bit rotate through carry
  swap8(a)               — wire swap of nibbles; no flags at all
  da8(a, cy, ac)         — BCD adjust; nibble comparators + conditional adds
  mul8(a, b)             — shift-and-add loop (8 iterations)
  div8(a, b)             — repeated-subtraction loop

registers.rs
  RegisterFile8051: iram[256] + pc: u16
  read/write_iram8, read/write_pc, increment_pc (via gate-level adder)
  resolve_bit_addr, read_bit, write_bit

cpu.rs
  Cpu8051: rf + code[64KB] + xdata[64KB] + halted
  Harvard fetch, direct/indirect/bit addressing
  Full instruction dispatch: ~100 opcodes
  HALT sentinel: opcode 0xA5 (undefined on real 8051)
```

### SUBB model

SUBB (subtract with borrow) is implemented as:
```text
A − B − borrow = A + NOT(B) + NOT(borrow)
```

- `CY = NOT(carry_out)` — CY=1 means a borrow occurred (A < B + borrow)
- `AC = NOT(nibble_carry)` — AC=1 means lower nibble borrowed from upper
- `OV = XOR(carry[6], carry[7])` — signed overflow

### INC / DEC flag rule

INC and DEC **never** modify CY, AC, or OV.  They only change the register
value and (for INC/DEC A) update PSW.P.  This is a deliberate 8051 design
choice: INC and DEC are typically used as loop counters, not arithmetic, and
must not disturb the carry from a previous addition.

### MUL / DIV

MUL AB multiplies A × B using a shift-and-add loop over 8 iterations,
placing the low byte in A and the high byte in B.  OV=1 if result > 255.

DIV AB divides A by B using repeated subtraction, placing the quotient in A
and the remainder in B.  OV=1 for divide-by-zero; CY=0 always.

## Usage

```rust
use coding_adventures_intel8051_gatelevel::cpu::Cpu8051;

let mut cpu = Cpu8051::new();
// MOV A, #10; MOV R0, #5; ADD A, R0; HALT
cpu.execute(&[0x74, 10, 0x78, 5, 0x28, 0xA5], 0, 100);
assert_eq!(cpu.rf.read_iram8(0xE0), 15); // ACC = 15
assert!(cpu.halted);

// Loop: count from 3 down to 0 via DJNZ, incrementing B each iteration
// MOV R0, #3; MOV B, #0; loop: INC B; DJNZ R0, loop(-3); HALT
cpu.execute(&[0x78, 3, 0x75, 0xF0, 0, 0x05, 0xF0, 0xD8, 0xFD, 0xA5], 0, 100);
assert_eq!(cpu.rf.read_iram8(0xF0), 3); // B = 3
```

## Covered instructions

| Class | Opcodes | Notes |
|-------|---------|-------|
| MOV   | 0x74-0xFF range | A/Rn/dir/@Ri/imm, all addressing modes |
| MOVC  | 0x83, 0x93 | Code table lookup: @A+PC, @A+DPTR |
| MOVX  | 0xE0-0xF3 range | External data read/write |
| ADD   | 0x24-0x2F | A + B/Rn/dir/@Ri/imm |
| ADDC  | 0x34-0x3F | A + B + CY |
| SUBB  | 0x94-0x9F | A − B − CY (borrow) |
| INC   | 0x04-0x0F, 0xA3 | A/Rn/dir/@Ri, INC DPTR |
| DEC   | 0x14-0x1F | A/Rn/dir/@Ri |
| MUL   | 0xA4 | AB ← A × B (16-bit result) |
| DIV   | 0x84 | A ← quotient, B ← remainder |
| DA    | 0xD4 | BCD decimal adjust |
| ANL   | 0x52-0x5F | Logical AND, A and dir targets |
| ORL   | 0x42-0x4F | Logical OR |
| XRL   | 0x62-0x6F | Logical XOR |
| CLR   | 0xC3, 0xC4, 0xE4 | CLR C / CLR A |
| CPL   | 0xB3, 0xB2, 0xF4 | CPL C / CPL bit / CPL A |
| RL/RR | 0x23, 0x03 | Rotate without carry |
| RLC/RRC | 0x33, 0x13 | Rotate through carry |
| SWAP  | 0xC4 | Swap nibbles |
| XCH   | 0xC5-0xCF | Exchange A with Rn/dir/@Ri |
| XCHD  | 0xD6-0xD7 | Exchange lower nibble |
| PUSH/POP | 0xC0, 0xD0 | Stack via SP |
| BIT   | 0x72-0xD3 range | SETB, CLR, CPL, ANL/ORL C,bit |
| MOV bit | 0x92, 0xA2 | Move C ↔ bit |
| JB/JNB/JBC | 0x10, 0x20, 0x30 | Bit conditional jump |
| LJMP  | 0x02 | 16-bit unconditional jump |
| SJMP  | 0x80 | Relative 8-bit jump |
| AJMP  | xyzx0001 | 11-bit page jump |
| JMP   | 0x73 | @A+DPTR indirect jump |
| JZ/JNZ | 0x60, 0x70 | Jump if ACC zero/non-zero |
| JC/JNC | 0x40, 0x50 | Jump if CY set/clear |
| CJNE  | 0xB4-0xBF | Compare and jump if not equal |
| DJNZ  | 0xD5, 0xD8-0xDF | Decrement and jump if not zero |
| LCALL | 0x12 | 16-bit subroutine call |
| ACALL | xyzx0001 | 11-bit page call |
| RET   | 0x22 | Return from subroutine |
| RETI  | 0x32 | Return from interrupt |
| NOP   | 0x00 | No operation |

## Limitations

- **Timers/interrupts**: not simulated; RETI is identical to RET.
- **Serial port**: not simulated.
- **Port I/O**: port latches (P0-P3 at SFR 0x80/0x90/0xA0/0xB0) are
  readable and writable as SFRs, but no physical pin simulation.
- **Indirect addressing into SFR space**: `@Ri` with addr ≥ 0x80 is
  undefined on base 8051 — the simulator silently reads/writes IRAM at
  that address (no bounds check or exception).
- **AJMP/ACALL page boundary**: the PC high-5-bits source is the PC
  after the opcode fetch (before the operand fetch), consistent with
  real 8051 behaviour.

## How it fits in the stack

This crate is part of the **coding-adventures** gate-level CPU simulator
series (spec layer 07p2). Sibling crates implement the Intel 4004 (06i2),
Intel 8008 (07a2), Intel 8080 (07j2), MOS 6502 (07i2), Zilog Z80 (07h2),
Intel 8086 (07m2), and Motorola 68000 (07n2) at the same level of gate
fidelity.
