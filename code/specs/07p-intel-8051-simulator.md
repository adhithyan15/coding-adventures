# Spec 07p — Intel 8051 Behavioral Simulator

## Overview

The **Intel 8051** (MCS-51, 1980) is an 8-bit microcontroller — Intel's first
single-chip computer — and one of the most successful and widely manufactured
silicon designs in history. Designed at Intel's Santa Clara facility, the 8051
combined a CPU, RAM, ROM, timers, serial port, and I/O ports on a single chip.
In doing so it defined the microcontroller as a product category.

Historical milestones powered by the 8051:

- **Intel MCS-51 family** (1980) — the original 8051, 8052, 8031, 8032 variants
  established the architecture and split ROM-internal vs ROM-external variants
- **Embedded systems everywhere** — industrial controllers, keyboards, modems,
  printers, medical devices, toys, automotive engine management units
- **Philips/NXP 80C51** — CMOS variant became a long-lived commodity part
  widely used in embedded systems through the 2010s
- **AT89S52** (Atmel, later Microchip) — a popular modern 8051-compatible
  part still sold today as a learning platform
- **Estimated production: > 20 billion units** — the 8051 ISA has been
  implemented by more companies and fabricated in more units than any other
  microprocessor architecture. It is genuinely the most-produced CPU design
  in history.

### Why the 8051 was revolutionary

Before the 8051, building an embedded controller required a CPU chip, a
separate RAM chip, a ROM chip, an I/O peripheral chip, and glue logic — a
full printed circuit board just for a simple controller. The 8051 put all of
this onto one package:

| Feature              | External solution (pre-8051) | 8051 (single chip)          |
|----------------------|------------------------------|-----------------------------|
| CPU core             | Separate chip (e.g., 8080)   | Integrated                  |
| Program memory       | Separate EPROM               | 4 KB internal ROM           |
| Data memory          | Separate SRAM                | 128 bytes internal RAM      |
| I/O ports            | Peripheral chip (e.g., 8255) | 32 bits over 4 ports (P0–P3)|
| Serial port (UART)   | Separate USART chip          | Integrated full-duplex UART |
| Timers               | Separate timer/counter chip  | 2 × 16-bit timers           |
| Interrupt controller | Separate chip (e.g., 8259)   | Integrated (6 sources)      |

This integration reduced BOM cost, board area, and power consumption by an
order of magnitude, enabling an entirely new class of embedded product.

The 8051 also introduced the **Harvard architecture** to the microcontroller
world — separate address spaces for program (code) memory and data memory,
allowing the CPU to fetch the next instruction and read data simultaneously.
This differs from the von Neumann (Princeton) architecture used by most
desktop processors, where code and data share one address space.

This spec defines Layer **07p** — a Python behavioral simulator for the 8051
following the SIM00 `Simulator[I8051State]` protocol.

---

## Architecture

### Memory spaces

The 8051 has **four distinct memory spaces**, each with independent address
ranges and access semantics. This multiplicity is the defining characteristic
of the 8051 and the most important thing to understand before studying its ISA.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     8051 Memory Map                                     │
│                                                                         │
│  Code Memory (64 KB, Harvard)    Data Memory (internal, 256 B)         │
│  ┌─────────────────────────┐     ┌─────────────────────┐               │
│  │  0x0000 – 0xFFFF        │     │  0x00 – 0x1F (32 B) │ Register banks│
│  │  (external ROM/EPROM)   │     │  0x20 – 0x2F (16 B) │ Bit-addressable│
│  │  0x0000 – 0x0FFF        │     │  0x30 – 0x7F (80 B) │ Scratch RAM   │
│  │  (internal ROM, 4 KB)   │     │  0x80 – 0xFF (128 B)│ SFRs          │
│  └─────────────────────────┘     └─────────────────────┘               │
│                                                                         │
│  External Data (XDATA, 64 KB)    Bit Space (128 bits)                  │
│  ┌─────────────────────────┐     ┌─────────────────────┐               │
│  │  0x0000 – 0xFFFF        │     │  bit 0x00 – 0x7F    │               │
│  │  accessed via MOVX      │     │  (internal RAM 0x20)│               │
│  └─────────────────────────┘     └─────────────────────┘               │
└─────────────────────────────────────────────────────────────────────────┘
```

#### Internal data memory (256 bytes)

The 128-byte lower half (0x00–0x7F) is directly accessible by all data-memory
instructions. The upper half (0x80–0xFF) is the **Special Function Register
(SFR)** space — each address in this range is a hardware register, not RAM.
Accessing 0x80–0xFF with a direct address hits the SFRs; accessing the same
numeric range with an indirect address (@R0 / @R1) hits a second, separate
upper RAM bank available on the 8052.

#### Register banks

The lower 32 bytes of internal RAM are divided into four 8-byte **register
banks** (bank 0–3). Each bank provides 8 working registers named R0–R7.
The active bank is selected by bits RS1:RS0 in the PSW. At reset, bank 0 is
active (R0=0x00, R1=0x01, …, R7=0x07 in RAM).

```
Address  Bank 0  Bank 1  Bank 2  Bank 3
0x00     R0      –       –       –
0x01     R1      –       –       –
...      ...
0x07     R7      –       –       –
0x08     –       R0      –       –
...      ...
0x0F     –       R7      –       –
0x10     –       –       R0      –
...      ...
0x17     –       –       R7      –
0x18     –       –       –       R0
...      ...
0x1F     –       –       –       R7
```

#### Bit-addressable area

Internal RAM bytes 0x20–0x2F provide 128 individually addressable bits,
numbered bit 0x00 (LSB of byte 0x20) through bit 0x7F (MSB of byte 0x2F).
The SETB, CLR, CPL, MOV C,bit, MOV bit,C, JB, JNB, JBC instructions all
operate on this 128-bit space plus the SFR bit addresses (0x80–0xFF).

---

### Registers

#### Accumulator (ACC / A)

The **accumulator** is the primary working register. It is the implicit
source and/or destination for most arithmetic and logical operations. It
occupies SFR address 0xE0 and is individually bit-addressable (bits ACC.0
through ACC.7 map to SFR bits 0xE0–0xE7).

#### B register

The **B register** (SFR 0xF0) is the secondary operand for MUL AB and DIV
AB. After MUL, the high byte of the 16-bit product is stored in B (low byte
in A). After DIV, the remainder is stored in B (quotient in A). It is
bit-addressable.

#### Data pointer (DPTR)

The **data pointer** is a 16-bit register formed by two 8-bit SFRs:
- DPH (SFR 0x83) — high byte
- DPL (SFR 0x82) — low byte

DPTR is used as a 16-bit pointer for accessing external data memory (MOVX
@DPTR) and code memory (MOVC A,@A+DPTR). It is the only 16-bit register on
the 8051.

#### Stack pointer (SP)

The **stack pointer** (SFR 0x81) is an 8-bit register pointing into
internal RAM. At reset, SP=0x07. The stack grows upward: PUSH increments SP
then writes; POP reads then decrements SP. Unlike x86, the stack lives in
internal RAM and is only 256 bytes deep at most. Practical stack depth is
limited by the need to not overwrite register banks and bit-addressable area.

#### Program counter (PC)

The **program counter** (not in SFR space) is a 16-bit register pointing
into code memory. At reset, PC=0x0000.

#### Program status word (PSW)

The **PSW** (SFR 0xD0) is an 8-bit status register. It is bit-addressable.

```
Bit  7    6    5    4    3    2    1    0
     CY   AC   F0   RS1  RS0  OV   –    P
```

| Bit  | Name | Description                                                    |
|------|------|----------------------------------------------------------------|
| PSW.7| CY   | Carry flag. Set by addition carry-out, subtraction borrow.    |
| PSW.6| AC   | Auxiliary carry. Carry from bit 3 to bit 4 (BCD arithmetic).  |
| PSW.5| F0   | User-defined flag. No hardware effect.                         |
| PSW.4| RS1  | Register bank select bit 1.                                    |
| PSW.3| RS0  | Register bank select bit 0.                                    |
| PSW.2| OV   | Overflow flag. Set by signed overflow or DIV by zero.         |
| PSW.1| –    | Reserved (always 0).                                           |
| PSW.0| P    | Parity flag. Even parity of ACC (hardware-computed).          |

Register bank selection:

| RS1 | RS0 | Active bank | RAM addresses |
|-----|-----|-------------|---------------|
|  0  |  0  | Bank 0      | 0x00–0x07     |
|  0  |  1  | Bank 1      | 0x08–0x0F     |
|  1  |  0  | Bank 2      | 0x10–0x17     |
|  1  |  1  | Bank 3      | 0x18–0x1F     |

---

### Special function registers (SFRs)

Addresses 0x80–0xFF in the direct-addressed data space are SFRs. The 8051
defines 21 SFRs; addresses not listed are reserved (reads return 0xFF on
real hardware). Our simulator implements the subset relevant to a behavioral
simulation:

| Address | Name | Reset | Description                          |
|---------|------|-------|--------------------------------------|
| 0x80    | P0   | 0xFF  | Port 0 latch                         |
| 0x81    | SP   | 0x07  | Stack pointer                        |
| 0x82    | DPL  | 0x00  | DPTR low byte                        |
| 0x83    | DPH  | 0x00  | DPTR high byte                       |
| 0x87    | PCON | 0x00  | Power control                        |
| 0x88    | TCON | 0x00  | Timer/counter control                |
| 0x89    | TMOD | 0x00  | Timer/counter mode                   |
| 0x8A    | TL0  | 0x00  | Timer 0 low byte                     |
| 0x8B    | TL1  | 0x00  | Timer 1 low byte                     |
| 0x8C    | TH0  | 0x00  | Timer 0 high byte                    |
| 0x8D    | TH1  | 0x00  | Timer 1 high byte                    |
| 0x90    | P1   | 0xFF  | Port 1 latch                         |
| 0x98    | SCON | 0x00  | Serial control                       |
| 0x99    | SBUF | 0x00  | Serial data buffer                   |
| 0xA0    | P2   | 0xFF  | Port 2 latch                         |
| 0xA8    | IE   | 0x00  | Interrupt enable                     |
| 0xB0    | P3   | 0xFF  | Port 3 latch                         |
| 0xB8    | IP   | 0x00  | Interrupt priority                   |
| 0xD0    | PSW  | 0x00  | Program status word                  |
| 0xE0    | ACC  | 0x00  | Accumulator                          |
| 0xF0    | B    | 0x00  | B register                           |

The simulator treats SFR reads and writes as direct accesses to the internal
SFR array (sfr[addr - 0x80]). Parity (PSW.P) is automatically recomputed
after every operation that changes ACC.

---

## Addressing modes

The 8051 supports 5 addressing modes. Each instruction specifies exactly
which modes it accepts; unlike the PDP-11 orthogonal ISA, modes are hard-
coded per opcode, not a field.

### 1. Register addressing

The operand is one of R0–R7 in the currently active register bank. The
register number is encoded in the 3 low bits of the opcode.

```
ADD A, R2    ; A = A + R2
```

### 2. Direct addressing (dir)

The operand is a byte at a directly specified 8-bit address. Addresses
0x00–0x7F access internal RAM; addresses 0x80–0xFF access SFRs.

```
MOV A, 0x30  ; A = iram[0x30]
MOV A, PSW   ; A = sfr[PSW]    (PSW = 0xD0)
```

### 3. Register-indirect addressing (@Ri)

The operand is a byte in internal RAM pointed to by R0 or R1 (only these
two are indirect pointers). For 8052 extended RAM (0x80–0xFF data), indirect
addressing 0x80–0xFF reaches the upper 128 bytes of RAM (not SFRs).

```
MOV A, @R0   ; A = iram[R0]
```

### 4. Immediate addressing (#data)

The operand is an 8-bit constant following the opcode byte in code memory.
For 16-bit loads (MOV DPTR, #data16), a 16-bit immediate follows.

```
MOV A, #42   ; A = 42
MOV DPTR, #0x8000  ; DPTR = 0x8000
```

### 5. Indexed addressing (@A+DPTR or @A+PC)

Used only by MOVC (move code byte). The effective address is A + DPTR or
A + PC (after the MOVC instruction's fetch). This provides read-only lookup
table access into code memory.

```
MOVC A, @A+DPTR  ; A = code_memory[A + DPTR]
```

For external data memory (MOVX):

```
MOVX A, @DPTR    ; A = xdata[DPTR]
MOVX @DPTR, A    ; xdata[DPTR] = A
MOVX A, @Ri      ; A = xdata[Ri]  (8-bit address, high byte from P2)
MOVX @Ri, A      ; xdata[Ri] = A
```

---

## Instruction set

### Encoding format

The 8051 uses a single-byte opcode. Most instructions are 1, 2, or 3 bytes:

- **1-byte**: opcode only (e.g., `NOP`, `CLR A`, `ADD A,R0`)
- **2-byte**: opcode + operand (e.g., `ADD A,#imm8`, `ADD A,dir`)
- **3-byte**: opcode + operand1 + operand2 (e.g., `MOV dir,dir`, `CJNE A,#imm8,rel`)

Branch instructions use **relative** (signed 8-bit) or **absolute** 11-bit /
16-bit offsets:

- **SJMP / Jcc**: 8-bit signed relative offset (–128 to +127 from next PC)
- **AJMP / ACALL**: 11-bit absolute address within current 2 KB page
- **LJMP / LCALL**: 16-bit absolute address
- **RET / RETI**: return from subroutine / interrupt

### Data transfer

| Mnemonic           | Encoding  | Bytes | Operation                              |
|--------------------|-----------|-------|----------------------------------------|
| MOV A, Rn          | 0xE8+n    | 1     | A ← Rn                                |
| MOV A, dir         | 0xE5      | 2     | A ← iram[dir] or sfr[dir]             |
| MOV A, @Ri         | 0xE6+i    | 1     | A ← iram[Ri]                          |
| MOV A, #imm        | 0x74      | 2     | A ← imm                               |
| MOV Rn, A          | 0xF8+n    | 1     | Rn ← A                                |
| MOV Rn, dir        | 0xA8+n    | 2     | Rn ← iram[dir]                        |
| MOV Rn, #imm       | 0x78+n    | 2     | Rn ← imm                              |
| MOV dir, A         | 0xF5      | 2     | iram[dir] ← A                         |
| MOV dir, Rn        | 0x88+n    | 2     | iram[dir] ← Rn                        |
| MOV dir, dir2      | 0x85      | 3     | iram[dir] ← iram[dir2]  (src,dst order)|
| MOV dir, @Ri       | 0x86+i    | 2     | iram[dir] ← iram[Ri]                  |
| MOV dir, #imm      | 0x75      | 3     | iram[dir] ← imm                       |
| MOV @Ri, A         | 0xF6+i    | 1     | iram[Ri] ← A                          |
| MOV @Ri, dir       | 0xA6+i    | 2     | iram[Ri] ← iram[dir]                  |
| MOV @Ri, #imm      | 0x76+i    | 2     | iram[Ri] ← imm                        |
| MOV DPTR, #imm16   | 0x90      | 3     | DPTR ← imm16                          |
| MOVC A, @A+DPTR    | 0x93      | 1     | A ← code[A + DPTR]                    |
| MOVC A, @A+PC      | 0x83      | 1     | A ← code[A + PC]  (PC = addr after)   |
| MOVX A, @Ri        | 0xE2+i    | 1     | A ← xdata[Ri]                         |
| MOVX A, @DPTR      | 0xE0      | 1     | A ← xdata[DPTR]                       |
| MOVX @Ri, A        | 0xF2+i    | 1     | xdata[Ri] ← A                         |
| MOVX @DPTR, A      | 0xF0      | 1     | xdata[DPTR] ← A                       |
| PUSH dir           | 0xC0      | 2     | SP++; iram[SP] ← iram[dir]            |
| POP dir            | 0xD0      | 2     | iram[dir] ← iram[SP]; SP--            |
| XCH A, Rn          | 0xC8+n    | 1     | A ↔ Rn                                |
| XCH A, dir         | 0xC5      | 2     | A ↔ iram[dir]                         |
| XCH A, @Ri         | 0xC6+i    | 1     | A ↔ iram[Ri]                          |
| XCHD A, @Ri        | 0xD6+i    | 1     | Low nibble of A ↔ low nibble of iram[Ri]|

**Note on MOV dir,dir2 operand order**: The encoding is `0x85 src dst`, so
the second byte is source and third byte is destination. This is unusual among
8-bit CPUs.

### Arithmetic

| Mnemonic           | Encoding  | Bytes | Operation                              |
|--------------------|-----------|-------|----------------------------------------|
| ADD A, Rn          | 0x28+n    | 1     | A ← A + Rn; set CY,AC,OV,P            |
| ADD A, dir         | 0x25      | 2     | A ← A + iram[dir]                     |
| ADD A, @Ri         | 0x26+i    | 1     | A ← A + iram[Ri]                      |
| ADD A, #imm        | 0x24      | 2     | A ← A + imm                           |
| ADDC A, Rn         | 0x38+n    | 1     | A ← A + Rn + CY; set CY,AC,OV,P      |
| ADDC A, dir        | 0x35      | 2     | A ← A + iram[dir] + CY               |
| ADDC A, @Ri        | 0x36+i    | 1     | A ← A + iram[Ri] + CY               |
| ADDC A, #imm       | 0x34      | 2     | A ← A + imm + CY                     |
| SUBB A, Rn         | 0x98+n    | 1     | A ← A – Rn – CY; set CY,AC,OV,P     |
| SUBB A, dir        | 0x95      | 2     | A ← A – iram[dir] – CY              |
| SUBB A, @Ri        | 0x96+i    | 1     | A ← A – iram[Ri] – CY              |
| SUBB A, #imm       | 0x94      | 2     | A ← A – imm – CY                    |
| INC A              | 0x04      | 1     | A ← A + 1 (no flags updated)         |
| INC Rn             | 0x08+n    | 1     | Rn ← Rn + 1 (no flags)              |
| INC dir            | 0x05      | 2     | iram[dir] ← iram[dir] + 1 (no flags)|
| INC @Ri            | 0x06+i    | 1     | iram[Ri] ← iram[Ri] + 1             |
| INC DPTR           | 0xA3      | 1     | DPTR ← DPTR + 1 (16-bit, no flags)  |
| DEC A              | 0x14      | 1     | A ← A – 1 (no flags)                |
| DEC Rn             | 0x18+n    | 1     | Rn ← Rn – 1 (no flags)             |
| DEC dir            | 0x15      | 2     | iram[dir] ← iram[dir] – 1           |
| DEC @Ri            | 0x16+i    | 1     | iram[Ri] ← iram[Ri] – 1            |
| MUL AB             | 0xA4      | 1     | B:A ← A × B (unsigned); CY=0; OV= B≠0|
| DIV AB             | 0x84      | 1     | A ← A ÷ B (quotient); B ← remainder  |
|                    |           |       | CY=0; OV=1 if B=0 (division by zero) |
| DA A               | 0xD4      | 1     | Decimal adjust A after BCD ADD        |

**INC/DEC flag behavior**: INC and DEC do **not** modify any PSW flags. This
is unlike most other architectures and a common source of bugs. To test the
result of INC/DEC, use `CJNE A, #val, label` or a separate `JZ`/`JNZ` via
`ANL A, A` or similar.

**DA A (Decimal Adjust)**: After ADD/ADDC of two BCD bytes, DA A corrects
the result to valid BCD:
- If low nibble > 9 or AC=1: add 0x06
- If high nibble > 9 or CY=1: add 0x60 and set CY

### Logic

| Mnemonic           | Encoding  | Bytes | Operation                              |
|--------------------|-----------|-------|----------------------------------------|
| ANL A, Rn          | 0x58+n    | 1     | A ← A & Rn; set P                     |
| ANL A, dir         | 0x55      | 2     | A ← A & iram[dir]                     |
| ANL A, @Ri         | 0x56+i    | 1     | A ← A & iram[Ri]                      |
| ANL A, #imm        | 0x54      | 2     | A ← A & imm                           |
| ANL dir, A         | 0x52      | 2     | iram[dir] ← iram[dir] & A             |
| ANL dir, #imm      | 0x53      | 3     | iram[dir] ← iram[dir] & imm           |
| ORL A, Rn          | 0x48+n    | 1     | A ← A | Rn                            |
| ORL A, dir         | 0x45      | 2     | A ← A | iram[dir]                     |
| ORL A, @Ri         | 0x46+i    | 1     | A ← A | iram[Ri]                     |
| ORL A, #imm        | 0x44      | 2     | A ← A | imm                           |
| ORL dir, A         | 0x42      | 2     | iram[dir] ← iram[dir] | A             |
| ORL dir, #imm      | 0x43      | 3     | iram[dir] ← iram[dir] | imm           |
| XRL A, Rn          | 0x68+n    | 1     | A ← A ^ Rn                            |
| XRL A, dir         | 0x65      | 2     | A ← A ^ iram[dir]                     |
| XRL A, @Ri         | 0x66+i    | 1     | A ← A ^ iram[Ri]                      |
| XRL A, #imm        | 0x64      | 2     | A ← A ^ imm                           |
| XRL dir, A         | 0x62      | 2     | iram[dir] ← iram[dir] ^ A             |
| XRL dir, #imm      | 0x63      | 3     | iram[dir] ← iram[dir] ^ imm           |
| CLR A              | 0xE4      | 1     | A ← 0; P ← 0                         |
| CPL A              | 0xF4      | 1     | A ← ~A; update P                     |
| RL A               | 0x23      | 1     | A ← (A << 1) | (A >> 7)  (rotate left, no CY)|
| RLC A              | 0x33      | 1     | {CY,A} ← {A,CY}  (rotate left through carry)|
| RR A               | 0x03      | 1     | A ← (A >> 1) | (A << 7)  (rotate right)|
| RRC A              | 0x13      | 1     | {A,CY} ← {CY,A}  (rotate right through carry)|
| SWAP A             | 0xC4      | 1     | A[7:4] ↔ A[3:0]  (swap nibbles)      |

### Bit manipulation

The 8051 is one of the few architectures with a dedicated bit-manipulation
instruction set. This is essential for I/O port control (setting individual
pins) and flag manipulation.

| Mnemonic           | Encoding  | Bytes | Operation                              |
|--------------------|-----------|-------|----------------------------------------|
| CLR C              | 0xC3      | 1     | CY ← 0                                |
| CLR bit            | 0xC2      | 2     | bit_addr ← 0                          |
| SETB C             | 0xD3      | 1     | CY ← 1                                |
| SETB bit           | 0xD2      | 2     | bit_addr ← 1                          |
| CPL C              | 0xB3      | 1     | CY ← ~CY                              |
| CPL bit            | 0xB2      | 2     | bit_addr ← ~bit_addr                  |
| ANL C, bit         | 0x82      | 2     | CY ← CY & bit_addr                   |
| ANL C, /bit        | 0xB0      | 2     | CY ← CY & ~bit_addr                  |
| ORL C, bit         | 0x72      | 2     | CY ← CY | bit_addr                   |
| ORL C, /bit        | 0xA0      | 2     | CY ← CY | ~bit_addr                  |
| MOV C, bit         | 0xA2      | 2     | CY ← bit_addr                        |
| MOV bit, C         | 0x92      | 2     | bit_addr ← CY                        |

**Bit address resolution**:
- Bit addresses 0x00–0x7F → RAM bytes 0x20–0x2F (bit N is byte 0x20+(N>>3), bit (N&7))
- Bit addresses 0x80–0xFF → SFR bits (only bit-addressable SFRs: ACC=0xE0,
  B=0xF0, PSW=0xD0, TCON=0x88, SCON=0x98, IE=0xA8, IP=0xB8, P0=0x80,
  P1=0x90, P2=0xA0, P3=0xB0)
  For these, bit N → SFR at (N & 0xF8), bit position (N & 0x07)

### Branching and control flow

#### Unconditional jumps

| Mnemonic           | Encoding        | Bytes | Operation                           |
|--------------------|-----------------|-------|-------------------------------------|
| LJMP addr16        | 0x02 hi lo      | 3     | PC ← addr16                        |
| AJMP addr11        | *(a10:a9:a8)*01 | 2     | PC[10:0] ← addr11; PC[15:11] unchanged|
| SJMP rel           | 0x80 rel        | 2     | PC ← PC + 2 + rel (signed 8-bit)   |
| JMP @A+DPTR        | 0x73            | 1     | PC ← A + DPTR                      |

**AJMP encoding**: The 5 high bits of the first byte are `a10:a9:a8:0:0:0:0:1`.
That is, opcode = `(addr11 >> 3) & 0xE0 | 0x01`. This gives page-relative
jumps within each 2 KB block: addr11 replaces the low 11 bits of PC; the
upper 5 bits remain unchanged (same 2 KB page).

#### Conditional jumps

All conditional jumps are 2 bytes: opcode + signed 8-bit offset. Target =
PC_after_fetch + rel = (PC_of_instruction + 2) + rel.

| Mnemonic           | Encoding  | Condition                              |
|--------------------|-----------|----------------------------------------|
| JZ rel             | 0x60      | Jump if A = 0                         |
| JNZ rel            | 0x70      | Jump if A ≠ 0                         |
| JC rel             | 0x40      | Jump if CY = 1                        |
| JNC rel            | 0x50      | Jump if CY = 0                        |
| JB bit, rel        | 0x20      | Jump if bit = 1  (3-byte)             |
| JNB bit, rel       | 0x30      | Jump if bit = 0  (3-byte)             |
| JBC bit, rel       | 0x10      | Jump if bit = 1, then clear bit (3-byte)|

#### Compare-and-branch (CJNE)

| Mnemonic              | Encoding  | Bytes | Operation                          |
|-----------------------|-----------|-------|------------------------------------|
| CJNE A, dir, rel      | 0xB5      | 3     | if A ≠ dir: branch; CY = A < dir  |
| CJNE A, #imm, rel     | 0xB4      | 3     | if A ≠ imm: branch; CY = A < imm  |
| CJNE Rn, #imm, rel    | 0xB8+n    | 3     | if Rn ≠ imm: branch; CY = Rn < imm|
| CJNE @Ri, #imm, rel   | 0xB6+i    | 3     | if [Ri] ≠ imm: branch             |

CJNE sets CY = 1 if the first operand is less than the second (unsigned
comparison); it does not modify any other flags.

#### Decrement-and-branch (DJNZ)

| Mnemonic           | Encoding  | Bytes | Operation                           |
|--------------------|-----------|-------|-------------------------------------|
| DJNZ Rn, rel       | 0xD8+n    | 2     | Rn--; if Rn ≠ 0: branch            |
| DJNZ dir, rel      | 0xD5      | 3     | iram[dir]--; if ≠ 0: branch        |

DJNZ does **not** modify PSW flags (like INC/DEC). The branch offset is
relative to the byte after the instruction (PC + 2 for Rn form, PC + 3 for
dir form).

#### Subroutines

| Mnemonic           | Encoding        | Bytes | Operation                           |
|--------------------|-----------------|-------|-------------------------------------|
| LCALL addr16       | 0x12 hi lo      | 3     | SP+=2; iram[SP-1]=PCH; iram[SP]=PCL; PC=addr16|
| ACALL addr11       | *(a10:a9:a8)*10 | 2     | SP+=2; push PC+2; PC[10:0]=addr11  |
| RET                | 0x22            | 1     | PCL=iram[SP]; PCH=iram[SP-1]; SP-=2|
| RETI               | 0x32            | 1     | like RET, also re-enables interrupt level|
| NOP                | 0x00            | 1     | No operation                       |

**LCALL stack layout** (grows upward, SP starts at 0x07):
```
Before: SP = N
After push:
  iram[N+1] = PC_low  (address of instruction after LCALL)
  iram[N+2] = PC_high
  SP = N + 2
```
RET reverses: reads iram[SP] as PC_high, iram[SP-1] as PC_low, decrements SP by 2.

---

## Condition flag summary

| Instruction group | CY | AC | OV | P  |
|-------------------|----|----|----|-----|
| ADD, ADDC         | ✓  | ✓  | ✓  | ✓  |
| SUBB              | ✓  | ✓  | ✓  | ✓  |
| MUL AB            | 0  | –  | *  | ✓  |
| DIV AB            | 0  | –  | *  | ✓  |
| INC, DEC          | –  | –  | –  | –  |
| DJNZ              | –  | –  | –  | –  |
| ANL, ORL, XRL, CLR, CPL | – | – | – | ✓ |
| RL, RLC, RR, RRC  | *  | –  | –  | ✓  |
| DA A              | *  | –  | –  | ✓  |
| CJNE              | *  | –  | –  | –  |
| CLR C / SETB C / CPL C | * | – | – | – |
| MOV/MOV bit/SETB/CLR bit | – | – | – | – |

(✓ = updated, – = unchanged, 0 = cleared, * = specific rule above)

**Parity** (PSW.P) is always the even parity of ACC (1 if ACC has odd number
of 1-bits). It is updated whenever ACC changes. This is a hardware-computed
bit in the real chip.

---

## HALT convention

The real 8051 has no HALT instruction. In this simulator, executing opcode
`0xA5` (undefined/reserved on real hardware) is the HALT sentinel that stops
execution and sets `halted=True`. Programs are terminated with `0xA5` in the
same way PDP-11 programs terminate with `0x0000`.

---

## SIM00 protocol implementation

The simulator implements `Simulator[I8051State]` from `simulator_protocol`.

### `I8051State` (frozen dataclass)

```python
@dataclass(frozen=True)
class I8051State:
    pc:     int               # 16-bit program counter
    acc:    int               # accumulator (8-bit)
    b:      int               # B register (8-bit)
    sp:     int               # stack pointer (8-bit)
    dptr:   int               # 16-bit data pointer
    psw:    int               # program status word (8-bit)
    iram:   tuple[int, ...]   # 256 bytes of internal RAM (includes SFRs at 0x80+)
    xdata:  tuple[int, ...]   # 65536 bytes of external data memory
    code:   tuple[int, ...]   # 65536 bytes of code memory
    halted: bool

    @property
    def cy(self) -> bool: return bool((self.psw >> 7) & 1)
    @property
    def ac(self) -> bool: return bool((self.psw >> 6) & 1)
    @property
    def ov(self) -> bool: return bool((self.psw >> 2) & 1)
    @property
    def parity(self) -> bool: return bool(self.psw & 1)
    @property
    def bank(self) -> int: return (self.psw >> 3) & 0x3
```

### Reset state

| Register | Value   | Rationale                               |
|----------|---------|-----------------------------------------|
| PC       | 0x0000  | 8051 always starts at address 0         |
| ACC      | 0x00    | undefined at reset, 0 by convention     |
| B        | 0x00    |                                         |
| SP       | 0x07    | 8051 hardware reset default             |
| DPTR     | 0x0000  |                                         |
| PSW      | 0x00    | Bank 0, CY=0, AC=0, OV=0, P=0          |
| P0–P3    | 0xFF    | All port latches = 0xFF at reset        |
| All IRAM | 0x00    | Zeroed (simulator convention)           |
| All XDATA| 0x00    | Zeroed                                  |

### `load(program: bytes) → None`

1. Call `reset()`.
2. Copy `program` bytes into `_code[0:len(program)]`. Raises `ValueError` if
   `len(program) > 65536`.
3. PC remains 0x0000.

### `step() → StepTrace`

1. If `_halted`: return a `StepTrace` with `mnemonic="HALT"`, `pc_before=pc`,
   `pc_after=pc` (no-op).
2. Fetch opcode byte at `_code[pc]`, increment PC.
3. Decode, fetch additional operand bytes (incrementing PC).
4. Execute: update `_iram`, `_xdata`, `_code`, PSW, SP, DPTR, ACC, B, PC.
5. Recompute parity: `PSW.P = popcount(ACC) & 1`.
6. Return `StepTrace(pc_before, pc_after, mnemonic, state_after=get_state())`.

### `execute(program, max_steps=100_000) → ExecutionResult`

1. Call `load(program)`.
2. Loop: call `step()` up to `max_steps` times.
3. If `_halted`: return `ExecutionResult(ok=True, halted=True, steps=n, ...)`.
4. If `max_steps` exceeded: return `ExecutionResult(ok=False, error="max_steps exceeded")`.
5. On exception: return `ExecutionResult(ok=False, error=str(e))`.

---

## Package layout

```
code/packages/python/intel8051-simulator/
├── pyproject.toml
├── README.md
├── CHANGELOG.md
└── src/
    └── intel8051_simulator/
        ├── __init__.py        # exports: I8051Simulator, I8051State
        ├── py.typed
        ├── state.py           # I8051State frozen dataclass + constants
        ├── flags.py           # arithmetic helpers: add8_flags, sub8_flags, da_flags
        └── simulator.py       # I8051Simulator implementation
tests/
    ├── test_protocol.py       # SIM00 compliance
    ├── test_instructions.py   # per-instruction + per-mode coverage
    ├── test_programs.py       # end-to-end programs
    └── test_coverage.py       # edge cases, flags, DA, MUL/DIV
```

---

## Design notes

### Why no timers or interrupts?

The real 8051 includes hardware timers, a serial port, and a 6-source
interrupt controller. These are inherently time-based and event-driven —
simulating them faithfully requires either cycle-accurate timing or an event
loop. This simulator is a **behavioral instruction-set simulator**, not a
cycle-accurate or peripheral-accurate simulator. Timers and interrupts are
omitted to keep the implementation focused and testable; the SFR addresses
for TCON, TMOD, TL0, TH0, TL1, TH1, SCON, SBUF, IE, IP are modeled as
plain readable/writable bytes with no side effects.

### Harvard architecture in the simulator

In the real 8051, code memory and data memory are physically separate buses.
In the simulator they are two separate Python bytearrays:
- `_code`: 64 KB code memory (loaded by `load()`)
- `_iram`: 256 bytes internal RAM (+ SFR mirror at 0x80–0xFF)
- `_xdata`: 64 KB external data memory

MOVC accesses `_code`; MOVX accesses `_xdata`; all other data operations
access `_iram`. This clean separation matches the real hardware model.

### SFR aliasing

Internal RAM addresses 0x00–0x7F and SFR addresses 0x80–0xFF are both stored
in `_iram[0:256]`. Direct addressing (dir byte in instruction) accesses
`_iram[dir]` for any address. Indirect addressing via @R0/@R1 with R0/R1 ≥
0x80 would access the upper 128-byte RAM of the 8052; in the 8051 base model
this behavior is undefined — the simulator raises `ValueError` for indirect
addresses ≥ 0x80.

After any instruction that modifies ACC, the simulator recomputes PSW.P
(even parity). This matches hardware behavior where P is always live.

### Register bank abstraction

`_r(n)` returns the IRAM address of register Rn in the current bank:

```python
def _r(self, n: int) -> int:
    """IRAM address of Rn in the current register bank."""
    bank = (self._iram[0xD0] >> 3) & 0x3  # PSW RS1:RS0
    return bank * 8 + n
```

This means reading R3 in bank 2 reads `_iram[2*8 + 3] = _iram[19]`.

---

## Test plan

### test_protocol.py — SIM00 compliance
- `isinstance(sim, Simulator)`
- All 5 protocol methods callable
- `execute` returns `ExecutionResult`
- `step` returns `StepTrace`
- `get_state` returns `I8051State`
- `reset`: PC=0, SP=7, PSW=0, IRAM zeroed, ports=0xFF
- `load`: bytes at code[0], PC=0, raises on overflow
- `get_state` is frozen, memory is tuple, registers field present

### test_instructions.py — per-instruction
- **Data transfer**: MOV A/Rn/dir/@Ri/#imm in all combinations
- **MOV dir,dir2**: byte order (src=byte2, dst=byte3)
- **PUSH/POP**: SP increments, stack round-trip
- **XCH/XCHD**: swap semantics
- **Arithmetic**: ADD/ADDC carry chain, overflow detection, DA A BCD cases
- **SUBB**: borrow propagation, CY/AC/OV
- **MUL AB**: B:A product, OV set if B≠0
- **DIV AB**: quotient in A, remainder in B, OV on div-by-zero
- **INC/DEC**: no flag modification confirmed
- **Logic**: ANL/ORL/XRL all operand forms
- **RL/RRC/RLC/RR**: bit rotation including carry thread
- **Bit ops**: CLR/SETB/CPL C and bit, ANL/ORL C,bit and C,/bit
- **Branches**: JZ/JNZ/JC/JNC forward and backward
- **JB/JNB/JBC**: bit test with clear
- **CJNE**: not-equal branch + CY unsigned comparison
- **DJNZ**: count-down loop
- **LJMP/SJMP/JMP @A+DPTR**: all jump forms
- **LCALL/RET**: call/return stack integrity

### test_programs.py — end-to-end
- Sum 1–10 using DJNZ loop
- Multiply using repeated addition
- String copy: MOVX loop copying bytes from xdata src→dst
- Bubble sort 8 bytes in IRAM
- Fibonacci (first 8 terms)
- Factorial (5! using MUL AB)
- BCD addition (DA A usage)
- Nested subroutine calls, stack balance

### test_coverage.py — edge cases
- BCD arithmetic: DA A after ADD of two BCD bytes
- MUL AB overflow (product > 0xFF)
- DIV by zero (OV flag)
- CJNE CY flag: A < imm vs A ≥ imm
- DJNZ exact boundary (counts from 1 → 0 vs 0 → 255)
- Register bank switching (PSW.RS1:RS0)
- Bit address resolution for both RAM area and SFR area
- MOVC @A+DPTR lookup table
- MOVC @A+PC lookup table
- XCHD nibble swap
- Parity recomputation on every ACC change
- SP wrap-around (push 128 items)
- SJMP backward branch (negative offset)
