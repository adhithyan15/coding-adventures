# fpga

FPGA (Field-Programmable Gate Array) abstraction — LUTs, slices, CLBs, routing fabric, and I/O blocks.

## What is this?

This package models the architecture of an FPGA, built from the ground up on top of the logic-gates and block-ram packages:

1. **LUT (Look-Up Table)** — the atom of programmable logic. Stores a truth table in SRAM, uses a MUX tree to evaluate it. A K-input LUT can implement ANY boolean function of K variables.
2. **Slice** — 2 LUTs + 2 D flip-flops + output MUXes + carry chain. The basic compute unit.
3. **CLB (Configurable Logic Block)** — 2 slices with carry chain propagation between them.
4. **SwitchMatrix** — programmable routing crossbar connecting CLBs through the fabric.
5. **IOBlock** — bidirectional I/O pad with input/output/tri-state modes.

## The Key Insight

**A truth table is a program.**

Any boolean function of K variables has a truth table with 2^K entries. A LUT stores that truth table in SRAM and uses a MUX tree to look up the output. To "reprogram" the hardware, you just load a different truth table.

## How it fits in the stack

```
logic-gates (NOT, AND, OR, MUX, decoder, tri-state)
    │
    ├── block-ram (SRAM cells, arrays, dual-port RAM)
    │
    └── fpga ← YOU ARE HERE
            ├── LUT (SRAM + MUX tree)
            ├── Slice (2 LUTs + 2 FFs + carry)
            ├── CLB (2 slices)
            ├── SwitchMatrix (routing)
            └── IOBlock (external I/O)
```

## Usage

```python
from fpga import LUT, Slice, CLB, SwitchMatrix, IOBlock, IOMode

# === Configure a LUT as a 2-input AND gate ===
and_table = [0] * 16
and_table[3] = 1  # I0=1, I1=1 → output 1
lut = LUT(k=4, truth_table=and_table)
lut.evaluate([1, 1, 0, 0])  # → 1
lut.evaluate([1, 0, 0, 0])  # → 0

# === Reprogram the same LUT as XOR ===
xor_table = [0] * 16
xor_table[1] = 1; xor_table[2] = 1
lut.configure(xor_table)
lut.evaluate([1, 1, 0, 0])  # → 0 (XOR)

# === Use a CLB for 2-bit addition ===
clb = CLB(lut_inputs=4)
clb.slice0.configure(xor_table, and_table, carry_enabled=True)
clb.slice1.configure(xor_table, and_table, carry_enabled=True)
out = clb.evaluate([1,0,0,0], [1,0,0,0], [0,1,0,0], [0,1,0,0], clock=0)

# === Route signals between components ===
sm = SwitchMatrix({"clb_out", "north", "south", "east", "west"})
sm.connect("clb_out", "east")
sm.route({"clb_out": 1})  # → {"east": 1}
```

## Installation

```bash
pip install coding-adventures-fpga
```

## Development

```bash
uv venv && uv pip install -e ".[dev]"
pytest
ruff check src/ tests/
```
