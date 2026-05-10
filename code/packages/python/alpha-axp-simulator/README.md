# alpha-axp-simulator

Layer 07s in the historical CPU simulator series — **DEC Alpha AXP 21064 (1992)**.

The Alpha AXP 21064 was Digital Equipment Corporation's first 64-bit RISC processor,
introduced in February 1992. Designed by Richard Sites, it achieved 200 MIPS at 200 MHz —
roughly 5× faster than the Intel 486 at the time.

## What makes the Alpha unusual in this series

| Feature | Alpha AXP | Prior simulators |
|---------|-----------|-----------------|
| Word width | 64-bit throughout | 8–32 bit |
| Byte order | **Little-endian** | Big-endian |
| Condition codes | **None** — comparisons write 0/1 to GPRs | Yes (Z, N, C, V) |
| Delay slots | **None** | MIPS has one |
| Register windows | **None** | SPARC has 8 |
| Registers | 32 × 64-bit (r31 = zero) | Varies |
| HALT encoding | All-zeros word (0x00000000) | Special instruction |

## Architecture

- **32 × 64-bit integer registers** (r0–r31; r31 hardwired zero)
- **64 KiB flat address space**, little-endian byte order
- **No condition codes** — compare instructions (CMPEQ, CMPLT, etc.) write 0 or 1 to a GPR
- **No delay slots** — unlike MIPS R2000, branches take effect immediately
- **HALT = `call_pal 0x0000`** = the all-zeros 32-bit word `0x00000000`

## Instruction formats (all 32-bit)

```
Memory:  [op:6][Ra:5][Rb:5][disp16:16]   ea = Rb + sext(disp16)
Branch:  [op:6][Ra:5][disp21:21]          target = (PC+4) + sext(disp21)*4
Operate: [op:6][Ra:5][Rb/lit8:5+i:1][func:7][Rc:5]
Jump:    [0x1A:6][Ra:5][Rb:5][func:2][hint:14]
PALcode: [0x00:6][palcode:26]
```

## Implemented instructions

**Arithmetic (op=0x10):** ADDL, ADDQ, SUBL, SUBQ, MULL, MULQ, CMPEQ, CMPLT, CMPLE, CMPULT, CMPULE, S4ADDL/Q, S8ADDL/Q, S4SUBL/Q, S8SUBL/Q

**Logical (op=0x11):** AND, BIC, BIS, ORNOT, XOR, EQV, CMOVLBS, CMOVLBC, CMOVEQ, CMOVNE, CMOVLT, CMOVGE, CMOVLE, CMOVGT, AMASK, IMPLVER

**Shift & byte (op=0x12):** SLL, SRL, SRA, EXTBL/WL/LL/QL, INSBL/WL/LL/QL, MSKBL/WL/LL/QL, ZAP, ZAPNOT, SEXTB, SEXTW

**Multiply (op=0x13):** MULL, MULQ, UMULH

**Memory:** LDL, LDQ, LDL\_L, LDQ\_L, LDBU, LDWU, STL, STQ, STB, STW

**Branches:** BR, BSR, BEQ, BNE, BLT, BLE, BGT, BGE, BLBC, BLBS

**Jumps:** JMP, JSR, RET, JSR\_COROUTINE

## Usage

```python
from alpha_axp_simulator import AlphaSimulator, AlphaState
import struct

def w32(v: int) -> bytes:
    return struct.pack("<I", v & 0xFFFFFFFF)   # little-endian!

HALT = w32(0x00000000)

# BIS r31, 42, r1  (MOV immediate: r1 = 42)
mov_r1_42 = w32((0x11 << 26) | (31 << 21) | (42 << 13) | (1 << 12) | (0x20 << 5) | 1)

sim = AlphaSimulator()
result = sim.execute(mov_r1_42 + HALT)
print(result.ok)                        # True
print(result.final_state.regs[1])       # 42
print(result.steps)                     # 2
```

## SIM00 protocol

Implements `Simulator[AlphaState]`:

| Method | Description |
|--------|-------------|
| `reset()` | Zero all state; PC=0, nPC=4 |
| `load(data)` | Reset then copy bytes to memory[0..] |
| `step()` → `StepTrace` | Execute one instruction |
| `execute(data, max_steps)` → `ExecutionResult` | Load and run to HALT |
| `get_state()` → `AlphaState` | Frozen snapshot of CPU state |

## Where this fits

```
Layer 07a  Intel 8080 (1974)
Layer 07b  Zilog Z80 (1976)
Layer 07c  MOS 6502 (1975)
Layer 07d  Motorola 68000 (1979)
Layer 07e  PDP-11 (1970)
Layer 07f  CDC 6600 (1964)
Layer 07g  IBM System/360 (1964)
Layer 07h  VAX-11 (1977)
Layer 07i  MIPS R2000 (1985)
Layer 07r  SPARC V8 (1987)
Layer 07s  DEC Alpha AXP 21064 (1992)  ← this package
```
