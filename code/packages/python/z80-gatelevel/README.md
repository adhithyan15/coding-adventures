# coding-adventures-z80-gatelevel

Zilog Z80 gate-level simulator — every ALU operation routes through real logic gate functions.

## What "gate-level" means

In a behavioral simulator, `A = A + B` is computed with Python's `+` operator.
In this gate-level simulator, every arithmetic/logic operation is built from primitive gate functions:

```python
from arithmetic import full_adder, ripple_carry_adder
from logic_gates import AND, OR, XOR, NOT

# ADD A, B routes through 8 full-adder stages:
carry = 0
for i in range(8):
    sum_bit, carry = full_adder(A_bits[i], B_bits[i], carry)
```

No Python integer arithmetic in the ALU path — only gate operations.

## Architecture

```
Z80GateLevelSimulator
├── RegisterFile       ← D flip-flop arrays (Register8, Register16)
│   ├── Main bank:     A, B, C, D, E, H, L, F
│   ├── Alt bank:      A', B', C', D', E', H', L', F'
│   └── Index:         IX, IY
├── Register16         ← PC, SP (16 flip-flop arrays)
├── DecoderZ80         ← Combinational decoder using AND/NOT gates
└── ALU functions      ← add8, sub8, and8, or8, xor8, rlc8, etc.
```

### Gate count for ADD A, B

| Component             | Gate operations |
|-----------------------|-----------------|
| Decode (group10)      | ~6 AND/NOT      |
| 8 × full_adder        | ~40 gates       |
| Overflow XOR          | 1 gate          |
| Zero detection        | ~8 gates        |
| Parity tree           | ~8 gates        |
| **Total**             | **~63 gates**   |

Compare: a behavioral simulator does `A + B` in a single Python bytecode.

## Installation

```bash
pip install coding-adventures-z80-gatelevel
```

## Usage

```python
from z80_gatelevel import Z80GateLevelSimulator

sim = Z80GateLevelSimulator()
result = sim.execute(bytes([
    0x3E, 0x05,  # LD A, 5
    0x06, 0x03,  # LD B, 3
    0x80,        # ADD A, B  ← 8 full-adder stages
    0x76,        # HALT
]))

assert result.final_state.a == 8
assert result.final_state.flag_z is False
assert result.halted is True
```

### Cross-validating with behavioral simulator

The gate-level and behavioral simulators are drop-in compatible:

```python
from z80_gatelevel import Z80GateLevelSimulator
from z80_simulator import Z80Simulator

program = bytes([0x3E, 0x42, 0x76])  # LD A, 66; HALT

gate_sim = Z80GateLevelSimulator()
behav_sim = Z80Simulator()

gate_result = gate_sim.execute(program)
behav_result = behav_sim.execute(program)

# Both produce identical Z80State output
assert gate_result.final_state.a == behav_result.final_state.a
```

## Supported instruction set

- **Unprefixed**: LD r,r', LD r,n, LD rp,nn, ALU A,r, ALU A,n, INC/DEC r/rp,
  PUSH/POP, JP/JR/CALL/RET (conditional and unconditional), DJNZ, RST,
  EX AF AF', EXX, RLCA/RRCA/RLA/RRA, DAA, CPL, SCF, CCF, IN A,(n), OUT (n),A, DI/EI
- **CB-prefix**: RLC/RRC/RL/RR/SLA/SRA/SRL r, BIT/SET/RES b,r
- **ED-prefix**: ADC HL,rp, SBC HL,rp, NEG, LD A,I, LD A,R, LD I,A, LD R,A,
  IM 0/1/2, RETI/RETN, LDI/LDD/LDIR/LDDR, CPI/CPD/CPIR/CPDR, IN r,(C), OUT (C),r
- **DD/FD-prefix**: IX/IY indexed variants

## How it fits in the stack

```
Layer 07: CPU Simulators
  07a  Manchester Baby (1948)  — behavioral
  07b  IBM 704 (1954)          — behavioral
  07c  Intel 8080 (1974)       — behavioral
  07d  Z80 (1976)              — behavioral
  07k1 Intel 8080              — gate-level
  07k2 Z80                     — gate-level (this package)
```

## Running tests

```bash
cd code/packages/python/z80-gatelevel
pip install -e ".[dev]"
pytest -v
```

## Design notes

### Z80 flags differ from Intel 8080

| Flag | Z80              | 8080    |
|------|------------------|---------|
| H    | Same as AC: half-carry or borrow | AC (auxiliary carry) |
| P/V  | Parity (logical) OR Overflow (arithmetic) | P (parity only) |
| N    | 1 after SUB/DEC, 0 after ADD/INC | Not present |

### Subtraction via two's complement

All subtraction is implemented as `A + NOT(B) + 1`:
- The NOT gate chain inverts all 8 bits of B
- The ripple-carry adder adds A + NOT(B) + 1
- The H flag is INVERTED from the adder's half-carry (because we're negating B)
- The C flag is INVERTED from the adder's carry (borrow semantics)

This exactly mirrors how real Z80 silicon works.
