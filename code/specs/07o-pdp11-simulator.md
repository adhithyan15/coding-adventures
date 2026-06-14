# Spec 07o — DEC PDP-11 Behavioral Simulator

## Overview

The **DEC PDP-11** (1970) is a 16-bit minicomputer family designed at Digital
Equipment Corporation (DEC) and represents one of the most influential computer
architectures in history. While the Intel 4004 (1971) was launching the
microprocessor revolution, the PDP-11 was already teaching a generation of
engineers what elegant computer design looked like at the minicomputer scale.

Historical milestones powered by the PDP-11:
- **UNIX** (1970) — Ken Thompson and Dennis Ritchie wrote the first version of
  Unix on a PDP-11/20 at Bell Labs. The C language was designed specifically
  to be PDP-11-friendly; the fact that `int` defaults to the native word size
  and that pointers and integers can be freely cast traces directly to the
  PDP-11's architecture.
- **C language** (1972) — Dennis Ritchie's C was created as a portable
  alternative to PDP-11 assembly. The bitwise operators, the `++`/`--`
  operators (inspired by PDP-11 autoincrement/autodecrement addressing), and
  the memory model of C are deeply PDP-11-shaped.
- **FORTRAN compilers, BASIC interpreters, early LISP** — the PDP-11 was the
  platform of choice for language research in the 1970s.
- **Eunice**, **RSTS/E**, **RT-11**, **RSX-11** — a rich ecosystem of
  operating systems evolved on PDP-11 hardware.

The PDP-11 was remarkable for introducing the concept of **orthogonal ISA**
(independent instruction set architecture): any addressing mode can be
applied to any operand of any instruction. This uniformity made the ISA far
easier to learn, use, and compile for than contemporaries like the IBM 360 or
Intel 8008.

| Feature            | PDP-11 (1970)                    | Intel 8080 (1974, for comparison) |
|--------------------|-----------------------------------|-------------------------------------|
| Width              | 16-bit                            | 8-bit                               |
| Registers          | 8 GPRs (R0-R7, incl. SP and PC)  | A + BC + DE + HL + SP (asymmetric) |
| Addressing modes   | 8 (apply to **any** register)     | 5 (register-specific)               |
| Memory model       | Flat 64 KB                        | Flat 64 KB                          |
| Byte order         | Little-endian                     | Little-endian                       |
| Byte ops           | Yes (MOVB, CLRB, etc.)            | Yes (8-bit accumulator)             |
| PC in GPR file     | **Yes** (R7 = PC)                 | No                                  |
| SP in GPR file     | **Yes** (R6 = SP)                 | No (separate SP)                    |
| Auto-inc/dec       | Yes (register addressing modes)   | No (separate INX/DCX)               |

The elegance of putting PC and SP in the general-purpose register file means
that JSR and RTS are not special — they use the same autoincrement/autodecrement
addressing modes as everything else. C's `++p` and `*p++` mirror PDP-11's
`(Rn)+` (register deferred autoincrement) and `@(Rn)+` patterns exactly.

This spec defines Layer **07o** — a Python behavioral simulator for the PDP-11
following the SIM00 `Simulator[PDP11State]` protocol.

---

## Architecture

### Registers

The PDP-11 has **eight 16-bit general-purpose registers**: R0 through R7.
By convention two have dedicated roles:

| Name | Alias | Role |
|------|-------|------|
| R0   | —     | General-purpose |
| R1   | —     | General-purpose |
| R2   | —     | General-purpose |
| R3   | —     | General-purpose |
| R4   | —     | General-purpose |
| R5   | —     | General-purpose (frame pointer by convention) |
| R6   | SP    | Stack pointer — pre-decremented on push, post-incremented on pop |
| R7   | PC    | Program counter — always points to the next instruction word to fetch |

All registers are 16 bits wide. All arithmetic is modulo 2¹⁶. There is no
sign extension on register write; the hardware stores exactly the 16 bits given.

### Processor Status Word (PSW)

The **Processor Status Word** is a 16-bit register with the following layout:

```
Bit: 15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
     [ Current mode ] [ Prev mode ] [ IPL (interrupt priority) ] T N Z V C
```

For this behavioral simulator we only track the four condition codes in bits
3–0:

| Bit | Flag | Name     | Set when…                                       |
|-----|------|----------|-------------------------------------------------|
|  3  |  N   | Negative | Result MSB is 1 (word bit 15, byte bit 7)       |
|  2  |  Z   | Zero     | Result is exactly 0                             |
|  1  |  V   | Overflow | Signed overflow occurred                        |
|  0  |  C   | Carry    | Carry-out from MSB (unsigned overflow)          |

The T (trap) bit and interrupt priority level (IPL) are not simulated.

---

## Addressing Modes

The PDP-11's **eight addressing modes** are the heart of its orthogonality.
Each instruction encodes its operand(s) as `(mode, register)` pairs using
3 bits each — 6 bits total per operand:

```
Bits 5–3: mode (0–7)
Bits 2–0: register (R0–R7)
```

### Mode Table

| Mode | Assembly syntax | Name                     | Effective address (EA)                           |
|------|----------------|--------------------------|--------------------------------------------------|
|  0   | `Rn`           | Register                 | Operand is Rn itself (no memory access)          |
|  1   | `(Rn)` or `@Rn`| Register deferred        | EA = Rn                                          |
|  2   | `(Rn)+`        | Autoincrement            | EA = Rn; Rn += size (2 for word, 1 for byte)    |
|  3   | `@(Rn)+`       | Autoincrement deferred   | EA = M[Rn]; Rn += 2 (always word pointer)       |
|  4   | `-(Rn)`        | Autodecrement            | Rn -= size; EA = Rn                              |
|  5   | `@-(Rn)`       | Autodecrement deferred   | Rn -= 2; EA = M[Rn]                             |
|  6   | `X(Rn)`        | Index                    | EA = Rn + next-word (sign-extended); PC += 2     |
|  7   | `@X(Rn)`       | Index deferred           | EA = M[Rn + next-word]; PC += 2                  |

### PC-Relative Addressing (R7 special cases)

Because R7 *is* the PC, the addressing modes applied to R7 produce the
classic "immediate" and "absolute" syntaxes used in PDP-11 assembly:

| Mode | Rn = R7 | Assembly sugar | Meaning                                    |
|------|---------|----------------|--------------------------------------------|
|  2   | R7=PC   | `#n`           | Immediate: operand is the next word        |
|  3   | R7=PC   | `@#addr`       | Absolute: EA = next word (absolute address)|
|  6   | R7=PC   | `addr`         | Relative: EA = PC + next-word displacement |
|  7   | R7=PC   | `@addr`        | Relative deferred: EA = M[PC + disp]      |

Since PC always points past the current instruction word *before* addressing
is evaluated, `#n` simply reads the word at the current PC and advances it —
which is exactly what mode 2 (autoincrement) does when applied to R7.

### Stack conventions (R6 = SP)

Mode 4 on R6: `-(SP)` — pre-decrement SP by 2, then write to M[SP]. **PUSH**.
Mode 2 on R6: `(SP)+` — read from M[SP], then post-increment SP by 2. **POP**.

JSR uses autodecrement to push the link register; RTS uses autoincrement to
pop. These are not hard-wired stack operations — they are the same autoincrement/
autodecrement mode that works on R0–R7.

---

## Instruction Set

### Double-Operand Instructions (word)

```
Format: [  opcode(4)  |  src mode(3) | src reg(3) | dst mode(3) | dst reg(3)  ]
Bits:    15          12  11          9  8          6  5          3  2          0
```

| Opcode | Mnemonic | Operation                          | Flags updated |
|--------|----------|------------------------------------|---------------|
| 0001   | MOV      | dst ← src                          | N, Z, V=0     |
| 0010   | CMP      | src − dst (result discarded)       | N, Z, V, C    |
| 0011   | BIT      | src AND dst (result discarded)     | N, Z, V=0     |
| 0100   | BIC      | dst ← dst AND NOT src              | N, Z, V=0     |
| 0101   | BIS      | dst ← dst OR src                   | N, Z, V=0     |
| 0110   | ADD      | dst ← dst + src                    | N, Z, V, C    |

### Double-Operand Instructions (byte)

Byte instructions are encoded with bit 15 = 1 (adding 0x8000 to the word
opcode):

| Opcode | Mnemonic | Operation                          |
|--------|----------|------------------------------------|
| 1001   | MOVB     | dst (byte) ← src (byte); sign-extend when dst is register |
| 1010   | CMPB     | src(B) − dst(B) (discarded)        |
| 1011   | BITB     | src(B) AND dst(B) (discarded)      |
| 1100   | BICB     | dst(B) ← dst(B) AND NOT src(B)    |
| 1101   | BISB     | dst(B) ← dst(B) OR src(B)         |

Note: there is no ADDB (byte add) in the PDP-11 ISA.

### Single-Operand Instructions

```
Format: [    opcode(10)     | dst mode(3) | dst reg(3) ]
Bits:    15               6   5          3   2         0
```

| Encoding   | Mnemonic | Operation                               | Flags |
|------------|----------|-----------------------------------------|-------|
| 0000 0011 0x | SWAB   | Swap bytes in dst word                  | N,Z,V=0,C=0 |
| 0000 1000 0x | CLR    | dst ← 0                                 | N=0,Z=1,V=0,C=0 |
| 0000 1000 1x | CLRB   | dst(byte) ← 0                           | N=0,Z=1,V=0,C=0 |
| 0000 1001 0x | COM    | dst ← NOT dst                           | N,Z,V=0,C=1 |
| 0000 1001 1x | COMB   | dst(byte) ← NOT dst(byte)               | N,Z,V=0,C=1 |
| 0000 1010 0x | INC    | dst ← dst + 1                           | N,Z,V |
| 0000 1010 1x | INCB   | dst(byte) ← dst(byte) + 1              | N,Z,V |
| 0000 1011 0x | DEC    | dst ← dst − 1                           | N,Z,V |
| 0000 1011 1x | DECB   | dst(byte) ← dst(byte) − 1              | N,Z,V |
| 0000 1100 0x | NEG    | dst ← 0 − dst                           | N,Z,V,C |
| 0000 1100 1x | NEGB   | dst(byte) ← 0 − dst(byte)              | N,Z,V,C |
| 0000 1101 0x | ADC    | dst ← dst + C                           | N,Z,V,C |
| 0000 1101 1x | ADCB   | dst(byte) ← dst(byte) + C             | N,Z,V,C |
| 0000 1110 0x | SBC    | dst ← dst − C                           | N,Z,V,C |
| 0000 1110 1x | SBCB   | dst(byte) ← dst(byte) − C             | N,Z,V,C |
| 0000 1111 0x | TST    | src − 0 (flags only)                    | N,Z,V=0,C=0 |
| 0000 1111 1x | TSTB   | src(byte) − 0 (flags only)             | N,Z,V=0,C=0 |
| 0110 0000 0x | ROR    | Rotate right through C (word)           | N,Z,V,C |
| 0110 0000 1x | RORB   | Rotate right through C (byte)           | N,Z,V,C |
| 0110 0001 0x | ROL    | Rotate left through C (word)            | N,Z,V,C |
| 0110 0001 1x | ROLB   | Rotate left through C (byte)            | N,Z,V,C |
| 0110 0010 0x | ASR    | Arithmetic shift right (word)           | N,Z,V,C |
| 0110 0010 1x | ASRB   | Arithmetic shift right (byte)           | N,Z,V,C |
| 0110 0011 0x | ASL    | Arithmetic shift left (word)            | N,Z,V,C |
| 0110 0011 1x | ASLB   | Arithmetic shift left (byte)            | N,Z,V,C |
| 0110 1100 0x | SUB (single dst) | Not standard; see double-op  | —     |

The `x` in the encoding column stands for the 6-bit operand field
(mode × 8 + reg).

### Control Flow

#### Branches (8-bit signed offset, in words)

```
Format: [ opcode(8) | offset(8) ]
Bits:    15        8  7        0
```

Offset is a **signed 8-bit integer** giving the number of *words* to jump
relative to the next instruction (i.e., relative to PC after the branch
word is fetched).  EA = PC + 2 × offset.

| Opcode (hex) | Mnemonic | Condition             |
|--------------|----------|-----------------------|
| 0x0001       | BR       | Always                |
| 0x0002       | BNE      | Z = 0                 |
| 0x0003       | BEQ      | Z = 1                 |
| 0x0004       | BGE      | N XOR V = 0           |
| 0x0005       | BLT      | N XOR V = 1           |
| 0x0006       | BGT      | Z = 0 AND (N XOR V) = 0 |
| 0x0007       | BLE      | Z = 1 OR (N XOR V) = 1  |
| 0x0100       | BPL      | N = 0                 |
| 0x0101       | BMI      | N = 1                 |
| 0x0102       | BHI      | C = 0 AND Z = 0       |
| 0x0103       | BLOS     | C = 1 OR Z = 1        |
| 0x0104       | BVC      | V = 0                 |
| 0x0105       | BVS      | V = 1                 |
| 0x0106       | BCC/BHIS | C = 0                 |
| 0x0107       | BCS/BLO  | C = 1                 |

#### Jump (JMP)

```
Encoding: 0000 0000 01 | mode(3) | reg(3)
```

JMP transfers control to the *effective address* computed from the given
addressing mode. JMP with mode 0 (register direct) is illegal.

#### Subroutine Calls (JSR / RTS)

```
JSR: 0000 1000 rr | mode(3) | reg(3)   (rr = link register number, 3 bits)
RTS: 0000 0010 00 000 rrr               (rrr = link register, 3 bits)
```

**JSR reg, dst**:
1. `-(SP)` ← reg  (push reg onto stack)
2. reg ← PC       (link register gets return address)
3. PC ← EA(dst)   (jump to subroutine)

**RTS reg**:
1. PC ← reg       (return to saved PC in link register)
2. reg ← `(SP)+`  (pop saved link register from stack)

The most common call convention is `JSR PC, addr` (link reg = R7 = PC):
1. `-(SP)` ← PC   (push return address)
2. PC ← PC        (link reg = PC, no-op since both are PC)
3. PC ← EA        (jump to subroutine)

And `RTS PC` simply pops SP into PC — equivalent to a normal subroutine
return on any other architecture.

#### SOB (Subtract One and Branch)

```
Encoding: 0111 11 | reg(3) | offset(6)
```

`SOB reg, label`: decrement reg; if reg ≠ 0, branch backward by offset words.
Offset is a **6-bit unsigned** value subtracted from PC: EA = PC − 2 × offset.
SOB always branches *backward*; it cannot branch forward.

### Miscellaneous

| Encoding (hex) | Mnemonic | Operation                                  |
|----------------|----------|--------------------------------------------|
| 0x0000         | HALT     | Halt the processor (simulator stop signal) |
| 0x00A0–0x00A7  | CLN/CLZ/CLV/CLC/SEN/SEZ/SEV/SEC | Set/clear individual PSW flags |
| 0x0004         | IOT      | I/O trap (not simulated)                   |
| 0x0005         | RESET    | Bus reset (not simulated)                  |
| 0x0006         | RTI      | Return from interrupt (not simulated)      |
| 0x0007         | MFPT     | Move from processor type (not simulated)   |
| 0x00A0         | NOP      | No operation (encoded as CLN+CLZ+CLV+CLC=0xA0)|

The simulator recognizes NOP (0x00A0) as an alias; other unrecognized
opcodes raise a Python `ValueError`.

---

## Memory Model

- **Size**: 65 536 bytes (16-bit address space, byte-addressed).
- **Layout**: flat; no segmentation.
- **Word access**: always 16-bit, little-endian, must be word-aligned (even
  address). Odd-address word access is a bus error on real hardware; the
  simulator raises `ValueError`.
- **Byte access**: any address, 8-bit.
- **Load address**: programs are loaded starting at address `0x1000`.
- **Stack**: grows *downward*. Initial SP (R6) = `0xF000`.

---

## Simulator Protocol (SIM00)

```python
from simulator_protocol import Simulator
from pdp11_simulator import PDP11Simulator, PDP11State

sim: Simulator[PDP11State] = PDP11Simulator()
```

### `PDP11State`

A dataclass capturing the complete, serializable CPU state:

```python
@dataclass
class PDP11State:
    r: list[int]       # R0–R7 (len=8), each 16-bit unsigned
    psw: int           # Processor Status Word, bits 3-0 = N,Z,V,C
    memory: bytes      # Full 64 KB snapshot
    halted: bool       # True if HALT was executed
```

### Protocol Methods

| Method | Signature | Behaviour |
|--------|-----------|-----------|
| `reset` | `() → PDP11State` | Zero all registers; zero memory; R6=0xF000, R7=0x1000; PSW=0; halted=False |
| `load`  | `(program: bytes) → PDP11State` | Reset then copy `program` into memory starting at 0x1000 |
| `step`  | `() → PDP11State` | Fetch, decode, and execute one instruction; return new state |
| `execute` | `(max_steps: int = 10_000) → PDP11State` | Loop calling `step()` until `halted=True` or `max_steps` reached |
| `get_state` | `() → PDP11State` | Return current state snapshot without executing |

---

## Condition Code Rules

### Standard word operations (MOV, ADD, SUB/CMP, etc.)

```
result_u = unsigned result (mod 2^16)
result_s = signed result (as int16)

N = (result_u >> 15) & 1
Z = 1 if result_u == 0 else 0
V = 1 if signed overflow else 0
C = 1 if carry-out (unsigned overflow) else 0
```

**Signed overflow** for addition A + B = R:
`V = 1 if (A and B have same sign) and (R has different sign)`

**Signed overflow** for subtraction A − B = R (CMP computes src − dst):
`V = 1 if (A and B have different sign) and (R has different sign from A)`

### Byte operations (MOVB, CMPB, etc.)

Same rules as word but using 8-bit values: N = bit 7, overflow compared over
the range −128..127 / 0..255.

When a byte result is stored to a **register** (mode 0), the byte is
sign-extended to 16 bits before writing (e.g., MOVB #0xFF, R0 → R0 = 0xFFFF).
When stored to **memory**, only the byte is written.

### MOV / MOVB — V always cleared

MOV and MOVB set N and Z from the result but always clear V. C is not
modified by MOV/MOVB.

### CLR / CLRB — all flags to known state

CLR: N=0, Z=1, V=0, C=0. The value stored is 0.

### COM / COMB — V=0, C=1

COM sets N and Z from the complemented result, clears V, and always sets C.

### NEG — C is set iff result ≠ 0

`C = 0 if result == 0 else 1`.  V is set if the result is the most-negative
value (word: 0x8000, byte: 0x80).

### ADC / SBC — carry-in from C flag

ADC: `dst ← dst + C_flag`.  SBC: `dst ← dst − C_flag`.
These are used for multi-precision arithmetic.

### Rotates (ROR / ROL)

ROR word:
```
new_C = bit 0 of dst
result = (dst >> 1) | (old_C << 15)
```
ROL word:
```
new_C = bit 15 of dst
result = ((dst << 1) & 0xFFFF) | old_C
```

### ASR / ASL (arithmetic shifts)

ASR word:
```
C = bit 0 of dst
result = sign_extend(dst >> 1, 16)   # bit 15 is preserved (arithmetic)
N, Z from result; V = N XOR C (after shift)
```

ASL word:
```
C = bit 15 of dst
result = (dst << 1) & 0xFFFF
N, Z from result; V = N XOR C (detects sign change)
```

### TST / TSTB

Sets N and Z from the operand; always clears V and C.

### SUB (double-operand)

```
Encoding: 1110 | src(6) | dst(6)
result = dst − src
```

Sets N, Z, V, C. Note: CMP is `src − dst` (reversed) for historical reasons;
SUB is `dst − src`.

---

## Instruction Encoding Quick Reference

```
Instruction word layout (16 bits, big-endian bit numbering):

15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
 ╔══╦══╦══╦══╦══╦══╦══╦══╦══╦══╦══╦══╦══╦══╦══╦══╗
 ║  op code fields     ║  src mode/reg  ║  dst mode/reg  ║
 ╚══╩══╩══╩══╩══╩══╩══╩══╩══╩══╩══╩══╩══╩══╩══╩══╝

Double-operand word instructions (bits 15-12 = 0001-0110):
  [4-bit opcode][3-bit src mode][3-bit src reg][3-bit dst mode][3-bit dst reg]

Double-operand byte instructions (bits 15-12 = 1001-1101):
  same layout, byte semantics

Single-operand instructions (bits 15-6 = opcode, bits 5-0 = dst):
  [10-bit opcode][3-bit dst mode][3-bit dst reg]

Branch instructions (bits 15-8 = opcode, bits 7-0 = signed offset in words):
  [8-bit opcode][8-bit signed offset]

JSR:  [0][0][0][1][0][0] [reg(3)] [mode(3)] [dst_reg(3)]
RTS:  [0][0][0][0][0][0][1][0][0][0][0] [reg(3)]
SOB:  [0][1][1][1][1][1] [reg(3)] [offset(6)]
HALT: [0][0][0][0][0][0][0][0][0][0][0][0][0][0][0][0]  = 0x0000
NOP:  [0][0][0][0][0][0][0][0][1][0][1][0][0][0][0][0]  = 0x00A0
```

### Key encoding examples

```
MOV R0, R1          = 0001 000 000 000 001 = 0x0201   (nope — see below)
                    ← Actually: 0001 | src=000 000 | dst=000 001
                    = 0001_000_000_000_001 = 0x1001

Wait — let's be precise. PDP-11 words are 16-bit, MSB first:
  Bit 15-12 = opcode = 0001  → MOV
  Bit 11-9  = src mode = 000 → Register direct
  Bit 8-6   = src reg  = 000 → R0
  Bit 5-3   = dst mode = 000 → Register direct
  Bit 2-0   = dst reg  = 001 → R1

  0001 000 000 000 001 = 0x1001  ✓

ADD R2, R3          = 0110 000 010 000 011 = 0x60C3   (let's verify)
  Bit 15-12 = 0110  → ADD
  Bit 11-9  = 000   → src mode = register direct
  Bit 8-6   = 010   → src reg  = R2
  Bit 5-3   = 000   → dst mode = register direct
  Bit 2-0   = 011   → dst reg  = R3
  = 0110 000 010 000 011 = 0x60C3 ✗ — let's recount:
  0110 = 6
  000  = 0
  010  = 2
  000  = 0
  011  = 3
  = 0x6083  ✓  (0110 0000 1000 0011)

MOV #5, R0          = MOV with src=mode2/R7 (immediate), dst=mode0/R0
  0001 010 111 000 000 = 0x15C0, followed by word 0x0005

HALT                = 0x0000

JSR PC, addr        = 0000 1000 111 mode reg
  Link reg = PC = R7 = 111
  = 0000 1000 111 | dst(mode, reg)
  JSR PC, @#0x1000 = 0x09FF followed by 0x1000   (mode=3/R7=absolute)
  JSR PC, label    (PC-relative) = 0x09F7 followed by displacement word
```

The encoding can be tricky; test vectors in the test suite serve as the
authoritative specification for corner cases.

---

## Halting

The simulator halts when it executes a **HALT instruction** (opcode `0x0000`).
The `halted` flag in `PDP11State` is set to `True` and `step()` becomes a
no-op thereafter.

Test programs conventionally end with `HALT` (2 bytes: `\x00\x00`).

---

## Implementation Notes

### Memory representation

Use a `bytearray` internally for efficient byte-addressable memory.
Word reads: `memory[addr] | (memory[addr+1] << 8)` (little-endian).
Word writes: `memory[addr] = value & 0xFF; memory[addr+1] = (value >> 8) & 0xFF`.

### Addressing mode dispatch

The addressing mode evaluator must handle two cases differently:

1. **Mode 0 (register)**: the operand *is* the register value (for reads) or
   the register itself (for writes). No memory address is computed.
2. **Modes 1–7**: an effective address (EA) is computed; the operand is
   `M[EA]` for reads and the store target for writes.

Autoincrement/autodecrement must update the register **after** computing the
EA (for increment) or **before** (for decrement).

For byte instructions, autoincrement/autodecrement step by 1 byte; for word
instructions they step by 2. Exception: when applied to PC (R7) or SP (R6),
the step is always 2 (pointer-sized), even for byte instructions. This is
the actual PDP-11 hardware behaviour and important for `#immediate` to work
with byte instructions.

### PC management

At the start of each `step()`, fetch the 16-bit instruction word at R7 (PC)
and increment PC by 2. Additional words (immediate values, index offsets)
are fetched by mode evaluation which further increments PC.

This means after decoding a two-word instruction (`MOV #5, R0` = opcode word
+ immediate word), PC will have advanced by 4.

### JSR implementation

```python
# JSR reg, dst_ea
old_reg = r[link_reg]
r[SP] -= 2
write_word(r[SP], old_reg)     # push old link reg
r[link_reg] = r[PC]            # link reg ← return address (already advanced)
r[PC] = dst_ea                 # jump
```

For `JSR PC, addr`: old_reg = PC (return address), link_reg = 7 (PC).
Step 1: push old PC (return address) → `-(SP)` ← PC.
Step 2: `PC ← PC` (no-op since link_reg=7 and we already saved PC).
Step 3: `PC ← dst_ea` (jump).

This is equivalent to the classic push-return-address + jump pattern.

### RTS implementation

```python
# RTS reg
r[PC] = r[link_reg]            # return to saved PC in link reg
r[link_reg] = read_word(r[SP]) # pop old link reg from stack
r[SP] += 2
```

For `RTS PC`: PC ← PC (the return address we saved in link register = PC,
which was set by JSR), then PC ← pop. The net effect is `PC ← M[SP]; SP += 2`.

---

## Package Layout

```
code/packages/python/pdp11-simulator/
├── pyproject.toml
├── README.md
├── CHANGELOG.md
├── src/
│   └── pdp11_simulator/
│       ├── __init__.py          # public re-exports
│       ├── state.py             # PDP11State dataclass
│       ├── flags.py             # condition code helpers
│       └── simulator.py         # PDP11Simulator class
└── tests/
    ├── test_protocol.py         # SIM00 protocol compliance
    ├── test_instructions.py     # per-instruction correctness
    ├── test_programs.py         # multi-instruction programs
    └── test_coverage.py         # edge cases for branch coverage
```

---

## Layer Position

| Layer | Package                     | Architecture          | Year |
|-------|-----------------------------|-----------------------|------|
| 07h   | ibm-704-simulator           | IBM 704               | 1954 |
| 07i   | intel-8080-simulator        | Intel 8080            | 1974 |
| 07j   | mos-6502-simulator          | MOS Technology 6502   | 1975 |
| 07k   | z80-simulator               | Zilog Z80             | 1976 |
| 07l   | manchester-baby-simulator   | Manchester Baby (SSEM)| 1948 |
| 07m   | intel-8086-simulator        | Intel 8086            | 1978 |
| 07n   | motorola-68000-simulator    | Motorola 68000        | 1979 |
| 07o   | **pdp11-simulator**         | **DEC PDP-11**        | **1970** |
| 07p   | intel-8051-simulator        | Intel 8051            | 1980 |

The PDP-11 is placed at 07o (after 68000 but note its year is 1970) for
pedagogical reasons: the 68000's ISA was heavily *influenced by* the PDP-11,
so understanding the PDP-11 after the 68000 illuminates the lineage. The
orthogonal ISA, flat memory model, and autoincrement/autodecrement modes the
68000 inherited from the PDP-11 become immediately clear.

---

## Test Verification Checklist

The test suite must verify:

1. **Protocol**: `reset()`, `load()`, `step()`, `execute()`, `get_state()` all
   return valid `PDP11State`.
2. **MOV word and byte**: register-to-register, immediate-to-register, register-
   to-memory, memory-to-register; byte sign extension into registers.
3. **ADD / SUB / CMP**: correct result, N/Z/V/C flags; overflow cases.
4. **BIT / BIC / BIS**: bitwise operations, V always clear.
5. **CLR / COM / NEG / INC / DEC**: single-operand, flags per table above.
6. **ADC / SBC / TST / SWAB**: boundary and carry-in cases.
7. **ASR / ASL / ROR / ROL**: shift and rotate, V = N XOR C, carry propagation.
8. **Branches**: all 15 branch conditions, both taken and not-taken paths.
9. **JMP**: all legal addressing modes (not mode 0).
10. **JSR / RTS**: correct stack manipulation, correct return address.
11. **SOB**: loop-down countdown, stops at zero.
12. **HALT**: simulator halts; subsequent `step()` is no-op.
13. **Addressing modes**: all 8 modes on a double-operand instruction.
14. **PC-relative addressing**: immediate `#n`, absolute `@#addr`.
15. **SP behavior**: push/pop via autodecrement/autoincrement on R6.
16. **Byte vs word** autoincrement step size; SP/PC always step by 2.
17. **Coverage**: ≥ 80% line coverage (target 90%+).
