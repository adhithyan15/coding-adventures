# Layer 07u — PowerPC 601 (1992) Behavioral Simulator

## Overview

The **PowerPC 601** (1992) was the first processor produced under the AIM alliance
(Apple, IBM, Motorola).  It powered the original Power Macintosh line (March 1994),
BeOS developer machines, and early PowerPC workstations.  The 601 is a hybrid:
it implements the new PowerPC ISA but retains backward compatibility with IBM's
older POWER architecture (the 603/604 dropped the POWER legacy ops).

Historical significance:
- **First PowerPC chip**: launched the AIM alliance that kept Apple competitive
  against x86 through 2006 when Apple switched to Intel
- **RISC revolution in consumer hardware**: brought clean load/store RISC to the
  desktop at a time when Intel's x86 still ruled via CISC inertia
- **Big-endian powerhouse**: classic Mac OS, BeOS, and early Linux ran big-endian;
  the ISA later added bi-endian support
- **Scoreboard heritage via POWER**: the 601 executed up to three instructions per
  cycle using a unified pipeline, borrowing scoreboarding ideas that trace back to
  Seymour Cray's CDC 6600 (the previous layer in this series)

Comparison with prior simulators:

| Feature          | CDC 6600 (07t)               | PowerPC 601 (07u)               |
|------------------|------------------------------|---------------------------------|
| Year             | 1964                         | 1992                            |
| Word width       | 60 bits                      | **32 bits**                     |
| GPRs             | 8 × 60-bit X + 8 × 18-bit A/B | **32 × 32-bit GPR**            |
| Special regs     | None (no LR/CTR equiv.)      | **LR, CTR, CR, XER**           |
| Instruction size | 15-bit or 30-bit (variable)  | **32-bit fixed**                |
| Endianness       | Big                          | **Big** (bi-endian optional)    |
| Memory model     | Word-addressed (60-bit words)| **Byte-addressed**              |
| Condition codes  | None (branch tests register) | **CR register (8 × 4-bit)**    |

---

## Architecture

### General-Purpose Registers: GPR0–GPR31

- 32 registers, each **32 bits** wide
- **GPR0 special case**: when GPR0 appears as the `rA` field in instructions that
  compute an effective address (loads, stores, `addi`, `addis`), the value used is
  **0** (not the contents of GPR0).  GPR0 still holds its value for all other
  operations (logical ops, arithmetic ops as source/destination, etc.).
- No register is hardwired to zero like MIPS R0 or Alpha R31; the GPR0 rule applies
  only in effective-address calculations.

### Special-Purpose Registers

#### Link Register (LR)
- 32 bits
- Set to the address of the next instruction by `bl` (branch and link) and `bctrl`
- Used as a branch target by `blr` (branch to link register)
- Read/written via `mfspr` / `mtspr` with SPR=8

#### Count Register (CTR)
- 32 bits
- Used as a decrement counter by `bdnz`-style conditional branches
- Used as a branch target by `bctr` / `bctrl`
- Read/written via `mfspr` / `mtspr` with SPR=9

#### XER (Fixed-Point Exception Register)
- 32 bits
- Bit 0 (MSB): **SO** — Summary Overflow (sticky; set by overflow, never cleared by
  instructions in this simulator)
- Bit 1: **OV** — Overflow (set by arithmetic overflow)
- Bit 2: **CA** — Carry (set by carry out of bit 31 in add/subtract variants)
- Bits 3–31: not simulated
- Read/written via `mfspr` / `mtspr` with SPR=1

#### Condition Register (CR)
- 32 bits
- Divided into 8 four-bit fields: **CR0** (bits 0–3, most significant) through **CR7**
  (bits 28–31, least significant).  PowerPC bit 0 = MSB.
- Within each field, the bit pattern is: `[LT, GT, EQ, SO]`
  - **LT** (Less Than, bit 0 of field): 1 if result < 0 (signed)
  - **GT** (Greater Than, bit 1 of field): 1 if result > 0 (signed)
  - **EQ** (Equal, bit 2 of field): 1 if result == 0
  - **SO** (Summary Overflow, bit 3 of field): copy of XER[SO] at compare time
- **CR0** is updated automatically by `andi.`, `andis.`, and any instruction with
  the `Rc=1` bit set (e.g., `add.`, `and.`).  This simulator implements `Rc=0`
  for all arithmetic/logical instructions unless explicitly noted.
- **Any CRn** field is updated by `cmpw`, `cmplw`, `cmpwi`, `cmplwi`.
- Read/written via `mfcr` / `mtcrf`; individual bits via `mcrxr` (not simulated).

#### CIA (Current Instruction Address)
- The program counter; always a multiple of 4 (instructions are 4-byte aligned)
- Called `cia` in this simulator's state
- Advances by 4 after each non-branch instruction

---

## Memory

- **Byte-addressed flat address space**
- Simulation uses **65 536 bytes (64 KiB)** — sufficient for all test programs
- **Big-endian**: multi-byte values store the most significant byte at the lowest
  address (opposite of x86 little-endian)
- Word (32-bit) accesses must be **4-byte aligned**; halfword (16-bit) must be
  **2-byte aligned**.  This simulator does not raise alignment exceptions — it
  masks the address (`addr & ~3` for words, `addr & ~1` for halfwords).

---

## Instruction Formats

All PowerPC instructions are exactly **32 bits**.  Bit 0 is the most significant.

### I-Form (Branch Unconditional)

```
 0  1  2  3  4  5  6  7  8  9 10 11 12 ... 29 30 31
[     OPCD     ][                LI              ][AA][LK]
```

- **OPCD** (bits 0–5): primary opcode = 18
- **LI** (bits 6–29): 24-bit signed integer; effective branch offset = LI × 4
- **AA** (bit 30): absolute address flag (0 = PC-relative, 1 = absolute)
- **LK** (bit 31): link flag (1 = save CIA+4 to LR before branching)
- `b target` = OPCD=18, AA=0, LK=0
- `bl target` = OPCD=18, AA=0, LK=1

### B-Form (Branch Conditional)

```
 0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 ... 29 30 31
[     OPCD     ][     BO     ][     BI     ][        BD        ][AA][LK]
```

- **OPCD** = 16
- **BO** (bits 6–10): 5-bit branch options field (see below)
- **BI** (bits 11–15): 5-bit CR bit index (0 = CR0.LT, 1 = CR0.GT, 2 = CR0.EQ, ...)
- **BD** (bits 16–29): 14-bit signed integer; offset = BD × 4
- `bc BO, BI, target` — conditional branch

#### BO Field Encoding

| BO (decimal) | BO (binary) | Meaning                                |
|--------------|-------------|----------------------------------------|
| 20           | 10100       | Branch always (unconditional)          |
| 18           | 10010       | Branch if CR[BI] = 1 (don't test CTR) |
| 16           | 10000       | Branch if CR[BI] = 0 (don't test CTR) |
|  4           | 00100       | Decrement CTR; branch if CTR ≠ 0, don't test CR (`bdnz`) |
| 12           | 01100       | Decrement CTR; branch if CTR = 0, don't test CR (`bdz`)  |

Full decoding (BO[0] is the MSB of the 5-bit field = bit 4 of the integer value):
1. If BO[0] = 0: decrement CTR; `ctr_ok = (CTR ≠ 0) XOR BO[1]`
   If BO[0] = 1: `ctr_ok = True`
2. Let `cr_bit = CR[BI]` (bit BI of CR, 0 = MSB).
   If BO[2] = 0: `cond_ok = (cr_bit == BO[3])`
   If BO[2] = 1: `cond_ok = True`
3. Branch if `ctr_ok AND cond_ok`

### D-Form (Immediate / Load / Store)

```
 0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31
[     OPCD     ][      rD     ][      rA     ][                  imm / d                   ]
```

- **rD** (bits 6–10): destination register
- **rA** (bits 11–15): source / base register
- **imm / d** (bits 16–31): 16-bit immediate (sign-extended for arithmetic/memory;
  zero-extended for `andi.`, `ori`, `xori`, `andis.`, `oris`, `cmplwi`)

### X-Form (Register–Register Logic / Shift / Compare)

```
 0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31
[     OPCD     ][      rS     ][      rA     ][      rB     ][            XO             ][Rc]
```

- Primary **OPCD** = 31 for most X-form instructions
- **XO** (bits 21–30): 10-bit secondary opcode

### XO-Form (Integer Arithmetic)

```
 0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31
[     OPCD     ][      rD     ][      rA     ][      rB     ][OE][           XO          ][Rc]
```

- Primary **OPCD** = 31
- **OE** (bit 21): overflow enable (this simulator always treats OE=0)
- **XO** (bits 22–30): 9-bit secondary opcode

### XFX-Form (Move to/from Special Registers)

```
 0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31
[     OPCD     ][      rS     ][  SPR[5:9]  ][  SPR[0:4]  ][            XO             ][--]
```

- **OPCD** = 31
- The 10-bit SPR field is stored split: SPR bits 5–9 at instruction bits 11–15, SPR bits 0–4 at instruction bits 16–20
- **XO** = 339 for `mfspr`, 467 for `mtspr`, 19 for `mfcr`, 144 for `mtcrf`

### XL-Form (Branch via LR / CTR)

```
 0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31
[     OPCD     ][     BO      ][     BI     ][     BH     ][            XO             ][LK]
```

- Primary **OPCD** = 19
- **XO** = 16 for `bclr` (branch to LR), 528 for `bcctr` (branch to CTR)
- `blr` = `bclr 20, 0` (unconditional branch to LR)
- `bctr` = `bcctr 20, 0` (unconditional branch to CTR)
- `bctrl` = `bcctr 20, 0` with LK=1

---

## Instruction Set (Simulated Subset)

### Integer Arithmetic (XO-form, OPCD=31)

| Mnemonic          | XO  | Operation                                   | Notes          |
|-------------------|-----|---------------------------------------------|----------------|
| `add rD,rA,rB`    | 266 | rD = rA + rB                                |                |
| `addc rD,rA,rB`   |  10 | rD = rA + rB; set CA                        | carry out       |
| `adde rD,rA,rB`   | 138 | rD = rA + rB + XER[CA]; set CA             | add extended    |
| `subf rD,rA,rB`   |  40 | rD = rB − rA  ("subtract from")            |                |
| `neg rD,rA`       | 104 | rD = −rA                                    | rB=0           |
| `mullw rD,rA,rB`  | 235 | rD = low32(rA × rB)  (signed)              |                |
| `divw rD,rA,rB`   | 491 | rD = rA ÷ rB  (signed)                     | truncates toward 0 |
| `divwu rD,rA,rB`  | 459 | rD = rA ÷ rB  (unsigned)                   |                |

### Integer Arithmetic Immediate (D-form)

| Mnemonic              | OPCD | Operation                                     | Notes             |
|-----------------------|------|-----------------------------------------------|-------------------|
| `addi rD,rA,SIMM`     |  14  | rD = (rA\|0) + SIMM                          | `li rD,val` = `addi rD,0,val` |
| `addis rD,rA,SIMM`    |  15  | rD = (rA\|0) + (SIMM << 16)                 | `lis rD,val` = `addis rD,0,val` |
| `subfic rD,rA,SIMM`   |   8  | rD = SIMM − rA; set CA                        |                   |

### Logical (X-form, OPCD=31)

| Mnemonic           | XO  | Operation              |
|--------------------|-----|------------------------|
| `and rA,rS,rB`     |  28 | rA = rS & rB           |
| `or rA,rS,rB`      | 444 | rA = rS \| rB          |
| `xor rA,rS,rB`     | 316 | rA = rS ^ rB           |
| `nand rA,rS,rB`    | 476 | rA = ~(rS & rB)        |
| `nor rA,rS,rB`     | 124 | rA = ~(rS \| rB)       |
| `cntlzw rA,rS`     |  26 | rA = count leading zeros of rS (32-bit) |

### Logical Immediate (D-form)

| Mnemonic              | OPCD | Operation                          | Sets CR0? |
|-----------------------|------|------------------------------------|-----------|
| `andi. rA,rS,UIMM`   |  28  | rA = rS & UIMM (zero-extended)    | Yes       |
| `andis. rA,rS,UIMM`  |  29  | rA = rS & (UIMM << 16)            | Yes       |
| `ori rA,rS,UIMM`     |  24  | rA = rS \| UIMM (zero-extended)   | No        |
| `oris rA,rS,UIMM`    |  25  | rA = rS \| (UIMM << 16)           | No        |
| `xori rA,rS,UIMM`    |  26  | rA = rS ^ UIMM (zero-extended)    | No        |

### Shift / Rotate (X-form, OPCD=31)

| Mnemonic            | XO  | Operation                                     |
|---------------------|-----|-----------------------------------------------|
| `slw rA,rS,rB`      |  24 | rA = rS << n; 0 if n ≥ 32  (n = rB & 0x3F)  |
| `srw rA,rS,rB`      | 536 | rA = rS >> n; 0 if n ≥ 32  (logical shift)   |
| `sraw rA,rS,rB`     | 792 | rA = signed(rS) >> n; sets CA               |
| `srawi rA,rS,SH`    | 824 | rA = signed(rS) >> SH; sets CA (SH in rB field) |

For `sraw`/`srawi`, XER[CA] is set if the operand is negative and any 1-bits are
shifted out; otherwise CA is cleared.

### Compare (X-form, OPCD=31)

| Mnemonic              | XO | Operation                         |
|-----------------------|----|-----------------------------------|
| `cmpw crfD,rA,rB`     |  0 | Signed compare; set CR field crfD |
| `cmplw crfD,rA,rB`    | 32 | Unsigned compare; set CR crfD     |

The `crfD` field occupies bits 6–8 of the instruction (the upper 3 bits of the
rS/rD position); bits 9–10 are 0.

### Compare Immediate (D-form)

| Mnemonic               | OPCD | Operation                          |
|------------------------|------|------------------------------------|
| `cmpwi crfD,rA,SIMM`  |  11  | Signed compare immediate          |
| `cmplwi crfD,rA,UIMM` |  10  | Unsigned compare immediate        |

### Load (D-form)

| Mnemonic           | OPCD | Operation                              | Update rA? |
|--------------------|------|----------------------------------------|------------|
| `lwz rD,d(rA)`    |  32  | rD = MEM[EA:EA+3] (4 bytes)           | No         |
| `lwzu rD,d(rA)`   |  33  | rD = MEM[EA:EA+3]; rA = EA            | Yes        |
| `lbz rD,d(rA)`    |  34  | rD = zero_extend(MEM[EA])             | No         |
| `lbzu rD,d(rA)`   |  35  | rD = zero_extend(MEM[EA]); rA = EA   | Yes        |
| `lhz rD,d(rA)`    |  40  | rD = zero_extend(MEM[EA:EA+1])       | No         |
| `lha rD,d(rA)`    |  42  | rD = sign_extend(MEM[EA:EA+1])       | No         |

EA = (rA == 0 ? 0 : GPR[rA]) + sign_extend(d)

### Store (D-form)

| Mnemonic           | OPCD | Operation                              | Update rA? |
|--------------------|------|----------------------------------------|------------|
| `stw rS,d(rA)`    |  36  | MEM[EA:EA+3] = rS                     | No         |
| `stwu rS,d(rA)`   |  37  | MEM[EA:EA+3] = rS; rA = EA            | Yes        |
| `stb rS,d(rA)`    |  38  | MEM[EA] = rS[24:31] (low byte)        | No         |
| `stbu rS,d(rA)`   |  39  | MEM[EA] = rS[24:31]; rA = EA          | Yes        |
| `sth rS,d(rA)`    |  44  | MEM[EA:EA+1] = rS[16:31] (low half)  | No         |

### Branch (I-form, OPCD=18)

| Mnemonic    | LK | Operation                     |
|-------------|-----|-------------------------------|
| `b target`  | 0   | CIA += sign_extend(LI) × 4   |
| `bl target` | 1   | LR = CIA+4; CIA += LI × 4    |

### Branch Conditional (B-form, OPCD=16)

| Mnemonic          | BO | BI | Operation                     |
|-------------------|----|----|-------------------------------|
| `blt target`      | 18 | 0  | Branch if CR0.LT = 1         |
| `bge target`      | 16 | 0  | Branch if CR0.LT = 0         |
| `bgt target`      | 18 | 1  | Branch if CR0.GT = 1         |
| `ble target`      | 16 | 1  | Branch if CR0.GT = 0         |
| `beq target`      | 18 | 2  | Branch if CR0.EQ = 1         |
| `bne target`      | 16 | 2  | Branch if CR0.EQ = 0         |
| `bdnz target`     |  4 | 0  | Decrement CTR; branch if CTR ≠ 0 |
| `bdz target`      | 12 | 0  | Decrement CTR; branch if CTR = 0  |

### Branch via LR / CTR (XL-form, OPCD=19)

| Mnemonic  | XO  | LK | Operation               |
|-----------|-----|----|-------------------------|
| `blr`     | 16  | 0  | CIA = LR (branch to LR)|
| `bctrl`   | 528 | 1  | LR = CIA+4; CIA = CTR  |
| `bctr`    | 528 | 0  | CIA = CTR              |

### Move to/from Special Registers (XFX-form, OPCD=31)

| Mnemonic       | XO  | SPR | Operation         |
|----------------|-----|-----|-------------------|
| `mfspr rD,XER` | 339 |   1 | rD = XER          |
| `mfspr rD,LR`  | 339 |   8 | rD = LR           |
| `mfspr rD,CTR` | 339 |   9 | rD = CTR          |
| `mtspr XER,rS` | 467 |   1 | XER = rS          |
| `mtspr LR,rS`  | 467 |   8 | LR = rS           |
| `mtspr CTR,rS` | 467 |   9 | CTR = rS          |
| `mfcr rD`      |  19 | n/a | rD = CR           |
| `mtcrf FXM,rS` | 144 | n/a | Update CR fields selected by FXM mask |

`mtcrf FXM, rS` updates each nibble of CR where the corresponding bit in the 8-bit
FXM mask is 1.  FXM=0xFF updates all of CR; FXM=0x80 updates only CR0.

### HALT

- Instruction word `0x00000000` — the all-zeros 32-bit word
- Not a valid PowerPC instruction; the simulator halts when it fetches it
- Exported as `HALT: bytes = b"\x00\x00\x00\x00"`

---

## Encoding Helpers (exported from the package)

```python
HALT: bytes                              # b"\x00\x00\x00\x00"

# SPR numbers
SPR_XER: int = 1
SPR_LR:  int = 8
SPR_CTR: int = 9

# BO values for common branch patterns
BO_ALWAYS: int = 20
BO_TRUE:   int = 18   # branch if CR[BI] = 1
BO_FALSE:  int = 16   # branch if CR[BI] = 0
BO_BDNZ:   int = 4    # decrement CTR, branch if CTR ≠ 0
BO_BDZ:    int = 12   # decrement CTR, branch if CTR = 0

# CR0 bit indices
BI_LT: int = 0   # CR0.LT
BI_GT: int = 1   # CR0.GT
BI_EQ: int = 2   # CR0.EQ
BI_SO: int = 3   # CR0.SO

# Instruction encoding helpers
def i_form(opcode, byte_offset, AA=0, LK=0) -> bytes   # I-form
def b_form(opcode, BO, BI, byte_offset, AA=0, LK=0) -> bytes  # B-form
def d_form(opcode, rD, rA, imm) -> bytes               # D-form
def x_form(opcode, rS, rA, rB, xo, rc=0) -> bytes      # X-form
def xo_form(opcode, rD, rA, rB, oe, xo, rc=0) -> bytes # XO-form
def xfx_form(opcode, rS, spr, xo) -> bytes             # XFX-form
def xl_form(opcode, BO, BI, BH, xo, lk=0) -> bytes     # XL-form
```

---

## State Snapshot

```python
@dataclass(frozen=True)
class PowerPC601State:
    cia:    int               # current instruction address (PC), 32-bit
    gpr:    tuple[int, ...]   # 32 × 32-bit general-purpose registers
    lr:     int               # link register, 32-bit
    ctr:    int               # count register, 32-bit
    xer:    int               # XER: [SO, OV, CA, ...], 32-bit
    cr:     int               # condition register, 32-bit (8 × 4-bit nibbles)
    memory: tuple[int, ...]   # 65536 bytes (each int is 0–255)
    halted: bool
```

Convenience properties: `r0` through `r31` alias into `gpr`.

---

## SIM00 Protocol Compliance

The simulator implements `Simulator[PowerPC601State]`:

| Method              | Behaviour                                                    |
|---------------------|--------------------------------------------------------------|
| `reset()`           | Zeroes all registers, memory, and halted flag; CIA = 0      |
| `load(program)`     | Calls `reset()`, then copies `program` bytes into memory[0] |
| `step()`            | Fetches one 32-bit instruction at CIA; executes it; returns `StepTrace` |
| `execute(program)`  | Calls `load(program)`, then steps until halted or `max_steps` exceeded |
| `get_state()`       | Returns an immutable frozen `PowerPC601State` snapshot       |

---

## Simplifications vs Real Hardware

1. **No floating-point** — FPR0–FPR31 are not simulated; the FPU is outside scope.
2. **No MMU / address translation** — all addresses are physical.
3. **No hardware exceptions** — undefined instructions halt the simulator; no
   interrupt vectors, no SRESET.
4. **No OE (overflow enable)** — XER[SO/OV] are never set by arithmetic in this
   simulator (simulating OE=0 for all ops).
5. **No Rc (record condition)** — arithmetic instructions don't set CR0 (simulating
   Rc=0); only compare instructions and `andi.`/`andis.` set CR fields.
6. **No memory-mapped I/O** — no peripheral simulation.
7. **No POWER legacy instructions** — only the clean PowerPC ISA subset.
8. **Alignment is relaxed** — misaligned loads/stores succeed (address masked to
   alignment boundary instead of faulting).
9. **64-bit memory model** — the real 601 has a 32-bit address bus; the simulator's
   65 536-byte address space is a behavioral subset.
