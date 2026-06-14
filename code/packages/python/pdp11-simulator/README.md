# pdp11-simulator — Layer 07o

A Python behavioral simulator for the **DEC PDP-11** (1970) minicomputer, part
of the `coding-adventures` simulator series.

## What is the PDP-11?

The PDP-11 is the computer on which Unix and the C language were created.
Designed at Digital Equipment Corporation in 1970, it introduced the **orthogonal
ISA**: any addressing mode applies to any register in any instruction.

Key features:
- 8 general-purpose 16-bit registers (R0–R7)
- R6 = SP (stack pointer), R7 = PC (program counter)
- 8 addressing modes applicable uniformly to all registers
- 64 KB flat byte-addressed little-endian memory
- Condition codes: N, Z, V, C

## Usage

```python
from pdp11_simulator import PDP11Simulator

sim = PDP11Simulator()

# MOV #42, R0 then HALT
prog = bytes([
    0xC0, 0x15,   # MOV #n, R0
    0x2A, 0x00,   # immediate: 42
    0x00, 0x00,   # HALT
])

result = sim.execute(prog)
assert result.ok
assert result.final_state.r[0] == 42
```

## Layer Position

| Layer | Architecture       | Year |
|-------|--------------------|------|
| 07m   | Intel 8086         | 1978 |
| 07n   | Motorola 68000     | 1979 |
| **07o** | **DEC PDP-11** | **1970** |
| 07p   | Intel 8051         | 1980 |

## Protocol

Implements `Simulator[PDP11State]` from `coding-adventures-simulator-protocol`.
