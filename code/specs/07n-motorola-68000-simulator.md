# Spec 07n — Motorola 68000 Behavioral Simulator

## Overview

The **Motorola 68000** (MC68000, 1979) is a 16/32-bit microprocessor that became
the primary rival to Intel's 8086 in the early 1980s personal computer market.
Designed at Motorola's Austin design center, it was the first chip in Motorola's
massively successful 68000 family (68010, 68020, 68030, 68040, 68060).

Historical milestones powered by the 68000:
- **Apple Lisa** (1983) and **Macintosh** (1984) — the first personal computers
  with a graphical user interface to reach mass market
- **Commodore Amiga** (1985) — revolutionary multimedia computer
- **Atari ST** (1985) — popular in Europe, beloved by musicians for its MIDI ports
- **Sun-1 workstation** (1982) — one of the first Unix workstations
- **Sega Genesis / Mega Drive** (1988) — iconic gaming console
- **NeXT workstation** (1988) — Steve Jobs's platform after leaving Apple, which
  later became the foundation of macOS
- **Palm Pilot** (1996) — the 68328 "DragonBall" variant powered early PDAs

The 68000 represents a major philosophical departure from Intel's 8086:

| Feature            | Intel 8086                      | Motorola 68000                    |
|--------------------|----------------------------------|-----------------------------------|
| Internal width     | 16-bit                           | 32-bit (full internal registers)  |
| External data bus  | 16-bit                           | 16-bit                            |
| External addr bus  | 20-bit (1 MB)                    | 24-bit (16 MB)                    |
| Memory model       | Segmented (CS:IP, DS, SS, ES)    | Linear (flat, no segments)        |
| Register symmetry  | Asymmetric (AX≠BX≠CX≠DX)        | Orthogonal (D0–D7 interchangeable)|
| Byte order         | Little-endian                    | Big-endian                        |
| Addressing modes   | Limited (6 practical modes)      | Rich (14 modes)                   |
| Stack operations   | PUSH/POP instructions            | MOVE with -(An)/(An)+ modes       |
| Multiply/divide    | 16×16 → 32, special registers    | 16×16 → 32, stays in Dn          |

Where Intel chose backwards compatibility and market timing, Motorola chose
clean design. The result was an ISA that CPU architects still cite as exemplary.

This spec defines Layer **07n** — a Python behavioral simulator for the 68000
following the SIM00 `Simulator[M68KState]` protocol.

---

## Architecture

### Registers

#### Data registers (8 × 32-bit)

| Name | Description |
|------|-------------|
| D0   | General-purpose data register 0 |
| D1   | General-purpose data register 1 |
| D2   | General-purpose data register 2 |
| D3   | General-purpose data register 3 |
| D4   | General-purpose data register 4 |
| D5   | General-purpose data register 5 |
| D6   | General-purpose data register 6 |
| D7   | General-purpose data register 7 |

Data registers hold 32-bit values and support byte (bits 7–0), word (bits 15–0),
and longword (all 32 bits) operations. Byte and word operations affect only the
low-order portion; the rest of the register is unchanged.

#### Address registers (8 × 32-bit)

| Name | Description |
|------|-------------|
| A0   | General-purpose address register 0 |
| A1   | General-purpose address register 1 |
| A2   | General-purpose address register 2 |
| A3   | General-purpose address register 3 |
| A4   | General-purpose address register 4 |
| A5   | General-purpose address register 5 |
| A6   | General-purpose address register 6 (frame pointer by convention) |
| A7   | Stack pointer (SP). Two physical registers: USP and SSP. |

Address registers are strictly 32-bit. There are no byte operations on address
registers (only word and longword, and word values are sign-extended to 32 bits
before being written). The processor maintains two A7 registers:
- **USP** (user stack pointer) — used in user mode
- **SSP** (supervisor stack pointer) — used in supervisor mode, also called ISP

Our simulator runs in supervisor mode only, so A7 is always SSP.

#### Program counter

| Name | Description |
|------|-------------|
| PC   | 32-bit program counter (address of next instruction to fetch) |

The 68000's address bus is 24 bits wide, so only the low 24 bits of PC are
significant (0x000000–0xFFFFFF). Our simulator models 16 MB of memory.

#### Status register (SR, 16-bit)

```
Bit: 15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
      T1  T0   S   M   0  I2  I1  I0   0   0   0   X   N   Z   V   C
      ─────────────────────────────────   ──────────────────────────
           System byte                         Condition code register (CCR)
```

System byte (bits 15–8) — supervisor mode only:
- **T1/T0** — trace enable (00=off, 01=trace on branch, 10=trace all, 11=undef)
- **S** — supervisor/user mode (1=supervisor, 0=user)
- **M** — master/interrupt state (68020+ only; 0 on 68000)
- **I2:I0** — interrupt priority mask (0=allow all, 7=block all)

Condition code register / CCR (bits 7–0, only bits 4–0 are defined):
- **X** (bit 4) — Extend: set same as C on most arithmetic; used by extended-precision ops
- **N** (bit 3) — Negative: copy of the MSB of the result
- **Z** (bit 2) — Zero: result is zero
- **V** (bit 1) — oVerflow: signed overflow occurred
- **C** (bit 0) — Carry: unsigned overflow/borrow out of MSB

Note: The 68000 has **no Auxiliary Carry** (no BCD-carry between nibbles in
arithmetic — DAA is handled differently than on the 8080/8086).
The **X flag** is unique: it is set the same way as C by ADD/SUB, but CLR/NOT/TST
instructions do not touch X (unlike C). Extended arithmetic (ADDX, SUBX) uses
X as the carry input, not C.

---

### Memory model

The 68000 has a **linear 24-bit address space**: no segments, no bank switching,
just a flat array of bytes from address 0x000000 to 0xFFFFFF (16 MB).

- **Big-endian**: the most-significant byte is at the lowest address.
- Word (16-bit) and longword (32-bit) reads/writes must be **word-aligned**
  (address even). An odd address causes an Address Error exception. Our simulator
  raises `ValueError` on misaligned word/long access.
- The **exception vector table** occupies the first 1024 bytes (256 × 4-byte vectors).
  On real hardware, address 0x000000 holds the initial SSP and address 0x000004
  holds the initial PC. Our simulator loads programs starting at address 0x1000
  and sets PC=0x1000, SP=0xF00000 on reset.
- I/O is memory-mapped (no separate IN/OUT port space, unlike the 8086).

Memory layout in our simulator:
```
0x000000 – 0x0003FF   Exception vector table (1 KB)
0x001000 – …          Program load area (configurable)
0xF00000              Initial supervisor stack pointer
0xFFFFFF              Top of addressable memory
```

---

### Effective address (EA) encoding

Instructions encode memory operands as a 6-bit EA field:
```
mode[2:0]  reg[2:0]  — Addressing mode
  000        Dn      Data register direct
  001        An      Address register direct
  010        An      (An)          — address register indirect
  011        An      (An)+         — indirect with postincrement
  100        An      -(An)         — indirect with predecrement
  101        An      d16(An)       — indirect + 16-bit signed displacement
  110        An      d8(An,Xn.sz)  — indirect + 8-bit disp + index register
  111        000     (abs).W       — absolute short (sign-extended 16-bit)
  111        001     (abs).L       — absolute long (32-bit)
  111        010     d16(PC)       — PC-relative + 16-bit displacement
  111        011     d8(PC,Xn.sz)  — PC-relative + 8-bit disp + index
  111        100     #imm          — immediate data
```

The index word (for modes 110 and 111.011) encodes which register to use:
```
Bit 15:    D/A (0=data register, 1=address register)
Bits 14–12: register number
Bit 11:    W/L (0=sign-extend lower word of Xn to 32-bit, 1=use full 32-bit Xn)
Bits 7–0:  8-bit signed displacement
```

Size codes (for most instructions other than MOVE):
```
00 = byte   (8-bit)
01 = word   (16-bit)
10 = long   (32-bit)
```

Size codes for the MOVE instruction (different encoding):
```
01 = byte
11 = word
10 = long
```

---

## Instruction Set

### Summary table

Instructions are 16-bit words (+ optional 16-bit or 32-bit extension words).
Every instruction fetch must be from a word-aligned address.

| Instruction    | Opcode pattern              | Description |
|---------------|------------------------------|-------------|
| NOP           | `4E71`                       | No operation |
| STOP #imm     | `4E72 xxxx`                  | Load imm→SR, halt (simulator halt) |
| RESET         | `4E70`                       | External reset (no-op in sim) |
| RTS           | `4E75`                       | Return from subroutine |
| RTR           | `4E77`                       | Return and restore CCR |
| MOVE.sz src,dst | `00ss DRd d...`            | Move data between EA ↔ register |
| MOVEA.sz src,An | see MOVE with An dest      | Move to address register |
| MOVEQ #d8,Dn  | `0111rrr0 dddddddd`          | Move signed 8-bit immediate to Dn |
| MOVE SR,Dn    | `40C0+rrr`                   | Copy SR to data register |
| MOVE #imm,SR  | `46FC xxxx`                  | Load immediate into SR |
| MOVE CCR,Dn   | `42C0+rrr`                   | Copy CCR to data register |
| MOVE #imm,CCR | `44FC xxxx`                  | Load immediate into CCR |
| CLR.sz <ea>   | `0100 0010 ss mmm rrr`       | Clear (set to zero) |
| NOT.sz <ea>   | `0100 0110 ss mmm rrr`       | Bitwise NOT |
| NEG.sz <ea>   | `0100 0100 ss mmm rrr`       | Negate (0 − <ea>) |
| NEGX.sz <ea>  | `0100 0000 ss mmm rrr`       | Negate with extend |
| TST.sz <ea>   | `0100 1010 ss mmm rrr`       | Test (set N/Z, clear V/C) |
| SWAP Dn       | `4840+rrr`                   | Swap high and low 16-bit halves of Dn |
| EXT.W Dn      | `4880+rrr`                   | Sign-extend byte → word in Dn |
| EXT.L Dn      | `48C0+rrr`                   | Sign-extend word → long in Dn |
| ADD.sz <ea>,Dn | `1101 rrr0 ss ea`           | Dn + <ea> → Dn |
| ADD.sz Dn,<ea> | `1101 rrr1 ss ea`           | <ea> + Dn → <ea> |
| ADDI.sz #imm,<ea> | `0000 0110 ss ea`       | <ea> + imm → <ea> |
| ADDQ.sz #n,<ea>   | `0101 nnn0 ss ea`       | <ea> + n(1–8) → <ea> |
| ADDX.sz Ds,Dd     | `1101 rrr1 ss 00 0 rrr` | Dd + Ds + X → Dd |
| SUB.sz <ea>,Dn | `1001 rrr0 ss ea`           | Dn − <ea> → Dn |
| SUB.sz Dn,<ea> | `1001 rrr1 ss ea`           | <ea> − Dn → <ea> |
| SUBI.sz #imm,<ea> | `0000 0100 ss ea`       | <ea> − imm → <ea> |
| SUBQ.sz #n,<ea>   | `0101 nnn1 ss ea`       | <ea> − n(1–8) → <ea> |
| SUBX.sz Ds,Dd     | `1001 rrr1 ss 00 0 rrr` | Dd − Ds − X → Dd |
| AND.sz <ea>,Dn | `1100 rrr0 ss ea`           | Dn AND <ea> → Dn |
| AND.sz Dn,<ea> | `1100 rrr1 ss ea`           | <ea> AND Dn → <ea> |
| ANDI.sz #imm,<ea> | `0000 0010 ss ea`       | <ea> AND imm → <ea> |
| OR.sz <ea>,Dn  | `1000 rrr0 ss ea`           | Dn OR <ea> → Dn |
| OR.sz Dn,<ea>  | `1000 rrr1 ss ea`           | <ea> OR Dn → <ea> |
| ORI.sz #imm,<ea>  | `0000 0000 ss ea`       | <ea> OR imm → <ea> |
| EOR.sz Dn,<ea> | `1011 rrr1 ss ea`           | <ea> XOR Dn → <ea> |
| EORI.sz #imm,<ea> | `0000 1010 ss ea`       | <ea> XOR imm → <ea> |
| MULU.W <ea>,Dn | `1100 rrr0 11 ea`           | Dn.W × <ea>.W → Dn.L (unsigned) |
| MULS.W <ea>,Dn | `1100 rrr1 11 ea`           | Dn.W × <ea>.W → Dn.L (signed) |
| DIVU.W <ea>,Dn | `1000 rrr0 11 ea`           | Dn.L ÷ <ea>.W → Dn (quot in low word, rem in high) |
| DIVS.W <ea>,Dn | `1000 rrr1 11 ea`           | Dn.L ÷ <ea>.W → Dn (signed) |
| CMP.sz <ea>,Dn | `1011 rrr0 ss ea`           | Dn − <ea>, set flags only |
| CMPA.W <ea>,An | `1011 rrr0 11 ea`           | An − <ea>.W (sign-ext), set flags |
| CMPA.L <ea>,An | `1011 rrr1 11 ea`           | An − <ea>.L, set flags |
| CMPI.sz #imm,<ea> | `0000 1100 ss ea`       | <ea> − imm, set flags only |
| ASL.sz <ea>   | `1110 0001 11 ea` (mem)      | Arithmetic shift left (memory) |
| ASR.sz <ea>   | `1110 0000 11 ea` (mem)      | Arithmetic shift right (memory) |
| LSL.sz <ea>   | `1110 0011 11 ea` (mem)      | Logical shift left |
| LSR.sz <ea>   | `1110 0010 11 ea` (mem)      | Logical shift right |
| ROL.sz <ea>   | `1110 0111 11 ea` (mem)      | Rotate left |
| ROR.sz <ea>   | `1110 0110 11 ea` (mem)      | Rotate right |
| ROXL.sz <ea>  | `1110 0101 11 ea` (mem)      | Rotate left through extend |
| ROXR.sz <ea>  | `1110 0100 11 ea` (mem)      | Rotate right through extend |
| ASL/ASR/LSL/LSR/ROL/ROR/ROXL/ROXR Dn | `1110 ccc d ss i tt rrr` | Shift/rotate register |
| BRA #disp     | `6000 dd` or `6000 0000 dddd dddd` | Branch always |
| BSR #disp     | `6100 dd` or `6100 0000 dddd dddd` | Branch to subroutine |
| Bcc #disp     | `0110 cccc dddd dddd`        | Branch if condition |
| DBcc Dn,#disp | `0101 cccc 1100 1 rrr xxxx`  | Test/Decrement/Branch |
| JMP <ea>      | `4E C0+ea`                   | Jump to EA |
| JSR <ea>      | `4E 80+ea`                   | Jump to subroutine |
| LEA <ea>,An   | `0100 rrr1 11 ea`            | Load effective address |
| PEA <ea>      | `4840 + 01 ea` → `0100 1000 01 ea` | Push effective address |
| LINK An,#d16  | `4E50+rrr xxxx`              | Link and allocate stack frame |
| UNLK An       | `4E58+rrr`                   | Unlink stack frame |
| TRAP #n       | `4E40+n`                     | Software trap (n = 0–15) |

### Condition codes (for Bcc and DBcc)

| Code | Value | Mnemonic | Condition                      |
|------|-------|----------|--------------------------------|
|  T   | 0000  | BT/BRA   | True — always branch           |
|  F   | 0001  | BF       | False — never branch           |
| HI   | 0010  | BHI      | Unsigned higher: !C & !Z       |
| LS   | 0011  | BLS      | Unsigned lower or same: C ∣ Z  |
| CC   | 0100  | BCC/BHI  | Carry clear: !C                |
| CS   | 0101  | BCS/BLO  | Carry set: C                   |
| NE   | 0110  | BNE      | Not equal: !Z                  |
| EQ   | 0111  | BEQ      | Equal: Z                       |
| VC   | 1000  | BVC      | Overflow clear: !V             |
| VS   | 1001  | BVS      | Overflow set: V                |
| PL   | 1010  | BPL      | Plus (positive or zero): !N    |
| MI   | 1011  | BMI      | Minus (negative): N            |
| GE   | 1100  | BGE      | Signed ≥: N == V               |
| LT   | 1101  | BLT      | Signed <: N ≠ V                |
| GT   | 1110  | BGT      | Signed >: !Z & (N == V)        |
| LE   | 1111  | BLE      | Signed ≤: Z ∣ (N ≠ V)         |

---

## Condition Code Updates

### ADD/ADDI/ADDQ (same for X flag as C):
- **X** = set same as C
- **N** = MSB of result
- **Z** = result is zero
- **V** = signed overflow (both operands same sign, result opposite)
- **C** = unsigned carry (result > max unsigned)

### ADDX:
- **X**, **C** = carry out
- **N** = MSB of result
- **Z** = result is zero, BUT: Z is cleared if result ≠ 0 (never set by ADDX alone!)
- **V** = signed overflow

### SUB/SUBI/SUBQ (same as ADD):
- **X** = set same as C (borrow)
- **N** = MSB of result
- **Z** = result is zero
- **V** = signed overflow
- **C** = borrow (minuend < subtrahend)

### AND/OR/EOR/NOT/CLR:
- **N** = MSB of result
- **Z** = result is zero
- **V** = 0 (cleared)
- **C** = 0 (cleared)
- **X** = unchanged

### NEG:
- **X** = **C** = (result ≠ 0)
- **N** = MSB of result
- **Z** = result is zero
- **V** = (source == 0x80 for byte, 0x8000 for word, 0x80000000 for long)

### CMP/CMPA/CMPI/TST:
- Same as SUB but do not write result, do not modify X

### Shifts and rotates:
- **ASL/ASR**: N, Z set from result; C = last bit shifted out; V = any overflow (sign change for ASL); X = C
- **LSL/LSR**: N, Z set; C = last bit out; V = 0; X = C
- **ROL/ROR**: N, Z set; C = last bit rotated; V = 0; X unchanged
- **ROXL/ROXR**: N, Z set; C = last bit; X = C; V = 0

---

## SIM00 Protocol Implementation

This simulator implements `Simulator[M68KState]` from `simulator_protocol`.

### `M68KSimulator.reset()`

Clears all data registers (D0–D7 = 0), address registers (A0–A6 = 0),
sets A7 (SSP) = 0x00F00000, PC = 0x001000, SR = 0x2700 (supervisor, IMask=7).
Zeroes memory. Clears halt flag and trace list.

### `M68KSimulator.load(program)`

Copies `program` bytes into memory starting at the load address (0x001000).
Does **not** reset CPU state (registers, PC, SP) — call `reset()` first if
you want a clean state. Used by `execute()` internally after `reset()`.

### `M68KSimulator.step()`

Fetches, decodes, and executes one instruction. Returns a `StepTrace` with:
- `pc_before` — PC value before fetch
- `pc_after` — PC value after execution (next instruction or jump target)
- `mnemonic` — short assembly-language form (e.g., `"MOVE.W D1, D0"`)
- `description` — `"<mnemonic> @ 0x<hex pc_before>"`

Raises `RuntimeError` if the CPU is halted.

### `M68KSimulator.execute(program, max_steps=100_000)`

1. Calls `reset()`
2. Calls `load(program)`
3. Loops calling `step()` until halted or `max_steps` exceeded
4. Returns `ExecutionResult[M68KState]`

A STOP instruction halts the simulator.
TRAP #15 is treated as an alternate halt (for convenience).

### `M68KSimulator.get_state()`

Returns a frozen `M68KState` snapshot of the current CPU state, converting
all mutable lists/bytearrays to tuples.

---

## M68KState specification

```python
@dataclass(frozen=True)
class M68KState:
    # Data registers (32-bit unsigned)
    d0: int; d1: int; d2: int; d3: int
    d4: int; d5: int; d6: int; d7: int

    # Address registers (32-bit unsigned, A7 = supervisor stack pointer)
    a0: int; a1: int; a2: int; a3: int
    a4: int; a5: int; a6: int; a7: int

    # Program counter (24-bit effective, stored as 32-bit unsigned)
    pc: int

    # Status register (16-bit)
    sr: int

    # Individual CCR bits (derived from SR)
    # x: bool  # extend
    # n: bool  # negative
    # z: bool  # zero
    # v: bool  # overflow
    # c: bool  # carry

    # Halt flag
    halted: bool

    # Memory (16 MB = 16,777,216 bytes)
    memory: tuple[int, ...]
```

Properties on M68KState: `.x`, `.n`, `.z`, `.v`, `.c` extract CCR bits from `sr`.
Properties `.d` and `.a` return tuples of all data/address registers.

---

## Package layout

```
code/packages/python/motorola-68000-simulator/
├── BUILD
├── CHANGELOG.md
├── README.md
├── pyproject.toml
└── src/
    └── motorola_68000_simulator/
        ├── __init__.py
        ├── py.typed
        ├── state.py        # M68KState frozen dataclass
        ├── flags.py        # CCR computation helpers
        └── simulator.py    # M68KSimulator
tests/
├── __init__.py
├── test_instructions.py    # unit tests per instruction
├── test_programs.py        # multi-instruction programs
├── test_protocol.py        # SIM00 protocol conformance
└── test_coverage.py        # edge cases for full coverage
```

---

## Design decisions and divergences

1. **Always supervisor mode**: The simulator does not implement user/supervisor
   mode switching. The S bit in SR is always 1. TRAP and STOP do not switch modes.

2. **STOP halts immediately**: Real hardware STOP enters a low-power wait state
   until an interrupt. Our simulator treats STOP as a full halt (like HLT on x86).
   The SR is updated from the immediate operand, then execution stops.

3. **TRAP #15 also halts**: By convention many M68K simulators use TRAP #15 as
   a "halt/OS call" mechanism. We halt on TRAP #15; TRAP #0–#14 call a stub handler
   that simply records the trap number in D7 and continues.

4. **No address error on odd access**: Real 68000 hardware raises an address error
   fault when a word/long access targets an odd address. Our simulator raises
   Python `ValueError` immediately, which fails the test (an odd-aligned program
   is a bug). This simplifies exception-vector handling.

5. **24-bit address wrap**: Addresses are masked with 0xFFFFFF. Accesses above
   0xFFFFFF wrap around to the start of the address space.

6. **No bus errors, illegal instruction traps**: Unrecognised opcodes raise
   Python `RuntimeError` with a descriptive message.

7. **DIVU/DIVS overflow**: On real hardware, division overflow sets V=1 and does
   not update the register. Our simulator follows this semantics.

8. **Load address 0x001000**: Programs load at 0x001000, leaving the vector table
   (0x000000–0x0003FF) and a small guard region intact. Initial PC = 0x001000.
