# 07z — Apple M1 (AArch64 + NEON/AdvSIMD) Behavioral Simulator

## Position in the Timeline

| Layer | Chip / ISA           | Year | Key innovation                     |
|-------|----------------------|------|------------------------------------|
| 07v   | AArch64 (ARMv8-A)    | 2011 | Clean 64-bit ISA, fixed-width insns|
| 07x   | AArch64 / Intel 8086 | ...  | ...                                |
| **07z** | **Apple M1**       | **2020** | **First Apple Silicon desktop; AArch64 + NEON/AdvSIMD** |

The Apple M1 (November 2020) was Apple's first ARM-based SoC for Mac.
It implements the ARMv8.4-A instruction set — the same AArch64 integer ISA
as layer 07v, but with full NEON/AdvSIMD (FP and vector) support.

Key facts:
- 5 nm TSMC process; 16 billion transistors
- 4 "Firestorm" high-performance cores + 4 "Icestorm" efficiency cores
- 128-bit NEON/AdvSIMD SIMD units; up to 4 FP/SIMD ops per cycle
- Unified memory architecture (CPU and GPU share the same DRAM)
- Broke x86-64 performance-per-watt records at launch

This simulator implements the AArch64 integer base (same as 07v) plus a
behaviorally correct model of scalar FP and NEON vector operations.

---

## Register File

### General-Purpose Registers (GPR)

| Name    | Index | Description                                 |
|---------|-------|---------------------------------------------|
| X0–X7   | 0–7   | Argument / result registers                 |
| X8      | 8     | Indirect result / syscall number            |
| X9–X15  | 9–15  | Caller-saved temporaries                    |
| X16–X17 | 16–17 | Intra-procedure-call scratch (IP0/IP1)      |
| X18     | 18    | Platform register                           |
| X19–X28 | 19–28 | Callee-saved registers                      |
| X29     | 29    | Frame pointer (FP)                          |
| X30     | 30    | Link register (LR) — written by BL/BLR      |
| XZR     | 31    | Zero register — reads 0, writes discarded   |
| SP      | —     | Stack pointer (separate from GPR file)      |
| PC      | —     | Program counter (not directly accessible)   |

W-register (W0–W30): lower 32 bits of the corresponding X-register.
Writes zero-extend to 64 bits.

### NEON/FP Registers (V0–V31)

| Name    | Width    | Description                                     |
|---------|----------|-------------------------------------------------|
| V0–V31  | 128 bits | NEON vector / FP registers                      |
| D0–D31  | 64 bits  | Lower 64 bits of V0–V31 (double-precision FP)   |
| S0–S31  | 32 bits  | Lower 32 bits of V0–V31 (single-precision FP)   |
| H0–H31  | 16 bits  | Lower 16 bits (not simulated)                   |
| B0–B31  | 8 bits   | Lower 8 bits (not simulated)                    |

Write semantics:
- Writing a D value → zero-extends to 128 bits (upper 64 bits = 0)
- Writing an S value → zero-extends to 128 bits (upper 96 bits = 0)
- Reading D/S reads the low bits of the 128-bit vreg

---

## Condition Flags (NZCV)

Stored as a 4-bit nibble:

| Bit | Name | Meaning                              |
|-----|------|--------------------------------------|
|  3  | N    | Negative (MSB of result)             |
|  2  | Z    | Zero (result == 0)                   |
|  1  | C    | Carry (unsigned overflow)            |
|  0  | V    | Overflow (signed overflow)           |

For FP compare (FCMP), the flags are set as follows:

| Comparison result | N | Z | C | V | NZCV nibble |
|-------------------|---|---|---|---|-------------|
| Equal (a == b)    | 0 | 1 | 1 | 0 | 0b0110      |
| Less (a < b)      | 1 | 0 | 0 | 0 | 0b1000      |
| Greater (a > b)   | 0 | 0 | 1 | 0 | 0b0010      |
| Unordered (NaN)   | 0 | 0 | 1 | 1 | 0b0011      |

---

## Memory Model

- 64 KiB flat byte-addressed big-endian memory
- Addresses wrap modulo 65 536
- FP values stored/loaded big-endian
- Unaligned accesses succeed (wrap by address masking)

---

## HALT Sentinel

`0x00000000` (UDF #0) — permanently undefined in AArch64; used here as the
simulation stop sentinel. When this word is fetched, the simulator halts.

---

## Instruction Set

### AArch64 Integer Base (inherited from 07v)

All of these are implemented identically to layer 07v:

- **Data Processing Immediate**: ADD, SUB, ADDS, SUBS (with 12-bit immediate)
- **Move Wide**: MOVZ, MOVN, MOVK
- **Logical Immediate**: AND, ORR, EOR, ANDS (bitmask immediate encoding)
- **Logical Register**: AND, ORR, EOR, ANDS, BIC, ORN, EON, BICS (shifted reg)
- **Arithmetic Register**: ADD, SUB, ADDS, SUBS (shifted register)
- **Data Processing 2-Source**: UDIV, SDIV, LSLV, LSRV, ASRV, RORV
- **Data Processing 1-Source**: CLZ, RBIT, REV, REV16, REV32
- **3-Source**: MADD, MSUB, SMULH, UMULH
- **Conditional Select**: CSEL, CSINC, CSINV, CSNEG
- **Load/Store Unsigned Offset**: LDR, STR, LDRB, STRB, LDRH, STRH, LDRSB, LDRSH, LDRSW
- **Branches**: B, BL, B.cond, BR, BLR, RET
- **Compare-and-Branch**: CBZ, CBNZ
- **Test-and-Branch**: TBZ, TBNZ
- **SVC**: treated as NOP
- **NOP**: 0xD503201F

### Scalar FP Instructions

#### FP Data Processing (1 source)

Encoding:
```
bits[31:29] = 000 (sf=0, M=0, S=0) or 100 for sf=1 instructions
bits[28:24] = 11110
bits[23:22] = ftype (00=single, 01=double)
bit[21]     = 1
bits[20:15] = opcode
bits[14:10] = 10000
bits[9:5]   = Rn (source FP register)
bits[4:0]   = Rd (dest FP register)
```

| opcode (bits[20:15]) | Instruction | Operation              |
|----------------------|-------------|------------------------|
| 000000               | FMOV Fd,Fn  | Copy FP register       |
| 000001               | FABS Fd,Fn  | Absolute value         |
| 000010               | FNEG Fd,Fn  | Negate                 |
| 000011               | FSQRT Fd,Fn | Square root            |
| 000100               | FCVT        | Convert precision      |

FCVT: if ftype=01 → output=Dd, input=Sn (single→double); if ftype=00 → output=Sd, input=Dn (double→single).

#### FP Data Processing (2 sources: FADD/FSUB/FMUL/FDIV)

Encoding:
```
bits[28:24] = 11110
bits[23:22] = ftype
bit[21]     = 1
bits[20:16] = Rm
bits[15:12] = opcode
bits[11:10] = 10
bits[9:5]   = Rn
bits[4:0]   = Rd
```

| opcode (bits[15:12]) | Instruction |
|----------------------|-------------|
| 0000                 | FMUL        |
| 0001                 | FDIV        |
| 0010                 | FADD        |
| 0011                 | FSUB        |

#### FCMP

Encoding:
```
bits[28:24] = 11110
bits[23:22] = ftype
bit[21]     = 1
bits[20:16] = Rm
bits[15:10] = 001000
bits[9:5]   = Rn
bits[4:3]   = 00
bits[2:0]   = opc (000=FCMP Rn,Rm; 011=FCMP Rn,#0.0)
```

FCMP sets NZCV according to the FP comparison table above.

#### FMOV GPR ↔ FP

```
(GPR→FP double): sf=1, ftype=01, bits[20:16]=00111
(FP→GPR double): sf=1, ftype=01, bits[20:16]=00110
(GPR→FP single): sf=0, ftype=00, bits[20:16]=00111
(FP→GPR single): sf=0, ftype=00, bits[20:16]=00110
bits[28:24] = 11110, bit[21]=1, bits[15:10]=000000
```

#### FCVTZS (FP → integer, truncate toward zero)

```
bits[28:24] = 11110, bit[21]=1
bits[20:19] = 11 (rmode = toward-zero)
bits[18:16] = 000
bits[15:10] = 000000
bit[31] (sf) selects 32/64-bit output
```

Result is clamped to integer range on overflow. NaN → 0.

#### SCVTF / UCVTF (integer → FP)

```
bits[28:24] = 11110, bit[21]=1, bits[15:10]=000000
SCVTF: bits[20:16] = 00010 (signed)
UCVTF: bits[20:16] = 00011 (unsigned)
bit[31] (sf) selects 32/64-bit input; ftype selects output precision
```

### FP Load/Store

Uses the load/store unsigned-offset encoding with V=1:

```
size[31:30] | 111 | V=1 | 01 | opc[23:22] | imm12[21:10] | Rn[9:5] | Rt[4:0]
```

| size | opc | Instruction     | Transfer size |
|------|-----|-----------------|---------------|
| 10   | 00  | STR St,[Xn,#imm*4] | 32-bit     |
| 10   | 01  | LDR St,[Xn,#imm*4] | 32-bit     |
| 11   | 00  | STR Dt,[Xn,#imm*8] | 64-bit     |
| 11   | 01  | LDR Dt,[Xn,#imm*8] | 64-bit     |

FP loads read raw bytes from memory and store in vreg (zero-extended to 128 bits).
FP stores read vreg lower bits and write as bytes to memory.

### NEON Vector Instructions

#### AdvSIMD Three-Register Same

```
0[31] | Q[30] | U[29] | 01110[28:24] | size[23:22] | 1[21] | Rm[20:16] | opcode[15:11] | 1[10] | Rn[9:5] | Rd[4:0]
```

- Q=0 → 64-bit lane width (lower 64 bits only)
- Q=1 → 128-bit (full register)
- size: 00=8-bit, 01=16-bit, 10=32-bit, 11=64-bit elements

| opcode (bits[15:11]) | U | Operation                        |
|----------------------|---|----------------------------------|
| 10000 (0x10)         | 0 | ADD per-element integer          |
| 10000 (0x10)         | 1 | SUB per-element integer          |
| 10011 (0x13)         | 0 | MUL per-element integer (not 64b)|
| 11010 (0x1A), bit29=0| — | FADD per-element FP              |
| 11010 (0x1A), bit29=1| — | FSUB per-element FP              |
| 11011 (0x1B), bit29=1| — | FMUL per-element FP              |

For FADD/FSUB/FMUL: bit[23]=0, sz (bit[22]) selects element FP type (0=single, 1=double).

#### DUP (duplicate from GPR)

Broadcasts a scalar GPR value into all lanes of a vector register.

```
0[31] | Q[30] | 0[29] | 01110[28:24] | imm5[23:19] | 00001[18:14] | 1[13] | 0[12:11] | 1[10] | Rn[9:5] | Rd[4:0]
imm5=10000 → 64-bit D lanes
imm5=01000 → 32-bit S lanes
```

For DUP Vd.2D, Xn: imm5=10000 (bit3=1), Q=1, duplicates Xn into both 64-bit lanes.

#### FMLA (Fused Multiply-Accumulate)

```
0[31] | Q[30] | 0[29] | 01110[28:24] | 0[23] | sz[22] | 1[21] | Rm[20:16] | 11001[15:11] | 1[10] | Rn[9:5] | Rd[4:0]
```

Vd = Vd + Vn × Vm (per-element FP). Q=1 for 128-bit.

---

## SIM00 Protocol Compliance

| Method             | Behaviour                                               |
|--------------------|---------------------------------------------------------|
| `reset()`          | Zero all GPR, vreg, SP, PC, NZCV, memory; halted=False  |
| `load(program)`    | `reset()` then copy bytes to memory[0x0000…]            |
| `step()`           | Fetch-decode-execute one instruction; return StepTrace  |
| `execute(program)` | `load()` then step until HALT or max_steps              |
| `get_state()`      | Return frozen `AppleM1State` snapshot                   |

State type: `AppleM1State` (frozen dataclass) implementing the same interface
as `AArch64State` with the addition of `vreg: tuple[int, ...]` (32 × 128-bit).

---

## IEEE 754 Arithmetic

All scalar FP operations use Python's `struct.pack/unpack` for IEEE 754
compliance. The simulator does not model:
- Rounding modes (always round-to-nearest-even, Python default)
- FP exception flags (FPSR/FPCR)
- Denormal flushing

NaN propagation: if any FP operand is NaN, the result is NaN.
FCVTZS of NaN returns 0 (saturated conversion).

---

## Simplifications vs Real M1

- No exception levels (EL0–EL3)
- No MMU or virtual memory
- No cache effects or memory ordering
- UDIV/SDIV by zero returns 0
- SVC/HVC/SMC → NOP
- Memory barriers (DMB/DSB/ISB) → NOP
- Only NZCV tracked; DAIF and other PSTATE fields ignored
- No FPCR/FPSR
- No half-precision (H registers) or byte FP
- No SVE/SVE2 extensions
- No crypto extensions (AES, SHA2 hardware)
