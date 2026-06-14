# coding-adventures-apple-m1-simulator

Layer **07z** in the coding-adventures simulator series — a behavioral simulator for the **Apple M1** (AArch64 + NEON/AdvSIMD, 2020).

## What is the Apple M1?

The Apple M1 (November 2020) was Apple's first ARM-based SoC for Mac, designed in-house (Apple Silicon). It implements ARMv8.4-A — the same AArch64 integer ISA used by smartphones since 2013 — plus full NEON/AdvSIMD floating-point and vector support. It broke x86-64 performance-per-watt records at launch and triggered the Mac transition away from Intel.

Key specifications:
- 5 nm TSMC process; 16 billion transistors
- 4 "Firestorm" high-performance cores + 4 "Icestorm" efficiency cores
- 128-bit NEON/AdvSIMD SIMD units; up to 4 FP/SIMD ops per cycle
- Unified memory architecture (CPU and GPU share DRAM)

## What does this package simulate?

This simulator extends the AArch64 integer base (layer 07v) with:

**Integer base (inherited):**
- All GPR operations: ADD, SUB, AND, ORR, EOR, CLZ, RBIT, REV, etc.
- All branch forms: B, BL, B.cond, BR, BLR, RET, CBZ, CBNZ, TBZ, TBNZ
- Load/store: LDR, STR, LDRB, STRB, LDRH, STRH, LDRSB, LDRSH, LDRSW
- Multiply-accumulate: MADD, MSUB, SMULH, UMULH
- Conditional select: CSEL, CSINC, CSINV, CSNEG
- Move wide: MOVZ, MOVN, MOVK
- Logical immediate (bitmask)

**NEON/FP additions:**
- 32 × 128-bit NEON register file (V0–V31), with D (64-bit) and S (32-bit) views
- Scalar FP arithmetic: FADD, FSUB, FMUL, FDIV, FABS, FNEG, FSQRT (double and single)
- FP register move: FMOV (FP↔FP, GPR↔FP)
- FP compare: FCMP (sets NZCV for FP branch conditions)
- FP precision conversion: FCVT (single↔double), FCVTZS (FP→int, truncate), SCVTF/UCVTF (int→FP)
- FP load/store: LDR/STR for D and S registers
- NEON vector integer: ADD, SUB, MUL per-element (8B/4H/2S/4S/8H/2D arrangements)
- NEON vector FP: FADD, FSUB, FMUL per-element (4S, 2D)
- DUP from GPR (broadcast integer to all vector lanes)
- FMLA (fused multiply-accumulate: Vd = Vd + Vn × Vm)

## Usage

```python
from apple_m1_simulator import AppleM1Simulator
from apple_m1_simulator.simulator import (
    movwide, fp_dp1src, fp_dp2src, fmov_gpr_to_fp_d,
    scvtf, fcvtzs, HALT,
)
import struct

# Compute sqrt(9.0) = 3.0
bits9 = struct.unpack(">Q", struct.pack(">d", 9.0))[0]
program = (
    # Load 9.0 into X0 (as bit pattern), then FMOV to D0
    movwide(1, 0b10, 0, bits9 & 0xFFFF, 0) +
    movwide(1, 0b11, 1, (bits9 >> 16) & 0xFFFF, 0) +
    movwide(1, 0b11, 2, (bits9 >> 32) & 0xFFFF, 0) +
    movwide(1, 0b11, 3, (bits9 >> 48) & 0xFFFF, 0) +
    fmov_gpr_to_fp_d(0, 0) +               # FMOV D0, X0
    fp_dp1src(0b01, 0b000011, 0, 1) +      # FSQRT D1, D0
    HALT
)

sim = AppleM1Simulator()
result = sim.execute(program)
state = result.final_state
print(state.d1)   # → 3.0
```

## Memory model

- 64 KiB flat byte-addressed big-endian memory
- Addresses wrap modulo 65536
- HALT sentinel: `0x00000000` (UDF #0)

## Protocol

Implements the `SIM00` protocol from `simulator_protocol`:

```python
sim.reset()                   # zero all state
sim.load(program: bytes)      # reset + copy program to memory[0x0000]
trace = sim.step()            # execute one instruction → StepTrace
result = sim.execute(program) # run until HALT → ExecutionResult
state = sim.get_state()       # frozen AppleM1State snapshot
```

## State

`AppleM1State` is a frozen dataclass with:
- `pc`, `sp`, `nzcv`, `halted`
- `gpr: tuple[int, ...]` — 32 × 64-bit integer registers
- `vreg: tuple[int, ...]` — 32 × 128-bit NEON registers
- `memory: tuple[int, ...]` — 65536 bytes
- Convenience properties: `x0..x30`, `w0..w5`, `n/z/c/v` flags
- FP properties: `d0..d7` (float), `s0..s7` (float), `d0_bits..d7_bits` (int), `v0..v7` (raw 128-bit)

## In the series

```
07v  AArch64 (2011)      — integer ISA baseline
07x  Intel 8086 (1978)   — x86 16-bit
07z  Apple M1 (2020)     — AArch64 + NEON (this package)
```
