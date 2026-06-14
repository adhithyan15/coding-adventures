# coding-adventures-alpha-axp-gatelevel

**DEC Alpha AXP 21064 (1992) gate-level simulator** — Layer 07s2 in the
historical CPU simulator series.

## What is this?

Every arithmetic and logic data-path operation routes through logic gate
primitives from the `logic-gates` package:

- `AND(a, b)`, `OR(a, b)`, `XOR(a, b)`, `NOT(a)` — one call per bit
- `ripple_carry_adder(a_bits, b_bits, carry_in)` — 64 full adders in series

No Python operators (`+`, `-`, `&`, `|`, `^`, `~`, `*`) appear in the
execution path for ALU or register operations.

## Architecture

The DEC Alpha AXP 21064 was DEC's first 64-bit RISC processor (1992):

- **32 GPRs**: r0–r31, each 64-bit. r31 hardwired to 0.
- **No condition codes**: comparisons write 0/1 to a destination register.
- **No delay slots**: branches take effect immediately.
- **Little-endian** memory (64 KiB flat).
- **Fixed 32-bit instruction width**.
- **HALT** = all-zeros word (`0x00000000` = `call_pal 0`).

## Package structure

```
src/alpha_axp_gatelevel/
├── bits.py          — int ↔ bit-list bridge (64-bit operations)
├── alu.py           — 64-bit gate-level ALU (ADDQ/SUBQ/AND/OR/XOR/...)
├── register_file.py — 32 × 64-bit register file stored as bit lists
├── decoder.py       — combinational 32-bit instruction decoder
└── simulator.py     — AlphaAXPGateLevelSimulator (Simulator[AlphaState])
```

## Gate counts (approximate)

| Operation | Gate calls |
|-----------|-----------|
| ADDQ (64-bit) | ~384 (64 full adders × 6 gates each) |
| SUBQ | ~448 (64 NOT + 64 full adders) |
| AND/OR/XOR | 64 |
| MULQ | ~24,576 (64 iterations of ADDQ) |
| UMULH | ~24,576 (64 iterations of ADD_128) |

## Usage

```python
import struct
from alpha_axp_gatelevel import AlphaAXPGateLevelSimulator

def w(word):
    return struct.pack("<I", word)

# BIS r31, #42, r1 (load 42 into r1)
BIS_IMM = (0x11 << 26) | (31 << 21) | (42 << 13) | (1 << 12) | (0x20 << 5) | 1
HALT = 0x00000000

prog = w(BIS_IMM) + w(HALT)

sim = AlphaAXPGateLevelSimulator()
result = sim.execute(prog)
print(result.final_state.regs[1])  # 42
```

## Cross-validation

The gate-level simulator is cross-validated against the behavioral
`alpha_axp_simulator.AlphaSimulator` — they produce identical register
state for all test programs.

## Dependencies

- `coding-adventures-logic-gates` — AND, OR, XOR, NOT gate functions
- `coding-adventures-arithmetic` — ripple_carry_adder
- `coding-adventures-simulator-protocol` — Simulator protocol, ExecutionResult
- `coding-adventures-alpha-axp-simulator` — AlphaState, architecture constants
