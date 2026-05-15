# Layer 07x — ARMv7-A / Thumb-2 (2004) Behavioral Simulator

## Overview

ARMv7-A is the 32-bit application profile architecture that powered the
smartphone revolution of the 2000s and 2010s.  Every iPhone up to the 5s, every
Android phone before 2014, the Raspberry Pi 1/2, and countless embedded systems
run ARMv7-A code.  Understanding ARMv7-A is essential because it bridges the gap
between the classical 32-bit RISC designs (ARM1 through ARM9) and the modern
64-bit AArch64 world.

**How it came to be.**  ARM Ltd introduced the Thumb instruction set in 1995
(ARM7TDMI) — a 16-bit-encoded subset of ARM32 that reduced code size by ~30%
for embedded applications.  Over time the 16-bit encoding became a bottleneck:
it could not express the full power of the ARM register file or condition codes.
Thumb-2, introduced with ARMv6T2 and mandated in ARMv7-A (2004), fixed this by
mixing 16-bit and 32-bit Thumb instructions in the same stream.  The CPU detects
the instruction width at decode time by inspecting bits [15:11]: if they are
`0b11100` or higher it is a 32-bit instruction; otherwise it is 16-bit.

**Historical significance:**
- ARM Cortex-A8 (2007) — first ARMv7-A core; basis of the Apple A4 (iPhone 4)
- ARM Cortex-A9 (2008) — basis of the Apple A5 (iPad 2) and Tegra 2
- ARMv7-A Thumb-2 is the target ISA for Android NDK (armeabi-v7a ABI)
- Raspberry Pi 1 and 2 are ARMv6/ARMv7 targets; Pi 3/4 run in 32-bit mode too
- Linux still supports armeabi-v7a as a Tier-2 architecture in 2026
- Critical bridge between 32-bit embedded ARM and 64-bit AArch64

---

## Architecture

### Register File

ARMv7-A has 16 × 32-bit general-purpose registers:

| Index | Name | Role |
|-------|------|------|
| R0    | R0   | Argument / result 1; caller-saved |
| R1    | R1   | Argument / result 2; caller-saved |
| R2    | R2   | Argument 3; caller-saved |
| R3    | R3   | Argument 4; caller-saved |
| R4    | R4   | Variable; callee-saved |
| R5    | R5   | Variable; callee-saved |
| R6    | R6   | Variable; callee-saved |
| R7    | R7   | Variable; callee-saved; Thumb frame pointer convention |
| R8    | R8   | Variable; callee-saved |
| R9    | R9   | Variable; callee-saved (sometimes platform register) |
| R10   | R10  | Variable; callee-saved |
| R11   | R11  | Variable; callee-saved; ARM frame pointer convention |
| R12   | IP   | Intra-procedure scratch; caller-saved |
| R13   | SP   | Stack pointer |
| R14   | LR   | Link register — written by BL/BLX; read by BX LR |
| R15   | PC   | Program counter — reads return current PC + 4 (Thumb) or +8 (ARM) |

**PC alignment note:** In Thumb mode, reading R15/PC returns `current_pc + 4`,
rounded down to 4-byte alignment.  In ARM mode it returns `current_pc + 8`.
This simulator implements Thumb-2 only; PC reads return `pc + 4`.

### Current Program Status Register (CPSR)

The CPSR holds condition flags, execution state bits, and interrupt masks:

```
bit 31  N  Negative   — MSB of last result was 1
bit 30  Z  Zero       — last result was zero
bit 29  C  Carry      — unsigned carry-out (or borrow complement)
bit 28  V  Overflow   — signed overflow
bit  5  T  Thumb      — 1 = Thumb mode, 0 = ARM mode
bits 4:0   M  Mode    — processor mode (not modeled; always 0)
```

The simulator initializes CPSR with T=1 (Thumb mode) and all flags = 0.

### Memory Model

Flat 64 KiB (65 536 bytes) little-endian byte-addressed memory, same as all
other simulators in this series.  Reset zeros the entire array.  RSP (R13)
is initialized to 0xFFF8 on reset.

---

## Instruction Set — Thumb-2

### Instruction Width Detection

Bits [15:11] of the first halfword determine instruction width:

| bits [15:11] | Width |
|--------------|-------|
| 0b111xx (0xE800–0xFFFF but NOT 0b11100xxx) | 32-bit |
| 0b11101xx, 0b11110xx, 0b11111xx | 32-bit |
| Specifically: first halfword bits[15:11] in {0b11101, 0b11110, 0b11111} | 32-bit |
| All others | 16-bit |

Precise rule: a 32-bit Thumb-2 instruction has bits [15:13] == 0b111 AND
bits [12:11] != 0b00.  That is, bits [15:11] ∈ {0b11101, 0b11110, 0b11111}.

### 16-bit Thumb Instructions

#### Data Processing (register)

| Encoding | Mnemonic | Operation |
|----------|----------|-----------|
| 0100 0000 00xx xyyy | AND Rd, Rm | Rd = Rd & Rm; update N,Z |
| 0100 0000 01xx xyyy | EOR Rd, Rm | Rd = Rd ^ Rm; update N,Z |
| 0100 0000 10xx xyyy | LSL Rd, Rs | Rd = Rd << (Rs & 0xFF); update N,Z,C |
| 0100 0000 11xx xyyy | LSR Rd, Rs | Rd = Rd >> (Rs & 0xFF); update N,Z,C |
| 0100 0001 00xx xyyy | ASR Rd, Rs | Rd = Rd >>a (Rs & 0xFF); update N,Z,C |
| 0100 0001 01xx xyyy | ADC Rd, Rm | Rd = Rd + Rm + C; update N,Z,C,V |
| 0100 0001 10xx xyyy | SBC Rd, Rm | Rd = Rd - Rm - 1 + C; update N,Z,C,V |
| 0100 0001 11xx xyyy | ROR Rd, Rs | Rd = ROR(Rd, Rs & 0xFF); update N,Z,C |
| 0100 0010 00xx xyyy | TST Rd, Rm | Set N,Z on Rd & Rm (discard result) |
| 0100 0010 01xx xyyy | RSB Rd, Rm | Rd = 0 - Rd; update N,Z,C,V (NEG) |
| 0100 0010 10xx xyyy | CMP Rd, Rm | Set N,Z,C,V on Rd - Rm |
| 0100 0010 11xx xyyy | CMN Rd, Rm | Set N,Z,C,V on Rd + Rm |
| 0100 0011 00xx xyyy | ORR Rd, Rm | Rd = Rd \| Rm; update N,Z |
| 0100 0011 01xx xyyy | MUL Rd, Rm | Rd = Rd * Rm (low 32); update N,Z |
| 0100 0011 10xx xyyy | BIC Rd, Rm | Rd = Rd & ~Rm; update N,Z |
| 0100 0011 11xx xyyy | MVN Rd, Rm | Rd = ~Rm; update N,Z |

#### Shift Immediate

| Encoding | Mnemonic | Operation |
|----------|----------|-----------|
| 000 00 iii ii mmm ddd | LSL Rd, Rm, #imm5 | Logical shift left |
| 000 01 iii ii mmm ddd | LSR Rd, Rm, #imm5 | Logical shift right |
| 000 10 iii ii mmm ddd | ASR Rd, Rm, #imm5 | Arithmetic shift right |

Update N, Z, C flags.

#### Add/Sub Register

| Encoding | Mnemonic |
|----------|----------|
| 000 11 0 0 nnn mmm ddd | ADD Rd, Rn, Rm |
| 000 11 0 1 nnn mmm ddd | SUB Rd, Rn, Rm |
| 000 11 1 0 nnn iii ddd | ADD Rd, Rn, #imm3 |
| 000 11 1 1 nnn iii ddd | SUB Rd, Rn, #imm3 |

Update N, Z, C, V flags.

#### Move/Compare/Add/Sub Immediate

| Encoding | Mnemonic |
|----------|----------|
| 001 00 ddd iiiiiiii | MOV Rd, #imm8 — update N,Z |
| 001 01 nnn iiiiiiii | CMP Rn, #imm8 — update N,Z,C,V |
| 001 10 ddd iiiiiiii | ADD Rd, #imm8 — update N,Z,C,V |
| 001 11 ddd iiiiiiii | SUB Rd, #imm8 — update N,Z,C,V |

#### Load/Store

| Encoding | Mnemonic |
|----------|----------|
| 0101 000 mmm nnn ttt | STR Rt, [Rn, Rm] |
| 0101 100 mmm nnn ttt | LDR Rt, [Rn, Rm] |
| 0110 0 iiiii nnn ttt | STR Rt, [Rn, #imm5*4] |
| 0110 1 iiiii nnn ttt | LDR Rt, [Rn, #imm5*4] |
| 0111 0 iiiii nnn ttt | STRB Rt, [Rn, #imm5] |
| 0111 1 iiiii nnn ttt | LDRB Rt, [Rn, #imm5] |
| 1000 0 iiiii nnn ttt | STRH Rt, [Rn, #imm5*2] |
| 1000 1 iiiii nnn ttt | LDRH Rt, [Rn, #imm5*2] |
| 1001 0 ttt iiiiiiii | STR Rt, [SP, #imm8*4] |
| 1001 1 ttt iiiiiiii | LDR Rt, [SP, #imm8*4] |
| 1010 0 ttt iiiiiiii | ADR Rt, PC+#imm8*4 |

#### Stack Operations

| Encoding | Mnemonic |
|----------|----------|
| 1011 0 0 0 1 rrrrrrrr | PUSH {reglist} (with/without LR) |
| 1011 1 1 0 1 rrrrrrrr | POP  {reglist} (with/without PC) |
| 1011 0000 0 iiiiiii | ADD SP, #imm7*4 |
| 1011 0000 1 iiiiiii | SUB SP, #imm7*4 |

#### Branch

| Encoding | Mnemonic |
|----------|----------|
| 1101 cccc iiiiiiii | B{cond} #simm8*2 |
| 1110 0 iiiiiiiiiii | B #simm11*2 |
| 0100 0111 0 mmm 000 | BX Rm |
| 0100 0111 1 mmm 000 | BLX Rm |

#### High Register Operations

| Encoding | Mnemonic |
|----------|----------|
| 0100 0100 D N mm ddd | ADD Rd, Rm (high registers) |
| 0100 0101 D N mm ddd | CMP Rd, Rm (high registers) |
| 0100 0110 D N mm ddd | MOV Rd, Rm (high registers) |

### 32-bit Thumb-2 Instructions

#### Branch with Link

| Encoding | Mnemonic |
|----------|----------|
| 1111 0 S imm10 11 J1 1 J2 imm11 | BL #offset |

Where offset = SignExtend(S:I1:I2:imm10:imm11:0, 25), and:
- I1 = NOT(J1 XOR S), I2 = NOT(J2 XOR S)

#### Data Processing (wide immediate)

32-bit encoding `1111 0 i op ...` includes:
- MOV.W Rd, #imm16 (MOVW: opcode T3)
- MOVT  Rd, #imm16 (move to top halfword)
- ADD.W Rd, Rn, #imm12
- SUB.W Rd, Rn, #imm12
- AND.W Rd, Rn, #imm12
- ORR.W Rd, Rn, #imm12
- EOR.W Rd, Rn, #imm12

#### Load/Store (wide)

32-bit T3/T4 forms of LDR, STR with 12-bit unsigned or 8-bit signed offsets.

---

## Barrel Shifter

The ARM barrel shifter is a combinational unit that pre-shifts the second operand
before the ALU operation.  Every data-processing instruction can optionally shift
its register operand:

| Type | Symbol | Description |
|------|--------|-------------|
| LSL  | <<     | Logical Shift Left — fill with 0 |
| LSR  | >>     | Logical Shift Right — fill with 0 |
| ASR  | >>a    | Arithmetic Shift Right — fill with MSB |
| ROR  | ror    | Rotate Right — wraps bits |
| RRX  | rrx    | Rotate Right 1 through carry |

Carry-out from the shifter feeds the C flag for MOV/MVN/AND/ORR/EOR/BIC/TST.

---

## Condition Codes

Every ARM instruction in ARM mode encodes a 4-bit condition field [31:28].
In Thumb-2, only conditional branches and the IT block use explicit conditions.

| Code | Mnemonic | Condition | Flags |
|------|----------|-----------|-------|
| 0000 | EQ | Equal | Z=1 |
| 0001 | NE | Not equal | Z=0 |
| 0010 | CS/HS | Carry set / unsigned ≥ | C=1 |
| 0011 | CC/LO | Carry clear / unsigned < | C=0 |
| 0100 | MI | Minus / negative | N=1 |
| 0101 | PL | Plus / non-negative | N=0 |
| 0110 | VS | Overflow set | V=1 |
| 0111 | VC | Overflow clear | V=0 |
| 1000 | HI | Unsigned higher | C=1 AND Z=0 |
| 1001 | LS | Unsigned lower or same | C=0 OR Z=1 |
| 1010 | GE | Signed ≥ | N=V |
| 1011 | LT | Signed < | N≠V |
| 1100 | GT | Signed > | Z=0 AND N=V |
| 1101 | LE | Signed ≤ | Z=1 OR N≠V |
| 1110 | AL | Always | (any) |

---

## Halt Sentinel

This simulator uses the all-zero 16-bit halfword `0x0000` as a halt sentinel.
In real Thumb, `0x0000` is an undefined instruction.  When the simulator fetches
`0x0000`, it sets `halted = True` and stops execution.

---

## SIM00 Protocol

This simulator implements the same `Simulator[ARMv7AState]` protocol used by all
simulators in this series:

| Method | Description |
|--------|-------------|
| `reset()` | Zero all registers and memory; set SP=0xFFF8, T=1 |
| `load(program)` | Call reset(), copy bytes to memory[0..] |
| `step()` → `StepTrace` | Execute one instruction; return trace |
| `execute(program, max_steps)` → `ARMv7AState` | load + run until halt/max_steps |
| `get_state()` → `ARMv7AState` | Frozen snapshot of current state |
| `set_input_port(port, value)` | Stub (no-op) |
| `get_output_port(port)` → int | Stub (returns 0) |
| `interrupt(vector)` | Stub (no-op) |
| `nmi()` | Stub (no-op) |

---

## Simulation Scope

This simulator implements:
- Thumb-2 decode (16-bit and 32-bit mixed-width)
- 16-bit instructions: shift-immediate, add/sub, move-immediate, ALU (data
  processing register), load/store word/byte/halfword, stack PUSH/POP, branches
  (conditional and unconditional), high-register MOV/ADD/CMP, BX, BLX
- 32-bit instructions: BL (branch-with-link), MOVW, MOVT, ADD.W, SUB.W
- Barrel shifter for shift-immediate instructions
- Condition code evaluation for conditional branches
- SP (R13) used as stack pointer; LR (R14) saved by BL
- CPSR N/Z/C/V flag updates for arithmetic/logic instructions

The simulator does **not** implement:
- ARM (32-bit word-aligned) encoding mode
- IT (If-Then) block execution — the IT opcode itself halts if encountered
- SIMD / NEON / VFP floating-point
- Memory-mapped I/O, cache, TLB
- Privileged modes (Supervisor, IRQ, FIQ, Abort, Undefined, System)
- Coprocessor instructions (MCR, MRC)
- LDRD/STRD double-word loads/stores

---

## Package Layout

```
armv7a-simulator/
├── pyproject.toml
├── BUILD
├── README.md
├── CHANGELOG.md
└── src/
    └── armv7a_simulator/
        ├── __init__.py
        ├── py.typed
        ├── state.py       # ARMv7AState dataclass, constants, helpers
        └── simulator.py   # ARMv7ASimulator + decode/execute logic
tests/
    ├── conftest.py
    ├── test_protocol.py   # SIM00 protocol compliance
    └── test_instructions.py  # Per-instruction correctness
```
