# AArch64 Simulator — Layer 07v

A behavioral simulator for the AArch64 (ARMv8-A, 2011) integer instruction set.

## Overview

AArch64 is the 64-bit execution state introduced by ARM Ltd in 2011. It powers:
- Apple Silicon (M1/M2/M3/M4) — the dominant laptop/desktop chip since 2020
- AWS Graviton — putting AArch64 in cloud servers at scale
- Qualcomm Snapdragon — every modern Android phone
- Apple A-series — every iPhone since 2013

This simulator implements the full integer ISA as a behavioral (functional)
model following the SIM00 protocol.

## Architecture

- **31 × 64-bit GPRs** (X0–X30) + XZR (always-zero) + SP + PC
- **NZCV condition flags** updated only by S-suffix and compare instructions
- **64 KiB big-endian memory** at addresses 0x0000–0xFFFF
- **Fixed 32-bit instruction width** — all instructions are exactly 4 bytes

## Supported Instructions

- **Data Processing Immediate**: ADD/SUB/ADDS/SUBS (with optional 12-bit shift)
- **Data Processing Register**: ADD/SUB/ADDS/SUBS with shift (LSL/LSR/ASR/ROR)
- **Logical Immediate**: AND/ORR/EOR/ANDS with bitmask immediates
- **Logical Register**: AND/ORR/EOR/ANDS/BIC/ORN/EON/BICS (shifted)
- **Move Wide**: MOVZ/MOVN/MOVK with 16-bit immediate and hw shift
- **Load/Store Unsigned Offset**: LDR/STR/LDRB/LDRH/LDRSB/LDRSH/LDRSW/STRB/STRH
- **Branches**: B/BL (immediate), BR/BLR/RET (register), B.cond, CBZ/CBNZ, TBZ/TBNZ
- **Conditional Select**: CSEL/CSINC/CSINV/CSNEG
- **Multiply/Divide**: MADD/MSUB/MUL (alias), UMULH/SMULH, UDIV/SDIV
- **Shift by register**: LSLV/LSRV/ASRV/RORV
- **Bit operations**: CLZ, RBIT, REV, REV16, REV32
- **System**: NOP, SVC (treated as NOP)

## Usage

```python
from aarch64_simulator import (
    AArch64Simulator, HALT,
    dp_imm, movwide, branch_imm, COND_NE, branch_cond,
)

sim = AArch64Simulator()

# Sum 1..5
prog = (
    movwide(1, 0b10, 0, 5, 0)       # MOVZ X0, #5  (counter)
    + movwide(1, 0b10, 0, 0, 1)     # MOVZ X1, #0  (sum)
    + dp_imm(1, 0, 0, 1, 0, 1, 1)  # [loop] ADD X1, X1, #1  — wait, simpler with ADDS
    # ...
    + HALT
)
result = sim.execute(prog)
print(f"Steps: {result.steps}, halted: {result.halted}")
print(f"X0={result.final_state.x0}, X1={result.final_state.x1}")
```

## SIM00 Protocol

The simulator implements `Simulator[AArch64State]`:

```python
sim = AArch64Simulator()
sim.load(program_bytes)    # reset + copy program to memory
trace = sim.step()          # execute one instruction
state = sim.get_state()     # frozen AArch64State snapshot
result = sim.execute(prog)  # load + run until HALT
```

## Instruction Encoding Helpers

```python
from aarch64_simulator import dp_imm, movwide, ldst_uoff, branch_imm

# ADD X1, X0, #42
instr = dp_imm(sf=1, op=0, S=0, imm12=42, sh=0, Rn=0, Rd=1)

# MOVZ X0, #0x1234
instr = movwide(sf=1, opc=0b10, hw=0, imm16=0x1234, Rd=0)

# STR X1, [X0]
instr = ldst_uoff(size=3, V=0, opc=0b00, imm12=0, Rn=0, Rt=1)

# B #+8 (branch forward 2 instructions)
instr = branch_imm(op=0, imm26=2)
```

## Historical Context

Layer 07v in the coding-adventures architecture layer stack:
- 07b: ARM (32-bit ARMv7)
- 07u: PowerPC 601 (1992)
- **07v: AArch64 (2011)** ← you are here
- AArch64 is a clean 64-bit redesign, not an extension of 32-bit ARM
- First chip: Apple A7 in iPhone 5s (2013)
- Desktop breakthrough: Apple M1 (2020)
