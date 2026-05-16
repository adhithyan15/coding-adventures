# motorola68k-gatelevel

A gate-level behavioral simulator for the Motorola 68000 (1979) CPU.

Every ALU data-path operation (ADD, SUB, AND, OR, XOR, NOT, shifts, rotates)
routes through logic gate primitives from the `logic-gates` and `arithmetic`
packages.  No Python integer arithmetic is used in the critical ALU path;
all arithmetic is performed by ripple-carry adder chains operating on bit arrays.

## How it fits in the stack

```
Layer 07n2  motorola68k-gatelevel   ← this package
Layer 07n   motorola-68000-simulator (behavioral — for cross-validation)
Layer 05    logic-gates, arithmetic  (gate primitives and adders)
Layer 04    simulator-protocol       (Simulator[T] protocol, ExecutionResult)
```

## Usage

```python
from motorola68k_gatelevel.simulator import Motorola68kGateLevelSimulator

sim = Motorola68kGateLevelSimulator()

# Simple program: D0 = 5 + 3 = 8
prog = bytes([
    0x70, 0x05,              # MOVEQ #5, D0
    0x72, 0x03,              # MOVEQ #3, D1
    0xD0, 0x81,              # ADD.L D1, D0
    0x4E, 0x72, 0x27, 0x00, # STOP #0x2700
])

result = sim.execute(prog)
print(result.final_state.d0)  # 8
print(result.steps)           # 4
```

## Gate-Level Design

### Bit Representation

All register values and ALU intermediates are stored as lists of bits (LSB at
index 0), matching the `logic-gates` package convention.

### Addition: Ripple-Carry Adder Chain

```
bit 0: full_adder(a[0], b[0], carry_in)   → (sum[0], carry[0])
bit 1: full_adder(a[1], b[1], carry[0])   → (sum[1], carry[1])
...
bit 31: full_adder(a[31], b[31], carry[30]) → (sum[31], carry_out)
```

### Subtraction: Two's Complement

```
SUB A, B  =  A + NOT(B) + 1
           = A + [NOT gate per bit] + 1
```

### Overflow Detection

```
OF = XOR(carry into MSB, carry out of MSB)
```

### Logical Operations

```
AND.L D0, D1  →  32 AND gates in parallel
OR.L  D0, D1  →  32 OR  gates in parallel
XOR.L D0, D1  →  32 XOR gates in parallel
NOT.L D0      →  32 NOT gates in parallel
```

## Architecture Summary

- **Data registers:** D0–D7 (32-bit, byte/word/long ops)
- **Address registers:** A0–A6, A7=SSP (32-bit, no byte access)
- **PC:** 32-bit (24-bit address bus)
- **Status Register:** X N Z V C flags + interrupt mask + supervisor bit
- **Memory:** 16 MB flat big-endian, word-aligned for word/long
- **Transistors:** ~68,000 (on real hardware)

## Instruction Set

MOVE, MOVEA, MOVEQ, MOVEM, ADD, ADDA, ADDI, ADDQ, ADDX, SUB, SUBA, SUBI,
SUBQ, SUBX, AND, ANDI, OR, ORI, EOR, EORI, CMP, CMPA, CMPI, CMPM, NOT, NEG,
NEGX, CLR, TST, MULS, MULU, DIVS, DIVU, BRA, BSR, Bcc (all 16), DBcc, Scc,
JMP, JSR, RTS, RTR, RTE, NOP, RESET, STOP, ILLEGAL, BTST, BCHG, BCLR, BSET,
ASL, ASR, LSL, LSR, ROL, ROR, ROXL, ROXR, SWAP, EXT, PEA, LEA, LINK, UNLK,
TRAP, CHK, ABCD, SBCD, NBCD, EXG.

## Installation

```
pip install coding-adventures-motorola68k-gatelevel
```

## Testing

```
pytest tests/ -v
```

Cross-validation against the behavioral simulator:

```
pytest tests/test_equivalence.py -v
```
