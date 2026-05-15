# armv7a-simulator

ARMv7-A / Thumb-2 (2004) behavioral simulator — Layer 07x in the coding-adventures
simulator series.

## What it is

A pure-Python behavioral simulator for the ARMv7-A architecture running in
Thumb-2 mode.  Thumb-2 is the variable-width instruction encoding introduced in
ARMv7-A that mixes 16-bit and 32-bit instructions in the same stream — the
dominant instruction set for Android and iOS applications before 64-bit AArch64.

## Architecture overview

- **16 × 32-bit registers** (R0–R15): R13=SP, R14=LR, R15=PC
- **CPSR** (Current Program Status Register): N/Z/C/V flags + T bit (Thumb=1)
- **Little-endian** 64 KiB flat memory
- **Thumb-2**: 16-bit and 32-bit instructions, auto-detected at decode time
- **Barrel shifter**: LSL, LSR, ASR, ROR, RRX on shift-immediate instructions
- **Condition codes**: 14 conditions (EQ, NE, CS/HS, CC/LO, MI, PL, VS, VC,
  HI, LS, GE, LT, GT, LE, AL)
- **Halt sentinel**: 16-bit halfword `0x0000`

## Where it fits

| Layer | Architecture | Year | Package |
|-------|-------------|------|---------|
| 07u   | PowerPC 601 | 1993 | powerpc601-simulator |
| 07v   | AArch64 (ARMv8-A) | 2011 | aarch64-simulator |
| 07w   | x86-64 (AMD64) | 2003 | x86-64-simulator |
| **07x** | **ARMv7-A / Thumb-2** | **2004** | **armv7a-simulator** |
| 07y   | RISC-V RV64I | 2010s | riscv64-simulator |

## Usage

```python
from armv7a_simulator import ARMv7ASimulator

sim = ARMv7ASimulator()

# Execute a small Thumb-2 program:
# MOV R0, #42  (0x2A20 little-endian: bytes 0x2A, 0x20)
# halt         (0x0000)
state = sim.execute(bytes([0x2A, 0x20, 0x00, 0x00]))
print(state.r0)    # 42
print(state.z)     # False (42 ≠ 0)
print(state.n)     # False (42 is positive)

# Step-by-step execution:
sim.load(bytes([0x2A, 0x20, 0x00, 0x00]))
trace = sim.step()
print(trace.pc_before, trace.pc_after, trace.halted)  # 0  2  False
```

## Instruction support

### 16-bit Thumb

- **Shift immediate**: LSL, LSR, ASR with imm5
- **Add/subtract**: ADD/SUB register and immediate (imm3, imm8)
- **Move/compare**: MOV, CMP, ADD, SUB with imm8
- **Data processing**: AND, EOR, LSL, LSR, ASR, ADC, SBC, ROR, TST, NEG, CMP,
  CMN, ORR, MUL, BIC, MVN (register forms)
- **High register**: MOV, ADD, CMP for R8–R15
- **Load/store**: LDR/STR word, LDRB/STRB byte, LDRH/STRH halfword (imm and
  register offsets), SP-relative, PC-relative (ADR)
- **Stack**: PUSH (with optional LR), POP (with optional PC)
- **Stack adjust**: ADD SP, #imm7×4 and SUB SP, #imm7×4
- **Branch**: B (conditional, 14 conditions), B (unconditional), BX, BLX
- **Multiple**: LDM, STM

### 32-bit Thumb-2

- **BL**: Branch-and-link (T1 encoding, full ±16 MB range)
- **MOVW**: 16-bit immediate into low halfword
- **MOVT**: 16-bit immediate into high halfword
- **DP wide**: ADD.W, SUB.W, AND.W, ORR.W, EOR.W, ADC.W, RSB.W
- **LDR.W / STR.W**: 12-bit offset word/halfword/byte

## SIM00 protocol

Implements the standard simulator protocol:

```python
sim.reset()                    # zero everything, SP=0xFFF8, T=1
sim.load(program_bytes)        # reset + copy bytes to memory[0..]
state = sim.get_state()        # frozen ARMv7AState snapshot
trace = sim.step()             # execute one instruction → StepTrace
state = sim.execute(prog, max_steps=100_000)
```

## Running tests

```bash
uv venv
uv pip install -e ../simulator-protocol -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```
