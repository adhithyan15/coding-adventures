# F01 — FPGA (Field-Programmable Gate Array)

## Overview

An FPGA is a chip full of logic gates, memory, and wires — but unlike a CPU or GPU where the circuits are permanently etched in silicon, an FPGA's circuits are **programmable**. You upload a configuration file (called a **bitstream**) and the chip reconfigures itself to implement whatever digital circuit you described. Seconds later, you can upload a different bitstream and the same chip becomes a completely different circuit.

This is the "programmable hardware" layer of the computing stack. Where our existing packages model fixed-function circuits (an adder is always an adder, an ALU always performs the same operations), the FPGA package lets you define circuits at runtime through configuration.

**Why FPGAs matter:**
- **Prototyping** — test a chip design on an FPGA before spending millions on ASIC fabrication
- **Acceleration** — FPGAs can implement custom hardware accelerators for specific workloads (video encoding, network packet processing, ML inference) that outperform general-purpose CPUs
- **Reconfigurability** — the same hardware can be repurposed for different tasks by loading different bitstreams
- **Education** — understanding FPGAs teaches you how ALL digital circuits work, because you must build them from scratch

**The key insight:** a Look-Up Table (LUT) storing a truth table is functionally identical to a logic gate — but which gate it implements is determined by the truth table contents, not by its physical structure. A 4-input LUT loaded with `[0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0]` is an AND gate. Reload it with `[0,1,1,0,1,0,0,1,1,0,0,1,0,1,1,0]` and it becomes an XOR gate. Same silicon, different function. **A truth table is a program.**

## Layer Position

```
Logic Gates → Combinational (MUX, decoder) → Block RAM → [YOU ARE HERE]
                                                          ↑
                                              Clock ──────┘
                                              Arithmetic ──┘ (carry chains)
```

**Input from:**
- logic-gates: primitive gates (for building LUT internals), sequential logic (flip-flops in CLBs)
- logic-gates/combinational: MUX (core of LUT selection), decoder (address decoding)
- block-ram: SRAM arrays (for configuration memory and Block RAM tiles)
- clock: clock distribution to all sequential elements
- arithmetic: carry chain logic for fast addition within CLBs

**Output to:** User designs — any digital circuit described by a configuration/bitstream.

## Concepts

### What makes hardware "programmable"?

Consider a 2-input AND gate built from transistors. It always computes AND. The function is physically wired into the silicon — you cannot change it without fabricating a new chip.

Now consider this alternative: instead of hardwiring the AND truth table into transistors, what if we **stored** the truth table in a small memory (4 SRAM cells for a 2-input function) and used a MUX to look up the answer?

```
Traditional AND gate:          Programmable "gate" (2-input LUT):

  a ──┐                           Truth table in SRAM:
      │D──── output                  Address  Value
  b ──┘                              00       0     ← AND(0,0)
                                     01       0     ← AND(0,1)
  Fixed forever.                     10       0     ← AND(1,0)
                                     11       1     ← AND(1,1)

                                   a ──┐
                                       ├── address → [SRAM] → MUX → output
                                   b ──┘

                                   Reprogram SRAM → different function!
```

To change the 2-input LUT from AND to OR, we just rewrite the SRAM:

```
  Address  AND   OR    XOR   NAND
  00       0     0     0     1
  01       0     1     1     1
  10       0     1     1     1
  11       1     1     0     0
```

Same silicon, four different functions. This is the core idea behind FPGAs.

### Look-Up Table (LUT) — the atom of programmable logic

A K-input LUT can implement **any** boolean function of K variables. It has:
- K input signals
- 2^K SRAM cells storing the truth table
- A 2^K-to-1 MUX tree that selects the output based on the inputs

```
4-Input LUT (the most common in modern FPGAs):

                  SRAM (16 cells)
                  ┌───────────┐
  Configuration   │ Cell  0   │──┐
  (bitstream      │ Cell  1   │──┤
   writes these)  │ Cell  2   │──┤    ┌─────────────┐
                  │ Cell  3   │──┼────┤             │
                  │ Cell  4   │──┤    │   16-to-1   │
                  │ Cell  5   │──┤    │     MUX     ├──── Output (F)
                  │ Cell  6   │──┤    │    Tree     │
                  │ Cell  7   │──┤    │             │
                  │ Cell  8   │──┤    │             │
                  │ Cell  9   │──┤    └──┬──┬──┬──┬─┘
                  │ Cell 10   │──┤       │  │  │  │
                  │ Cell 11   │──┤       I0 I1 I2 I3
                  │ Cell 12   │──┤
                  │ Cell 13   │──┤       (4 input signals
                  │ Cell 14   │──┤        used as MUX
                  │ Cell 15   │──┘        select lines)
                  └───────────┘

  I3 I2 I1 I0 = 0000 → Output = Cell[0]
  I3 I2 I1 I0 = 0001 → Output = Cell[1]
  ...
  I3 I2 I1 I0 = 1111 → Output = Cell[15]
```

The MUX tree is built from 2:1 MUXes (from our logic-gates combinational module):

```
16-to-1 MUX tree for a 4-input LUT:

  Level 1 (I0 selects):      Level 2 (I1 selects):    Level 3 (I2):   Level 4 (I3):
  Cell[0]  ─┐                     ┌─────┐                ┌───┐
  Cell[1]  ─┤─ MUX → ────────────┤     │                │   │
  Cell[2]  ─┐                     │ MUX ├── ─────────────┤   │
  Cell[3]  ─┤─ MUX → ────────────┤     │                │MUX├── ──┐
  Cell[4]  ─┐                     └─────┘                │   │     │
  Cell[5]  ─┤─ MUX → ────────────┐                      │   │     │   ┌───┐
  Cell[6]  ─┐                     │ MUX ├── ─────────────┤   │     │   │   │
  Cell[7]  ─┤─ MUX → ────────────┘                      └───┘     ├───┤MUX├── F
  Cell[8]  ─┐                     ┌─────┐                ┌───┐     │   │   │
  Cell[9]  ─┤─ MUX → ────────────┤     │                │   │     │   └───┘
  Cell[10] ─┐                     │ MUX ├── ─────────────┤   │     │
  Cell[11] ─┤─ MUX → ────────────┤     │                │MUX├── ──┘
  Cell[12] ─┐                     └─────┘                │   │
  Cell[13] ─┤─ MUX → ────────────┐                      │   │
  Cell[14] ─┐                     │ MUX ├── ─────────────┤   │
  Cell[15] ─┤─ MUX → ────────────┘                      └───┘
             ↑                       ↑                      ↑           ↑
            I0                      I1                     I2          I3
```

**LUT size tradeoffs:**

| K (inputs) | SRAM cells | Can implement | Used in |
|------------|-----------|---------------|---------|
| 2 | 4 | Any 2-variable function | (too small for modern use) |
| 3 | 8 | Any 3-variable function | Early FPGAs (Xilinx XC2000, 1985) |
| 4 | 16 | Any 4-variable function | Common (Xilinx Spartan, Altera Cyclone) |
| 5 | 32 | Any 5-variable function | Xilinx Virtex-5 |
| 6 | 64 | Any 6-variable function | Modern FPGAs (Xilinx UltraScale, Intel Agilex) |

Larger LUTs can implement more complex functions in a single block but waste
SRAM when the function is simple. FPGAs typically use 6-input LUTs that can
also be configured as two independent 5-input LUTs sharing inputs.

### Configurable Logic Block (CLB) — the building block

A CLB groups multiple LUTs with flip-flops, carry chains, and local MUXes
into a reusable tile. This is the repeated unit that fills most of the FPGA fabric.

A typical CLB (simplified Xilinx-style):

```
┌──────────────────────────────────────────────────────────────────────┐
│                         CLB (Configurable Logic Block)               │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐    │
│  │                    Slice 0                                    │    │
│  │                                                               │    │
│  │  Inputs ────→ [LUT A (4-in)] ──┬──→ [MUX] ──→ Output A      │    │
│  │                                │       ↑                      │    │
│  │                                │    sel (config)              │    │
│  │                                │                              │    │
│  │                                └──→ [D-FF] ──→ Output A_reg  │    │
│  │                                       ↑                       │    │
│  │                                      CLK                      │    │
│  │                                                               │    │
│  │  Inputs ────→ [LUT B (4-in)] ──┬──→ [MUX] ──→ Output B      │    │
│  │                                │       ↑                      │    │
│  │                                └──→ [D-FF] ──→ Output B_reg  │    │
│  │                                                               │    │
│  │           Carry In ──→ [Carry Chain] ──→ Carry Out            │    │
│  └──────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐    │
│  │                    Slice 1 (same structure)                   │    │
│  └──────────────────────────────────────────────────────────────┘    │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

Each CLB contains:
- **2 Slices**, each with:
  - **2 LUTs** (4 or 6 inputs each) — the combinational logic
  - **2 D flip-flops** — for registered (synchronous) outputs
  - **2 output MUXes** — select between LUT output (combinational) or flip-flop output (registered)
  - **Carry chain** — fast carry propagation for arithmetic (avoids slow routing)
- **Local routing MUXes** — connect LUTs within the CLB to each other

The output MUX is crucial: it lets each LUT output be either:
1. **Combinational** — the raw LUT output, available immediately
2. **Registered** — the LUT output captured by a flip-flop on the clock edge, available one cycle later

This is how you build both combinational circuits (like an adder) and sequential circuits (like a state machine) from the same hardware.

**The carry chain** deserves special attention. When building an N-bit adder, the carry must ripple from bit 0 to bit N-1. If the carry had to travel through the general routing fabric, it would be slow. Instead, FPGAs provide a dedicated carry chain — a hardwired fast path between adjacent CLBs that propagates carry in a fraction of the time.

```
CLB[0]                    CLB[1]                    CLB[2]
┌──────────┐              ┌──────────┐              ┌──────────┐
│ LUT: bit 0│              │ LUT: bit 2│              │ LUT: bit 4│
│ LUT: bit 1│              │ LUT: bit 3│              │ LUT: bit 5│
│     ↓     │              │     ↓     │              │     ↓     │
│ Carry Out ├──(fast)──────┤ Carry Out ├──(fast)──────┤ Carry Out ├──→
└──────────┘              └──────────┘              └──────────┘
```

### Routing Fabric — the programmable interconnect

The routing fabric is what makes an FPGA truly flexible. It connects CLBs to each
other, to Block RAM, and to I/O blocks. Without it, each CLB would be an isolated
island.

The fabric consists of:

1. **Wire segments** — horizontal and vertical tracks running between CLBs
2. **Switch matrices** — programmable crossbar switches at the intersections of wire segments. SRAM bits control which connections are made.
3. **Connection boxes** — connect CLB inputs/outputs to nearby wire segments

```
           Wire Segments (horizontal)
         ════════════════════════════════════
              │          │          │
  ┌───────┐  │  ┌────┐  │  ┌────┐  │  ┌───────┐
  │  CLB  ├──┤──┤ SM ├──┤──┤ SM ├──┤──┤  CLB  │
  │ (0,0) │  │  └────┘  │  └────┘  │  │ (0,1) │
  └───┬───┘  │     │    │     │    │  └───┬───┘
      │      │     │    │     │    │      │
  ════│══════│═════│════│═════│════│══════│════  ← Wire Segments (vertical)
      │      │     │    │     │    │      │
  ┌───┴───┐  │  ┌──┴─┐  │  ┌──┴─┐  │  ┌───┴───┐
  │  CLB  ├──┤──┤ SM ├──┤──┤ SM ├──┤──┤  CLB  │
  │ (1,0) │  │  └────┘  │  └────┘  │  │ (1,1) │
  └───────┘  │          │          │  └───────┘
         ════════════════════════════════════

  SM = Switch Matrix (programmable crossbar)
```

**Modeling the routing fabric as a directed graph:**

This is where our existing `directed-graph` library comes in. The routing fabric is
naturally a graph:

- **Nodes**: CLB output pins, CLB input pins, BRAM ports, I/O pins, wire segment endpoints
- **Edges**: active connections through switch matrices and connection boxes

When the FPGA is configured, the bitstream determines which switch matrix connections
are active. This defines the graph edges. Signal propagation follows the graph edges.

```python
from directed_graph import DirectedGraph

fabric = DirectedGraph()
# CLB(0,0) LUT_A output connects to CLB(0,1) LUT_B input 2
fabric.add_edge("clb_0_0.lut_a.out", "clb_0_1.lut_b.in2")
# CLB(0,1) LUT_B output connects to CLB(1,1) LUT_A input 0
fabric.add_edge("clb_0_1.lut_b.out", "clb_1_1.lut_a.in0")
```

The graph enables:
- **Connectivity validation** — is every CLB input driven by exactly one source?
- **Combinational loop detection** — `has_cycle()` on the combinational-only subgraph
- **Critical path analysis** — longest path through the graph (determines max clock frequency)
- **Topological ordering** — `topological_sort()` gives a valid signal evaluation order

### Switch Matrix — the crossbar

A switch matrix is a programmable crossbar that connects wire segments at routing
intersections. Each crossing point has a **pass transistor** (modeled as an AND gate)
controlled by an SRAM configuration bit.

```
4×4 Switch Matrix:

  North[0] ─── ──┬──── ──┬──── ──┬──── ── East[0]
                  │       │       │
  North[1] ─── ──┼──── ──┼──── ──┼──── ── East[1]
                  │       │       │
  North[2] ─── ──┼──── ──┼──── ──┼──── ── East[2]
                  │       │       │
  North[3] ─── ──┼──── ──┼──── ──┼──── ── East[3]
                  │       │       │
            South[0] South[1] South[2] South[3]

  Each crossing point (┼) is controlled by an SRAM bit:
    SRAM = 1 → connection made (signal passes through)
    SRAM = 0 → no connection (wires are isolated)
```

A switch matrix with W wires on each side has up to W² crossing points, but
typically only a subset are connectable (to save area). Real FPGAs use
**Wilton** or **Universal** switch box topologies that guarantee routability
with fewer switches.

### I/O Blocks — connecting to the outside world

I/O blocks sit at the edges of the FPGA fabric, connecting internal signals to
physical package pins. Each I/O block can be configured as:

- **Input** — external signal drives internal logic (with optional input flip-flop for synchronization)
- **Output** — internal signal drives the pin (with optional output flip-flop)
- **Bidirectional** — tri-state buffer controls direction (using the tri-state buffer from logic-gates)

```
┌───────────────────────────────────────┐
│              I/O Block                │
│                                       │
│  Internal ──→ [Output FF] ──→ [Tri-state] ──→ Pin
│  Signal                         ↑
│                              OE (output enable)
│
│  Internal ←── [Input FF] ←─────────────── Pin
│  Signal
│                                       │
│  Direction: SRAM config bit           │
└───────────────────────────────────────┘
```

### Configuration Memory and Bitstream

Every programmable element in the FPGA — every LUT truth table entry, every switch
matrix connection, every I/O direction, every MUX select — is controlled by an
SRAM cell. The collection of all these SRAM cells is the **configuration memory**.

The **bitstream** is a binary file that contains the values for every configuration
SRAM cell. Loading a bitstream programs the FPGA.

```
Bitstream structure (simplified):

┌──────────────────────────────────────────────┐
│ Header                                        │
│   Device ID, bitstream length, checksum       │
├──────────────────────────────────────────────┤
│ CLB Configuration                             │
│   For each CLB:                               │
│     LUT A truth table (16 bits for 4-LUT)     │
│     LUT B truth table (16 bits)               │
│     FF enable bits (2 bits)                   │
│     Output MUX select bits (2 bits)           │
│     Carry chain config (2 bits)               │
├──────────────────────────────────────────────┤
│ Routing Configuration                         │
│   For each switch matrix:                     │
│     Connection bits (one per crossing point)  │
│   For each connection box:                    │
│     Input/output pin connection bits          │
├──────────────────────────────────────────────┤
│ BRAM Contents                                 │
│   Initial values for each Block RAM tile      │
├──────────────────────────────────────────────┤
│ I/O Configuration                             │
│   For each I/O block:                         │
│     Direction (input/output/bidirectional)     │
│     Pull-up/pull-down enable                  │
│     Slew rate, drive strength                 │
└──────────────────────────────────────────────┘
```

In our simulation, we use a **JSON configuration format** instead of a raw binary
bitstream. This is more readable and debuggable while being semantically equivalent.

### Place and Route — from design to physical layout

In real FPGA toolchains, the designer writes HDL (Verilog/VHDL), which is
synthesized into a netlist of LUTs and flip-flops, then:

1. **Place** — assign each LUT/FF to a specific CLB location on the chip
2. **Route** — find paths through the switch matrices to connect them

Our simulator skips synthesis (the user directly specifies LUT truth tables in
the JSON config) but does implement a simple placer and router:

- **Placement**: assign logical blocks to physical CLB coordinates
- **Routing**: use the directed graph to find paths between connected blocks, configuring switch matrices along the way

### Timing Model

Every signal path through the FPGA has a propagation delay:
- **LUT delay** — time for inputs to propagate through the MUX tree (~0.3 ns in modern FPGAs)
- **Routing delay** — time for signal to travel through switch matrices and wire segments (~0.1-2 ns depending on distance)
- **Setup time** — time a flip-flop input must be stable before the clock edge

The **critical path** is the longest combinational delay between any two flip-flops
(or between an input and a flip-flop). It determines the maximum clock frequency:

```
Max frequency = 1 / (longest_path_delay + setup_time)
```

Our simulator computes delays by summing LUT and routing delays along each path
in the routing graph.

## JSON Configuration Format

Instead of a binary bitstream, designs are specified in JSON:

```json
{
  "device": {
    "name": "CinchFPGA-Mini",
    "rows": 4,
    "cols": 4,
    "lut_inputs": 4,
    "bram_tiles": 2,
    "bram_bits": 18432,
    "io_pins": 16
  },

  "clbs": {
    "clb_0_0": {
      "lut_a": {
        "truth_table": [0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0],
        "comment": "4-input AND gate"
      },
      "lut_b": {
        "truth_table": [0,1,1,0,1,0,0,1,1,0,0,1,0,1,1,0],
        "comment": "4-input XOR gate"
      },
      "ff_a": { "enabled": true, "reset_value": 0 },
      "ff_b": { "enabled": false },
      "output_mux_a": "registered",
      "output_mux_b": "combinational",
      "carry_in": "external"
    }
  },

  "routing": [
    { "from": "io_pin_0", "to": "clb_0_0.lut_a.in0" },
    { "from": "io_pin_1", "to": "clb_0_0.lut_a.in1" },
    { "from": "clb_0_0.lut_a.out", "to": "clb_0_1.lut_a.in0" },
    { "from": "clb_0_0.ff_a.out", "to": "io_pin_8" }
  ],

  "bram": {
    "bram_0": {
      "width": 8,
      "initial_contents": [0, 0, 0, 255, 128, 64]
    }
  },

  "io": {
    "io_pin_0":  { "direction": "input",  "name": "A" },
    "io_pin_1":  { "direction": "input",  "name": "B" },
    "io_pin_8":  { "direction": "output", "name": "Result" }
  },

  "clock": {
    "frequency_mhz": 100,
    "source": "io_pin_15"
  }
}
```

## Worked Examples

### Example 1: Single LUT as AND gate

The simplest possible FPGA design: one LUT implementing a 2-input AND gate.

**Configuration:**
```json
{
  "device": { "name": "CinchFPGA-Mini", "rows": 2, "cols": 2, "lut_inputs": 4 },
  "clbs": {
    "clb_0_0": {
      "lut_a": {
        "truth_table": [0,0,0,1, 0,0,0,0, 0,0,0,0, 0,0,0,0],
        "comment": "AND(in0, in1) — only inputs 0,1 used; inputs 2,3 tied to 0"
      }
    }
  },
  "routing": [
    { "from": "io_pin_0", "to": "clb_0_0.lut_a.in0" },
    { "from": "io_pin_1", "to": "clb_0_0.lut_a.in1" },
    { "from": "clb_0_0.lut_a.out", "to": "io_pin_4" }
  ],
  "io": {
    "io_pin_0": { "direction": "input",  "name": "A" },
    "io_pin_1": { "direction": "input",  "name": "B" },
    "io_pin_4": { "direction": "output", "name": "Y" }
  }
}
```

**The graph built from this configuration:**
```
io_pin_0 ("A") ──→ clb_0_0.lut_a.in0 ──→ clb_0_0.lut_a.out ──→ io_pin_4 ("Y")
                                              ↑
io_pin_1 ("B") ──→ clb_0_0.lut_a.in1 ────────┘
```

**Simulation:**
```python
fpga = FPGA.from_json("and_gate.json")
result = fpga.simulate(
    inputs={"A": [0, 0, 1, 1], "B": [0, 1, 0, 1]},
    cycles=4
)
assert result.outputs["Y"] == [[0], [0], [0], [1]]
```

### Example 2: 4-bit Ripple-Carry Adder

Each bit of the adder uses one CLB (2 LUTs: one for sum, one for carry).

```
Bit 0:  LUT_A computes Sum0 = XOR(A0, B0, Cin)
        LUT_B computes Carry0 = majority(A0, B0, Cin)

Bit 1:  LUT_A computes Sum1 = XOR(A1, B1, Carry0)
        LUT_B computes Carry1 = majority(A1, B1, Carry0)

...and so on for bits 2 and 3.
```

**The graph for a 4-bit adder:**
```
A0 ──→ CLB_0.lut_a ──→ Sum0
B0 ──→ CLB_0.lut_a
Cin ──→ CLB_0.lut_a
A0 ──→ CLB_0.lut_b ──→ Carry0 ──(carry chain)──→ CLB_1.lut_a
B0 ──→ CLB_0.lut_b                                CLB_1.lut_b
Cin ──→ CLB_0.lut_b

A1 ──→ CLB_1.lut_a ──→ Sum1
B1 ──→ CLB_1.lut_a
       CLB_1.lut_b ──→ Carry1 ──(carry chain)──→ CLB_2 ...

A2 ──→ CLB_2.lut_a ──→ Sum2
...

A3 ──→ CLB_3.lut_a ──→ Sum3
       CLB_3.lut_b ──→ Cout
```

**Simulation (5 + 3 = 8):**
```python
fpga = FPGA.from_json("adder_4bit.json")
result = fpga.simulate(
    inputs={
        "A0": [1], "A1": [0], "A2": [1], "A3": [0],  # A = 0101 = 5
        "B0": [1], "B1": [1], "B2": [0], "B3": [0],  # B = 0011 = 3
        "Cin": [0]
    },
    cycles=1
)
# Sum = 1000 = 8, Cout = 0
assert result.outputs["Sum0"] == [[0]]
assert result.outputs["Sum1"] == [[0]]
assert result.outputs["Sum2"] == [[0]]
assert result.outputs["Sum3"] == [[1]]
assert result.outputs["Cout"] == [[0]]
```

### Example 3: Traffic Light State Machine

A sequential circuit that cycles through Red → Green → Yellow → Red, advancing
on each clock cycle. This uses LUTs for next-state logic and flip-flops for
state storage.

```
State encoding:
  Red    = 00
  Green  = 01
  Yellow = 10

Next-state logic:
  00 → 01 (Red → Green)
  01 → 10 (Green → Yellow)
  10 → 00 (Yellow → Red)
  11 → 00 (invalid → Red, safe default)

LUT_A (next_state[0]):         LUT_B (next_state[1]):
  state1 state0 │ next0         state1 state0 │ next1
  ──────────────┼──────         ──────────────┼──────
    0      0    │   1             0      0    │   0
    0      1    │   0             0      1    │   1
    1      0    │   0             1      0    │   0
    1      1    │   0             1      1    │   0
```

Both LUTs have their outputs registered (through flip-flops), creating the
feedback loop: current state → LUT → next state → flip-flop → current state
on next clock cycle.

**The graph:**
```
clb_0_0.ff_a.out ──→ clb_0_0.lut_a.in0 ──→ clb_0_0.ff_a ──→ (feedback)
clb_0_0.ff_b.out ──→ clb_0_0.lut_a.in1         ↓
                                            io_pin_4 (Red)
clb_0_0.ff_a.out ──→ clb_0_0.lut_b.in0 ──→ clb_0_0.ff_b ──→ (feedback)
clb_0_0.ff_b.out ──→ clb_0_0.lut_b.in1         ↓
                                            io_pin_5 (Green)
                                            io_pin_6 (Yellow)
```

This example demonstrates the core FPGA capability: implementing sequential
circuits (state machines) by routing flip-flop outputs back to LUT inputs.

## Public API

```python
from enum import Enum
from dataclasses import dataclass


# ═══════════════════════════════════════════════════════════════
# Core Primitives
# ═══════════════════════════════════════════════════════════════

class LUT:
    """K-input Look-Up Table — the atom of programmable logic.

    Stores a truth table in SRAM and uses a MUX tree to select
    the output based on input signals. Can implement ANY boolean
    function of K variables.
    """

    def __init__(self, k: int = 4, truth_table: list[int] | None = None) -> None: ...
        # k: number of inputs (2-6). truth_table: 2^k entries, each 0 or 1.
        # If truth_table is None, all entries default to 0.

    def configure(self, truth_table: list[int]) -> None: ...
        # Load a new truth table (reprogram the LUT)

    def evaluate(self, inputs: list[int]) -> int: ...
        # Compute output for given inputs by indexing into truth table

    @property
    def k(self) -> int: ...
    @property
    def truth_table(self) -> list[int]: ...


class Slice:
    """One slice of a CLB: 2 LUTs + 2 flip-flops + output MUXes + carry chain."""

    def __init__(self, lut_inputs: int = 4) -> None: ...

    def configure(
        self,
        lut_a_table: list[int],
        lut_b_table: list[int],
        ff_a_enabled: bool = False,
        ff_b_enabled: bool = False,
        carry_enabled: bool = False,
    ) -> None: ...

    def evaluate(
        self,
        inputs_a: list[int],
        inputs_b: list[int],
        clock: int,
        carry_in: int = 0,
    ) -> "SliceOutput": ...


@dataclass
class SliceOutput:
    output_a: int          # LUT A result (combinational or registered)
    output_b: int          # LUT B result (combinational or registered)
    carry_out: int         # Carry chain output


class CLB:
    """Configurable Logic Block — contains 2 slices."""

    def __init__(self, lut_inputs: int = 4) -> None: ...

    @property
    def slice0(self) -> Slice: ...
    @property
    def slice1(self) -> Slice: ...

    def evaluate(
        self,
        slice0_inputs_a: list[int],
        slice0_inputs_b: list[int],
        slice1_inputs_a: list[int],
        slice1_inputs_b: list[int],
        clock: int,
        carry_in: int = 0,
    ) -> "CLBOutput": ...


@dataclass
class CLBOutput:
    slice0: SliceOutput
    slice1: SliceOutput


class SwitchMatrix:
    """Programmable routing crossbar.

    Connects wire segments at routing intersections.
    Each crossing point is controlled by an SRAM configuration bit.
    """

    def __init__(self, width: int) -> None: ...
        # width: number of wires on each side

    def configure(self, connections: list[tuple[str, str]]) -> None: ...
        # Set which input-output pairs are connected.
        # e.g., [("north_0", "east_2"), ("south_1", "west_3")]

    def route(self, signals: dict[str, int]) -> dict[str, int]: ...
        # Given input signals, compute output signals based on connections


class IODirection(Enum):
    INPUT = "input"
    OUTPUT = "output"
    BIDIRECTIONAL = "bidirectional"


class IOBlock:
    """Bidirectional I/O pad with optional flip-flops."""

    def __init__(self, direction: IODirection = IODirection.INPUT) -> None: ...

    def configure(
        self,
        direction: IODirection,
        input_ff: bool = False,
        output_ff: bool = False,
    ) -> None: ...

    def drive_external(self, value: int) -> None: ...
        # Set external pin value (for input mode)

    def drive_internal(self, value: int) -> None: ...
        # Set internal signal value (for output mode)

    def read(self, clock: int = 0) -> int | None: ...
        # Read the value (from external for input, from internal for output)
        # Returns None if high-impedance (bidirectional, not driving)


# ═══════════════════════════════════════════════════════════════
# FPGA Fabric
# ═══════════════════════════════════════════════════════════════

@dataclass
class DeviceConfig:
    """FPGA device parameters."""
    name: str = "CinchFPGA-Mini"
    rows: int = 4                  # CLB grid rows
    cols: int = 4                  # CLB grid columns
    lut_inputs: int = 4            # K-input LUTs
    slices_per_clb: int = 2
    bram_tiles: int = 2            # Number of Block RAM tiles
    bram_bits_per_tile: int = 18432  # 18 Kbit per tile
    io_pins: int = 16


@dataclass
class TimingReport:
    """Timing analysis results."""
    critical_path: list[str]       # Node IDs along the longest path
    critical_path_delay_ns: float  # Total delay in nanoseconds
    max_frequency_mhz: float       # 1 / (delay + setup_time)
    lut_delay_ns: float = 0.3      # Per-LUT delay
    routing_delay_ns: float = 0.5  # Per-hop routing delay
    setup_time_ns: float = 0.1     # FF setup time


@dataclass
class SimResult:
    """Simulation output."""
    outputs: dict[str, list[list[int]]]  # pin_name → per-cycle output values
    timing: TimingReport
    cycle_count: int


class FPGA:
    """Complete FPGA fabric: CLBs + BRAMs + I/O + routing.

    This is the top-level class. Create it from a JSON configuration
    file, then simulate with input stimulus.
    """

    def __init__(self, config: DeviceConfig) -> None: ...

    @classmethod
    def from_json(cls, path: str) -> "FPGA": ...
        # Load device config and design from a JSON file

    @classmethod
    def from_dict(cls, config: dict) -> "FPGA": ...
        # Load from a Python dict (same schema as JSON)

    def configure(self, design: dict) -> None: ...
        # Apply a design configuration (LUT tables, routing, I/O)
        # Validates: no undriven inputs, no combinational loops, etc.

    def simulate(
        self,
        inputs: dict[str, list[int]],
        cycles: int,
    ) -> SimResult: ...
        # Run simulation for N clock cycles with given input stimulus.
        # inputs: pin_name → list of values (one per cycle, or single value held constant)
        # Returns outputs for each cycle.

    def analyze_timing(self) -> TimingReport: ...
        # Static timing analysis — find critical path and max frequency

    @property
    def routing_graph(self) -> "DirectedGraph": ...
        # The directed graph representing all active routes

    @property
    def clbs(self) -> dict[str, CLB]: ...
    @property
    def brams(self) -> dict[str, "ConfigurableBRAM"]: ...
    @property
    def io_blocks(self) -> dict[str, IOBlock]: ...

    def utilization(self) -> dict[str, str]: ...
        # Report: how many CLBs/LUTs/FFs/BRAMs are used vs. available
        # e.g., {"luts": "12/32 (37.5%)", "ffs": "4/32 (12.5%)", ...}
```

## Data Flow

```
Configuration phase:
  Input:  JSON config file (device params + LUT tables + routing + I/O)
  Process: build CLBs, load LUT truth tables, configure routing graph,
           set up I/O blocks, initialize BRAMs
  Output: configured FPGA ready for simulation

Simulation phase (per clock cycle):
  1. Read external inputs from I/O blocks
  2. Evaluate all CLBs in topological order (from routing graph)
  3. Propagate signals through routing fabric
  4. Clock all flip-flops (capture LUT outputs)
  5. Drive output I/O pins
  6. Record outputs

Timing analysis:
  Input:  configured routing graph with delay annotations
  Process: find longest combinational path (modified BFS/DFS on routing graph)
  Output: TimingReport with critical path and max frequency
```

## Test Strategy

### LUT Tests
- 2-input LUT: configure as AND, verify all 4 input combinations
- 4-input LUT: configure as XOR, verify all 16 input combinations
- Reconfigure: load AND table, verify, load OR table, verify same LUT now computes OR
- Empty LUT (all zeros): all inputs produce 0
- Full LUT (all ones): all inputs produce 1
- Invalid truth table length → error
- Invalid input values → error

### CLB Tests
- Combinational mode: LUT output goes directly to CLB output
- Registered mode: LUT output captured by FF on clock edge, output delayed by one cycle
- Carry chain: verify carry propagates between LUT A and LUT B within a slice
- Carry chain across CLBs: verify carry propagates between adjacent CLBs
- Both slices independent: verify slice 0 and slice 1 operate independently

### Routing Tests
- Direct connection: CLB A output → CLB B input, verify signal propagates
- Multi-hop route: A → switch matrix → B → switch matrix → C
- Fan-out: one output drives multiple inputs
- No fan-in violation: each input driven by exactly one source
- Undriven input detection: error if any CLB input has no source
- Combinational loop detection: error if routing creates a cycle with no flip-flop

### I/O Block Tests
- Input mode: external value propagates to internal
- Output mode: internal value drives pin
- Bidirectional: switch between input and output
- With input FF: value delayed by one clock cycle
- High-impedance: bidirectional pin not driving → None

### FPGA Integration Tests
- AND gate example (Example 1): verify correct outputs for all input combinations
- 4-bit adder (Example 2): verify addition for several test vectors including overflow
- State machine (Example 3): verify correct state transitions over multiple cycles
- Timing analysis: verify critical path matches expected longest path
- Utilization report: verify counts are accurate
- Invalid configuration detection: missing routes, loops, width mismatches

### Edge Cases
- Minimum FPGA: 1×1 grid, 1 I/O pin
- All LUTs unused (configured but no routing)
- BRAM read and write in same cycle (verify read mode behavior)
- Clock frequency = 0 (combinational-only design, no FFs)
- Maximum fan-out from a single CLB output

## Future Extensions

- **DSP slices** — hardwired multiply-accumulate blocks (like Xilinx DSP48)
- **PLL / clock management tiles** — frequency synthesis, phase alignment, multiple clock domains
- **Partial reconfiguration** — change part of the FPGA while the rest keeps running
- **HDL synthesis** — accept a subset of Verilog/VHDL, synthesize to LUT/routing configuration
- **Bitstream generation** — produce actual binary bitstreams (not just JSON)
- **Floorplanning visualization** — render the CLB grid with utilization heat map
- **Power estimation** — estimate dynamic and static power based on toggle rates and routing length
- **Multi-die / chiplet FPGAs** — model modern FPGAs that use multiple interconnected silicon dies
- **Hard processor cores** — model FPGAs with embedded ARM/RISC-V cores (like Xilinx Zynq)
- **SerDes / high-speed transceivers** — model multi-gigabit serial I/O (for networking, PCIe)
