# mips-r2000-gatelevel

MIPS R2000 (1985) gate-level simulator — Layer 07q2 of the computing stack.

Every arithmetic and logic ALU operation routes through logic gate primitives:
- `AND`, `OR`, `XOR`, `NOT` from the `logic_gates` package
- `ripple_carry_adder` from the `arithmetic` package

## What is gate-level simulation?

A gate-level simulator models the actual hardware data path rather than just
the behavior. Instead of computing `a + b` using Python's `+` operator, the
gate-level simulator:

1. Converts `a` and `b` to bit lists (LSB-first)
2. Runs those bits through 32 full adders (each made of XOR, AND, OR gates)
3. Converts the result bit list back to an integer

This is behaviorally equivalent to the `mips_r2000_simulator` package but
explicitly models the gate-level computation at every step.

## Architecture

```
simulator.py          ← top-level Simulator[MIPSState] implementation
    ↓ uses
decoder.py            ← bit-slice instruction field extraction
alu.py                ← 32-bit gate-level ALU
register_file.py      ← 32 GPRs + HI/LO/PC stored as bit arrays
bits.py               ← int ↔ bit-list bridge (add_32bit, shifts, etc.)
    ↓ uses
logic_gates           ← AND, OR, XOR, NOT
arithmetic            ← ripple_carry_adder
```

## Instruction Set

Implements the full MIPS R2000 instruction set:

**R-type**: SLL, SRL, SRA, SLLV, SRLV, SRAV, JR, JALR, SYSCALL (HALT),
BREAK, MFHI, MTHI, MFLO, MTLO, MULT, MULTU, DIV, DIVU, ADD, ADDU, SUB,
SUBU, AND, OR, XOR, NOR, SLT, SLTU

**I-type**: BEQ, BNE, BLEZ, BGTZ, ADDI, ADDIU, SLTI, SLTIU, ANDI, ORI,
XORI, LUI, LB, LH, LWL, LW, LBU, LHU, LWR, SB, SH, SWL, SW, SWR

**J-type**: J, JAL

**REGIMM**: BLTZ, BGEZ, BLTZAL, BGEZAL

## Usage

```python
import struct
from mips_r2000_gatelevel import MIPSR2000GateLevelSimulator

def w(word: int) -> bytes:
    return struct.pack(">I", word)

# ADDIU $v0, $zero, 42; SYSCALL
prog = w(0x24020000 | 42) + w(0x0000000C)

sim = MIPSR2000GateLevelSimulator()
result = sim.execute(prog)
state = result.final_state
print(f"$v0 = {state.regs[2]}")  # 42
```

## ALU operations

```python
from mips_r2000_gatelevel.alu import add32, sub32, and32, mult32, divu32

r = add32(0x7FFFFFFF, 1)
print(r.result, r.overflow)  # 0x80000000, 1 (signed overflow!)

hi, lo = mult32(1000, 1000)
print(lo)  # 1000000

q, rem = divu32(100, 7)
print(q, rem)  # 14, 2
```

## Key properties

- **R0 hardwired zero**: writes to `$zero` are silently discarded
- **Big-endian memory**: 64 KB flat address space
- **No delay slots**: branches take effect immediately
- **Overflow detection**: ADD/ADDI/SUB raise `ValueError` on signed overflow
- **Gate-level data path**: every operation routes through actual gate functions

## Package structure

```
src/mips_r2000_gatelevel/
├── __init__.py          exports MIPSR2000GateLevelSimulator
├── bits.py              int ↔ bit-list bridge
├── alu.py               32-bit gate-level ALU (ALUResult32)
├── register_file.py     RegisterFile32 (bit-array storage)
├── decoder.py           decode_instruction()
└── simulator.py         MIPSR2000GateLevelSimulator
```

## Dependencies

- `coding-adventures-logic-gates` — AND, OR, XOR, NOT gates
- `coding-adventures-arithmetic` — ripple_carry_adder
- `coding-adventures-simulator-protocol` — Simulator[T] protocol
- `coding-adventures-mips-r2000-simulator` — MIPSState, MIPSSimulator
