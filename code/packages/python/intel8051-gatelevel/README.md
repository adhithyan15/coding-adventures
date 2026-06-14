# Intel 8051 Gate-Level Simulator

A gate-level behavioral simulator for the Intel 8051 (MCS-51) microcontroller, built on top of logic gate primitives. Every ALU operation routes through `AND`, `OR`, `XOR`, `NOT` gates and `ripple_carry_adder` chains — no Python arithmetic operators appear in the data path.

## What is gate-level simulation?

In real hardware, an 8-bit adder is literally 8 full-adder cells wired in series. Each full adder is built from XOR and AND gates. When you run `ADD A, R0` on the 8051, the silicon routes two 8-bit numbers through this cascade of gates.

This simulator does the same in software:

```
ADD A, R0:
  a_bits = int_to_bits(A, 8)     # bridge from Python int to bit array
  b_bits = int_to_bits(R0, 8)    # same
  result_bits, carry = ripple_carry_adder(a_bits, b_bits, 0)
                        # ^-- chains 8 full_adder calls
                        #     each full_adder calls XOR, AND, OR gates
  A = bits_to_int(result_bits)   # bridge back to Python int
```

Every instruction in the data path follows this pattern.

## Architecture

The Intel 8051 uses a **Harvard architecture** with three separate address spaces:

```
CODE MEMORY  [64 KB]  — program instructions, read via PC
IRAM         [256 B]  — internal RAM + Special Function Registers (SFRs)
XDATA        [64 KB]  — external data memory (accessed via MOVX)
```

Key registers (all stored in IRAM or dedicated flip-flops):

| Register | IRAM Address | Reset Value |
|----------|-------------|-------------|
| ACC      | 0xE0        | 0x00        |
| B        | 0xF0        | 0x00        |
| PSW      | 0xD0        | 0x00        |
| SP       | 0x81        | 0x07        |
| DPL      | 0x82        | 0x00        |
| DPH      | 0x83        | 0x00        |
| PC       | (dedicated) | 0x0000      |

PSW bit layout: `CY(7) AC(6) F0(5) RS1(4) RS0(3) OV(2) -(1) P(0)`

## Package structure

```
src/intel8051_gatelevel/
├── bits.py          — int ↔ bit-list bridge (only file allowed to use Python arithmetic)
├── alu.py           — gate-level ALU (ADD, SUBB, ANL, ORL, XRL, rotates, MUL, DIV, DA)
├── register_file.py — IRAM and PC stored as bit arrays (flip-flop simulation)
├── decoder.py       — gate-tree instruction decoder (AND/OR/NOT classification)
└── simulator.py     — Intel8051GateLevelSimulator (Simulator[I8051State] protocol)
```

## Usage

```python
from intel8051_gatelevel import Intel8051GateLevelSimulator

sim = Intel8051GateLevelSimulator()

# Sum 1+2+...+10 = 55
prog = bytes([
    0x74, 0x00,   # MOV A, #0      (sum = 0)
    0x78, 0x0A,   # MOV R0, #10    (counter = 10)
    0x28,         # ADD A, R0      (sum += counter)
    0xD8, 0xFD,   # DJNZ R0, -3
    0xA5,         # HALT
])

result = sim.execute(prog)
print(f"Sum = {result.final_state.acc}")  # Sum = 55
```

## Gate-level guarantee

The following Python operators **never appear** in `alu.py`, `register_file.py`, or the execution path of `simulator.py`:
- Arithmetic: `+`, `-`, `*`, `/`, `//`, `%`
- Bitwise: `&`, `|`, `^`, `~`

Only in `bits.py` (the bridge layer) are these operators used for data conversion.

## Cross-validation

The `test_equivalence.py` test suite runs every program on both:
- `intel8051_simulator.I8051Simulator` (behavioral reference)
- `intel8051_gatelevel.Intel8051GateLevelSimulator` (gate-level)

And verifies identical final state (ACC, IRAM, PC, flags).

## Installation

```bash
pip install coding-adventures-intel8051-gatelevel
```

Or for development:

```bash
uv pip install -e ../logic-gates -e ../arithmetic -e ../simulator-protocol -e ../intel8051-simulator -e ".[dev]"
```

## Running tests

```bash
pytest tests/ -v --cov=intel8051_gatelevel
```
