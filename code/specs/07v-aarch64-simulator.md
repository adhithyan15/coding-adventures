# Layer 07v — AArch64 (2011) Behavioral Simulator

## Overview

AArch64 is the 64-bit execution state introduced by ARMv8-A in 2011.  It
powers every Apple Silicon chip (M1 / M2 / M3 / M4), the AWS Graviton family,
Qualcomm Snapdragon, and effectively every modern smartphone.  As of 2024,
AArch64 is the dominant server and client architecture by shipped unit count.

AArch64 was a clean break from the 32-bit ARM instruction set (ARMv7 / Thumb).
Rather than extending an ageing architecture, ARM Ltd designed a new 64-bit ISA
from scratch while keeping the RISC philosophy: fixed 32-bit instruction width,
load/store memory model, large uniform register file, and simple condition flags.

**Historical significance:**
- Successor to ARMv7 (07b) and conceptual descendant of the ARM1 (07e)
- Introduced in ARMv8-A (2011); first silicon in Apple A7 for iPhone 5s (2013)
- Apple's M1 (2020) displaced x86-64 in MacBook Pro — the fastest laptop chip
- AWS Graviton (2018) put AArch64 in cloud servers at scale
- Android adopted 64-bit AArch64 as mandatory baseline from 2019
- Demonstrates how clean 64-bit design avoids x86-64's legacy complexity

---

## Architecture

### Register File

```
Name       Width   Role
──────────────────────────────────────────────────────────────
X0–X7      64-bit  Argument / result registers
X8         64-bit  Indirect result location / syscall number
X9–X15     64-bit  Caller-saved temporaries
X16–X17    64-bit  Intra-procedure-call scratch (IP0/IP1)
X18        64-bit  Platform register (reserved)
X19–X28    64-bit  Callee-saved registers
X29        64-bit  Frame pointer (FP)
X30        64-bit  Link register (LR) — written by BL/BLR
XZR        64-bit  Zero register — reads 0, writes ignored
SP         64-bit  Stack pointer (aligned to 16 bytes)
PC         64-bit  Program counter (not directly accessible as GPR)

W0–W30     32-bit  Low-word views of X0–X30 (zero-extends on write)
WZR        32-bit  Low-word view of XZR
```

Key properties:
- **XZR / WZR**: the register numbered 31 in most instruction encodings
  acts as the zero register (reads 0; writes are silently discarded).
- **SP**: in some encodings, register 31 denotes SP instead of XZR
  (controlled by the instruction class).
- **32-bit view**: writing W0 zero-extends to fill X0.  Reading W0 returns
  bits [31:0] of X0.  There is no sign-extension on register write.
- **X30 as LR**: BL/BLR save PC+4 to X30.  RET branches to X30.

### Condition Flags

```
Bit  Flag  Meaning
─────────────────────────────────────────────────────────
N    Negative   Result was negative (bit 63 / bit 31 set)
Z    Zero       Result was zero
C    Carry      Unsigned carry-out / borrow-complement
V    Overflow   Signed overflow
```

The four NZCV flags live in the PSTATE register.  Only compare instructions
(CMP, CMN, TST) and the S-suffix variants (ADDS, SUBS, ANDS) update them.

### Program Counter

PC is not a general-purpose register.  It cannot be read or written by normal
arithmetic instructions.  It advances by 4 after each instruction.  Branch
instructions overwrite PC with a computed target address.

### Memory

64 KiB of byte-addressed big-endian memory (indices 0x0000–0xFFFF).
All multi-byte accesses are big-endian.  Addresses wrap modulo MEM\_SIZE.

HALT is the 32-bit word 0x00000000 (UDF #0 — permanently undefined in AArch64;
used here as a sentinel that stops simulation).

---

## Instruction Formats

AArch64 uses several encoding classes.  All instructions are exactly 32 bits.

### Data Processing — Immediate

```
31  30 29 28 23 22 21 10 9    5 4    0
┌──┬──┬──┬──────┬──┬─────────┬───────┬───────┐
│sf│op│S │111000│sh│  imm12  │  Rn   │  Rd   │
└──┴──┴──┴──────┴──┴─────────┴───────┴───────┘

sf = 0 → 32-bit (W regs);  sf = 1 → 64-bit (X regs)
sh = 0 → imm unshifted;    sh = 1 → imm << 12
op/S selects ADD/ADDS/SUB/SUBS
```

### Data Processing — Register (shifted)

```
31  30 29 28 24 23 22 21 20  16 15 10 9   5 4   0
┌──┬──┬──┬──────┬────┬──┬──────┬──────┬──────┬──────┐
│sf│op│S │01011 │shft│ 0│  Rm  │imm6  │  Rn  │  Rd  │
└──┴──┴──┴──────┴────┴──┴──────┴──────┴──────┴──────┘

shft: 00=LSL, 01=LSR, 10=ASR, 11=ROR
```

### Logical — Immediate

```
31  30 29 28 23 22 16 15 10 9   5 4   0
┌──┬────┬──┬──────┬──┬──────┬──────┬──────┬──────┐
│sf│ opc│ 0│100100│N │immr  │imms  │  Rn  │  Rd  │
└──┴────┴──┴──────┴──┴──────┴──────┴──────┴──────┘

opc: 00=AND, 01=ORR, 10=EOR, 11=ANDS
Immediate is a bitmask decoded from N:immr:imms fields.
```

### Move Wide Immediate

```
31  30 29 28 23 22 21 20    5 4    0
┌──┬────┬──────┬────┬────────────┬──────┐
│sf│ opc│100101│ hw │   imm16    │  Rd  │
└──┴────┴──────┴────┴────────────┴──────┘

opc: 00=MOVN, 10=MOVZ, 11=MOVK
hw: shift = hw × 16 (0, 16, 32, or 48 for 64-bit; 0 or 16 for 32-bit)
```

### Load / Store — Unsigned Offset

```
31 30 29 27 26 25 24 23 22 21 10 9   5 4   0
┌────┬─────┬──┬──┬──┬──┬───────────┬──────┬──────┐
│size│ 111 │V │01│opc│ │  imm12    │  Rn  │  Rt  │
└────┴─────┴──┴──┴──┴──┴───────────┴──────┴──────┘

size+opc selects STR/LDR and the data width (byte/halfword/word/dword).
EA = Rn + (imm12 << size)
```

### Unconditional Branch (register)

```
31          25 24 23 21 20 16 15 10 9   5 4   0
┌────────────┬──┬─────┬─────┬──────┬──────┬──────┐
│1101011 0   │ op│ 11111│  0  │00000 │  Rn  │ 00000│
└────────────┴──┴─────┴─────┴──────┴──────┴──────┘

op: 00=BR, 01=BLR, 10=RET (Rn default X30)
```

### Unconditional Branch (immediate)

```
31 30 29 26 25                          0
┌──┬────────┬────────────────────────────┐
│op│   000101  │         imm26            │
└──┴────────┴────────────────────────────┘

op=0 → B; op=1 → BL
Target = PC + SignExtend(imm26 × 4, 64)
```

### Conditional Branch (immediate)

```
31      24 23                         5 4 3       0
┌──────────┬───────────────────────────┬──┬────────┐
│ 01010100 │          imm19            │0 │  cond  │
└──────────┴───────────────────────────┴──┴────────┘

Target = PC + SignExtend(imm19 × 4, 64)
cond: 0000=EQ, 0001=NE, 0010=CS/HS, 0011=CC/LO,
      0100=MI, 0101=PL, 0110=VS, 0111=VC,
      1000=HI, 1001=LS, 1010=GE, 1011=LT,
      1100=GT, 1101=LE, 1110=AL
```

### Compare and Branch (CBZ / CBNZ)

```
31 30  25 24 23                       5 4      0
┌──┬──────┬──┬─────────────────────────┬────────┐
│sf│011010│op│         imm19           │   Rt   │
└──┴──────┴──┴─────────────────────────┴────────┘

op=0 → CBZ (branch if Rt == 0)
op=1 → CBNZ (branch if Rt != 0)
```

### Data Processing — 3 source (MADD / MSUB)

```
31 30 29 28 24 23 21 20  16 15 14  10 9   5 4   0
┌──┬──┬──┬──────┬─────┬──────┬──┬──────┬──────┬──────┐
│sf│ 0 │ 0│11011│ op54│  Rm  │ 0│  Ra  │  Rn  │  Rd  │
└──┴──┴──┴──────┴─────┴──────┴──┴──────┴──────┴──────┘

MADD: Rd = Ra + Rn × Rm
MSUB: Rd = Ra − Rn × Rm
MUL is MADD Ra=XZR.
```

---

## Supported Instructions

### Arithmetic (immediate)

| Mnemonic         | Operation                             | Flags |
|------------------|---------------------------------------|-------|
| ADD Rd, Rn, #imm | Rd = Rn + imm                         | —     |
| ADDS Rd, Rn, #imm| Rd = Rn + imm                         | NZCV  |
| SUB Rd, Rn, #imm | Rd = Rn − imm                         | —     |
| SUBS Rd, Rn, #imm| Rd = Rn − imm                         | NZCV  |
| CMP Rn, #imm     | SUBS XZR, Rn, #imm (discard result)   | NZCV  |
| CMN Rn, #imm     | ADDS XZR, Rn, #imm (discard result)   | NZCV  |

### Arithmetic (register)

| Mnemonic           | Operation                                 | Flags |
|--------------------|-------------------------------------------|-------|
| ADD Rd, Rn, Rm     | Rd = Rn + shift(Rm)                       | —     |
| ADDS Rd, Rn, Rm    | Rd = Rn + shift(Rm)                       | NZCV  |
| SUB Rd, Rn, Rm     | Rd = Rn − shift(Rm)                       | —     |
| SUBS Rd, Rn, Rm    | Rd = Rn − shift(Rm)                       | NZCV  |
| NEG Rd, Rm         | Rd = 0 − Rm (alias for SUB Rd, XZR, Rm)  | —     |
| NEGS Rd, Rm        | Rd = 0 − Rm                               | NZCV  |
| MUL Rd, Rn, Rm     | Rd = (Rn × Rm)[63:0]                      | —     |
| UMULH Rd, Rn, Rm   | Rd = (Rn × Rm)[127:64] (unsigned)         | —     |
| SMULH Rd, Rn, Rm   | Rd = (Rn × Rm)[127:64] (signed)           | —     |
| UDIV Rd, Rn, Rm    | Rd = Rn / Rm (unsigned; 0 if Rm=0)       | —     |
| SDIV Rd, Rn, Rm    | Rd = Rn / Rm (signed; 0 if Rm=0)         | —     |
| MADD Rd,Rn,Rm,Ra   | Rd = Ra + Rn × Rm                         | —     |
| MSUB Rd,Rn,Rm,Ra   | Rd = Ra − Rn × Rm                         | —     |
| CMP Rn, Rm         | SUBS XZR, Rn, shift(Rm)                   | NZCV  |
| CMN Rn, Rm         | ADDS XZR, Rn, shift(Rm)                   | NZCV  |

### Logical

| Mnemonic           | Operation                                | Flags |
|--------------------|------------------------------------------|-------|
| AND Rd, Rn, #bimm  | Rd = Rn & bitmask_imm                    | —     |
| ORR Rd, Rn, #bimm  | Rd = Rn \| bitmask_imm                   | —     |
| EOR Rd, Rn, #bimm  | Rd = Rn ^ bitmask_imm                    | —     |
| ANDS Rd, Rn, #bimm | Rd = Rn & bitmask_imm                    | NZCV  |
| TST Rn, #bimm      | ANDS XZR, Rn, #bitmask_imm              | NZCV  |
| AND Rd, Rn, Rm     | Rd = Rn & shift(Rm)                      | —     |
| ORR Rd, Rn, Rm     | Rd = Rn \| shift(Rm)                     | —     |
| EOR Rd, Rn, Rm     | Rd = Rn ^ shift(Rm)                      | —     |
| ANDS Rd, Rn, Rm    | Rd = Rn & shift(Rm)                      | NZCV  |
| BIC Rd, Rn, Rm     | Rd = Rn & ~shift(Rm)                     | —     |
| ORN Rd, Rn, Rm     | Rd = Rn \| ~shift(Rm)                    | —     |
| EON Rd, Rn, Rm     | Rd = Rn ^ ~shift(Rm)                     | —     |
| BICS Rd, Rn, Rm    | Rd = Rn & ~shift(Rm)                     | NZCV  |
| MVN Rd, Rm         | Rd = ~shift(Rm) (alias ORN Rd, XZR, Rm) | —     |
| TST Rn, Rm         | ANDS XZR, Rn, shift(Rm)                  | NZCV  |

### Shift (immediate — aliases using DATA PROC REG)

| Mnemonic       | Operation               |
|----------------|-------------------------|
| LSL Rd, Rn, #s | Rd = Rn << s            |
| LSR Rd, Rn, #s | Rd = Rn >> s (unsigned) |
| ASR Rd, Rn, #s | Rd = Rn >> s (signed)   |
| ROR Rd, Rn, #s | Rd = rotate right by s  |

### Shift (register)

| Mnemonic       | Operation                   |
|----------------|-----------------------------|
| LSLV Rd, Rn, Rm| Rd = Rn << (Rm mod 64)      |
| LSRV Rd, Rn, Rm| Rd = Rn >> (Rm mod 64)      |
| ASRV Rd, Rn, Rm| Rd = Rn >> (Rm mod 64) sign |
| RORV Rd, Rn, Rm| Rd = ror(Rn, Rm mod 64)     |

### Move

| Mnemonic              | Operation                                   |
|-----------------------|---------------------------------------------|
| MOVZ Rd, #imm, LSL #s | Rd = imm << s (other bits zero)             |
| MOVN Rd, #imm, LSL #s | Rd = ~(imm << s)                            |
| MOVK Rd, #imm, LSL #s | Rd[s+15:s] = imm (other bits unchanged)     |
| MOV Rd, Rm            | Rd = Rm (alias for ORR Rd, XZR, Rm)        |
| MOV Rd, #imm          | alias for MOVZ                              |

### Count / Reverse bits

| Mnemonic     | Operation                        |
|--------------|----------------------------------|
| CLZ Rd, Rn   | Rd = count leading zeros         |
| RBIT Rd, Rn  | Rd = reverse bits                |
| REV Rd, Rn   | Rd = byte-reverse                |
| REV16 Rd, Rn | Rd = byte-reverse within 16-bit halfwords |
| REV32 Rd, Rn | Rd = byte-reverse within 32-bit words (X only) |

### Load / Store

| Mnemonic              | Width | Sign-ext | EA                        |
|-----------------------|-------|----------|---------------------------|
| LDR Xt, [Xn, #imm]   | 64    | N/A      | Xn + imm                 |
| LDR Wt, [Xn, #imm]   | 32    | zero     | Xn + imm                 |
| LDRB Wt, [Xn, #imm]  | 8     | zero     | Xn + imm                 |
| LDRH Wt, [Xn, #imm]  | 16    | zero     | Xn + imm                 |
| LDRSB Xt, [Xn, #imm] | 8     | sign     | Xn + imm                 |
| LDRSH Xt, [Xn, #imm] | 16    | sign     | Xn + imm                 |
| LDRSW Xt, [Xn, #imm] | 32    | sign     | Xn + imm                 |
| STR Xt, [Xn, #imm]   | 64    | N/A      | Xn + imm                 |
| STR Wt, [Xn, #imm]   | 32    | N/A      | Xn + imm                 |
| STRB Wt, [Xn, #imm]  | 8     | N/A      | Xn + imm                 |
| STRH Wt, [Xn, #imm]  | 16    | N/A      | Xn + imm                 |

Pre/post-indexed and register-offset forms are also supported.

### Branch

| Mnemonic     | Operation                                      |
|--------------|------------------------------------------------|
| B #label     | PC = PC + offset                               |
| BL #label    | X30 = PC+4; PC = PC + offset                  |
| BR Xn        | PC = Xn                                        |
| BLR Xn       | X30 = PC+4; PC = Xn                           |
| RET {Xn}     | PC = X30 (or Xn if specified)                 |
| B.cond #label| Branch if condition true                       |
| CBZ Rt, #lbl | Branch to label if Rt == 0                     |
| CBNZ Rt, #lbl| Branch to label if Rt != 0                     |
| TBZ Rt,#b,#l | Branch if bit b of Rt is 0                     |
| TBNZ Rt,#b,#l| Branch if bit b of Rt is 1                     |

### Conditional Select

| Mnemonic              | Operation                            |
|-----------------------|--------------------------------------|
| CSEL Rd, Rn, Rm, cond | Rd = cond ? Rn : Rm                 |
| CSINC Rd, Rn, Rm, cond| Rd = cond ? Rn : Rm+1               |
| CSINV Rd, Rn, Rm, cond| Rd = cond ? Rn : ~Rm                |
| CSNEG Rd, Rn, Rm, cond| Rd = cond ? Rn : -Rm                |
| CSET Rd, cond         | Rd = cond ? 1 : 0                   |
| CSETM Rd, cond        | Rd = cond ? -1 : 0                  |

### System

| Mnemonic    | Operation                                    |
|-------------|----------------------------------------------|
| NOP         | No operation                                 |
| SVC #imm    | Supervisor call (treated as NOP in simulator)|

---

## Condition Evaluation

```python
def condition_holds(cond: int, nzcv: int) -> bool:
    N = (nzcv >> 3) & 1
    Z = (nzcv >> 2) & 1
    C = (nzcv >> 1) & 1
    V = (nzcv >> 0) & 1
    match cond >> 1:  # top 3 bits select the base condition
        case 0: result = Z == 1          # EQ / NE
        case 1: result = C == 1          # CS / CC
        case 2: result = N == 1          # MI / PL
        case 3: result = V == 1          # VS / VC
        case 4: result = C == 1 and Z == 0  # HI / LS
        case 5: result = N == V          # GE / LT
        case 6: result = N == V and Z == 0  # GT / LE
        case 7: result = True            # AL
    if cond & 1 and cond != 15:          # invert for the odd condition codes
        result = not result
    return result
```

---

## NZCV Update Rules

### Arithmetic (ADD / SUB)

```python
def add_with_flags(a: int, b: int, sf: int) -> tuple[int, int]:
    """Returns (result, nzcv).  sf=1 → 64-bit; sf=0 → 32-bit."""
    bits = 64 if sf else 32
    mask = (1 << bits) - 1
    unsigned_sum = (a & mask) + (b & mask)
    result = unsigned_sum & mask
    N = (result >> (bits - 1)) & 1
    Z = 1 if result == 0 else 0
    C = 1 if unsigned_sum > mask else 0
    a_sign = (a >> (bits - 1)) & 1
    b_sign = (b >> (bits - 1)) & 1
    r_sign = N
    V = 1 if (a_sign == b_sign) and (r_sign != a_sign) else 0
    return result, (N << 3) | (Z << 2) | (C << 1) | V

# SUB A, B computes A + (~B) + 1 — borrow-complement carry convention.
```

### Logical (AND / ORR / EOR / BIC / ...)

```python
# N = result[msb]; Z = (result == 0); C = 0; V = 0
```

---

## Bitmask Immediate Decoding

AArch64 logical immediates are encoded as a triple (N, immr, imms).  The
algorithm produces a 64-bit repeating bitmask:

```python
def decode_bitmask(N: int, immr: int, imms: int, sf: int) -> int:
    """
    Decode the logical immediate encoding into a 64-bit integer.
    N=1 forces 64-bit element width even on W-register instructions.
    """
    if N == 1:
        len_ = 6  # element length in bits: 2^len_
    else:
        # Find highest set bit in ~imms & ones(6) with N prepended
        combined = (~imms & 0x3F) | (N << 6)
        len_ = combined.bit_length() - 1
        if len_ == 0:
            raise ValueError("UNDEFINED bitmask immediate")
    esize = 1 << len_         # element size in bits (1, 2, 4, 8, 16, 32, 64)
    S = imms & (esize - 1)    # number of set bits minus 1
    R = immr & (esize - 1)    # right-rotation amount
    # Build a run of (S+1) ones and rotate right by R within esize bits
    welem = (1 << (S + 1)) - 1
    telem = ror(welem, R, esize)
    # Replicate telem to fill 64 bits
    result = 0
    for pos in range(0, 64, esize):
        result |= telem << pos
    return result & 0xFFFF_FFFF_FFFF_FFFF
```

---

## SIM00 Compliance

The simulator implements `Simulator[AArch64State]`:

| Method            | Behaviour                                         |
|-------------------|---------------------------------------------------|
| `reset()`         | Zero all registers, memory, PC, NZCV, halted flag |
| `load(prog)`      | `reset()` then copy bytes to memory[0x0000…]      |
| `step()`          | Fetch 4-byte word at PC, decode, execute; return `StepTrace` |
| `execute(prog, max_steps=100_000)` | `load()` then loop `step()` until HALT or max_steps |
| `get_state()`     | Return a **frozen** `AArch64State` snapshot        |

### AArch64State

```python
@dataclass(frozen=True)
class AArch64State:
    pc:     int              # current instruction address (0 on reset)
    gpr:    tuple[int, ...]  # 32 entries: X0–X30, XZR (index 31 = always 0)
    sp:     int              # stack pointer
    nzcv:   int              # condition flags (4-bit: N Z C V)
    memory: tuple[int, ...]  # MEM_SIZE bytes (65 536)
    halted: bool

    # Conveniences
    @property def x0(self) -> int  ...
    # ... x1 through x30
    @property def n(self)  -> bool  # Negative flag
    @property def z(self)  -> bool  # Zero flag
    @property def c(self)  -> bool  # Carry flag
    @property def v(self)  -> bool  # oVerflow flag
```

---

## Simplifications

1. **Integer only**: FPR/SIMD registers (V0–V31) and floating-point
   instructions are not simulated.
2. **No EL switching**: Exception levels (EL0–EL3) are not modelled.
3. **No MMU**: All addresses are physical; wraps modulo MEM\_SIZE.
4. **OE/flags**: Only S-suffix and compare instructions set NZCV.
5. **UDIV/SDIV by zero** returns 0 (UNDEFINED in the spec; our choice).
6. **No delay slots**: AArch64 has none — branches take effect immediately.
7. **SVC/HVC/SMC**: Treated as NOP.
8. **No barriers**: DMB/DSB/ISB treated as NOP.
9. **PSTATE fields**: only NZCV are tracked; DAIF, SPSel, etc. are ignored.
