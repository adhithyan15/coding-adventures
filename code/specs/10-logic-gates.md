# 01 — Logic Gates

## Overview

The logic gates package implements the fundamental building blocks of all digital circuits. Every computation a computer performs — from adding numbers to running neural networks — ultimately reduces to combinations of these gates.

This is Layer 1 of the computing stack. It has no dependencies.

## Layer Position

```
[YOU ARE HERE] → Arithmetic → CPU → ARM → Assembler → Lexer → Parser → Compiler → VM
```

**Input from:** Nothing — this is the foundation.
**Output to:** Arithmetic package (half adders, full adders, ALU).

## Concepts

### What is a logic gate?

A logic gate takes one or two binary inputs (0 or 1) and produces one binary output (0 or 1). The output is determined entirely by the input — no state, no memory, no randomness.

### The fundamental gates

| Gate | Inputs | Output | Description |
|------|--------|--------|-------------|
| NOT  | 1      | 1      | Inverts the input. 0→1, 1→0 |
| AND  | 2      | 1      | Output is 1 only if BOTH inputs are 1 |
| OR   | 2      | 1      | Output is 1 if EITHER input is 1 |
| XOR  | 2      | 1      | Output is 1 if inputs are DIFFERENT |
| NAND | 2      | 1      | NOT(AND) — opposite of AND |
| NOR  | 2      | 1      | NOT(OR) — opposite of OR |
| XNOR | 2     | 1      | NOT(XOR) — output is 1 if inputs are SAME |

### Why NAND is special

Every other gate can be built from NAND gates alone. This is called **functional completeness**. In real hardware, chips are often built entirely from NAND gates because they are the cheapest to manufacture.

```
NOT(a)    = NAND(a, a)
AND(a, b) = NOT(NAND(a, b))
OR(a, b)  = NAND(NOT(a), NOT(b))
```

### Sequential logic

The gates above are "combinational" — output depends only on current inputs. Sequential
logic adds **memory** through feedback (wiring a gate's output back to its input).

From feedback, we build the entire memory hierarchy:

```
SR Latch          → raw 1-bit memory (2 cross-coupled NOR gates)
D Latch           → controlled 1-bit memory (SR + enable signal)
D Flip-Flop       → edge-triggered 1-bit memory (2 D latches, master-slave)
Register          → N-bit word storage (N flip-flops in parallel)
Shift Register    → serial-to-parallel converter (chained flip-flops)
Counter           → binary counting (register + incrementer)
```

#### SR Latch

Two NOR gates cross-coupled — each gate's output feeds into the other's input. This
creates two stable states that persist even after inputs change.

```
S  R  | Q    Q_bar  | Action
------+-------------+----------------------------------
0  0  | Q    Q_bar  | Hold — remember previous state
1  0  | 1    0      | Set — store a 1
0  1  | 0    1      | Reset — store a 0
1  1  | 0    0      | Invalid — both outputs forced low
```

#### D Latch

Solves the SR latch's invalid state problem by deriving S and R from a single data
input D. An enable signal controls when the latch is transparent vs. opaque.

```
D  E  | Q    Q_bar  | Action
------+-------------+----------------------------------
X  0  | Q    Q_bar  | Hold — latch is opaque
0  1  | 0    1      | Store 0 — transparent
1  1  | 1    0      | Store 1 — transparent
```

#### D Flip-Flop

Master-slave configuration: two D latches in series with opposite enables. Data is
captured at the **rising edge** of the clock (transition from 0→1). This prevents
data races in pipelined circuits.

```
         ┌────────────┐          ┌────────────┐
  Data ──┤ D Latch    ├──────────┤ D Latch    ├── Q
         │ (Master)   │          │ (Slave)    │
  CLK' ──┤ Enable     │   CLK ──┤ Enable     │
         └────────────┘          └────────────┘
```

#### Register, Shift Register, Counter

- **Register**: N flip-flops in parallel sharing a clock — captures an N-bit word simultaneously.
- **Shift Register**: Chain of flip-flops where each output feeds the next input. Bits shift one position per clock cycle.
- **Counter**: Register + incrementer (chain of half-adders with carry_in=1). Counts up on each clock cycle, wraps on overflow.

### Combinational circuits

Between primitive gates and full arithmetic circuits, there is a family of
**combinational building blocks** used everywhere in digital design — CPUs, FPGAs,
memory controllers, bus arbiters. These circuits have no memory; output depends only
on current inputs.

#### Multiplexer (MUX)

A multiplexer is a **selector switch**. It takes N data inputs and a set of select
lines, and routes exactly one input to the output. Think of it as a railroad switch
that directs one of several trains onto a single track.

**2-to-1 MUX** — the simplest case:

```
         ┌──────────┐
  D0 ────┤          │
         │   MUX    ├──── Output
  D1 ────┤          │
         └────┬─────┘
              │
  Sel ────────┘

  Sel = 0 → Output = D0
  Sel = 1 → Output = D1
```

Truth table:

```
Sel  D0  D1  │ Output
─────────────┼───────
 0    0   X  │   0       D0 selected
 0    1   X  │   1       D0 selected
 1    X   0  │   0       D1 selected
 1    X   1  │   1       D1 selected
```

Built from gates: `Output = OR(AND(D0, NOT(Sel)), AND(D1, Sel))`

**4-to-1 MUX** — uses 2 select lines:

```
         ┌──────────┐
  D0 ────┤          │
  D1 ────┤   MUX    ├──── Output
  D2 ────┤          │
  D3 ────┤          │
         └───┬──┬───┘
             │  │
  S0 ────────┘  │
  S1 ───────────┘

  S1 S0 = 00 → Output = D0
  S1 S0 = 01 → Output = D1
  S1 S0 = 10 → Output = D2
  S1 S0 = 11 → Output = D3
```

Built from three 2:1 MUXes or directly from AND/OR gates.

**Why MUX matters everywhere:**
- In FPGAs, a K-input LUT is literally a 2^K-to-1 MUX with the truth table stored in SRAM
- In CPUs, MUXes select between register file outputs, ALU inputs, and forwarded values
- In memory, MUXes route data to/from the correct bank

**N-to-1 MUX** — generalizes to any power-of-2 size using ⌈log₂(N)⌉ select lines.
Can be built recursively from 2:1 MUXes:

```
8:1 MUX = two 4:1 MUXes feeding a 2:1 MUX
4:1 MUX = two 2:1 MUXes feeding a 2:1 MUX
```

#### Demultiplexer (DEMUX)

The inverse of a MUX: one data input, N outputs, select lines choose which output
receives the data. All other outputs are 0.

**1-to-4 DEMUX:**

```
  Sel  │ Y0  Y1  Y2  Y3
  ─────┼─────────────────
  00   │  D   0   0   0
  01   │  0   D   0   0
  10   │  0   0   D   0
  11   │  0   0   0   D
```

Used in memory address decoding: the address bits select which memory chip/bank
receives the read/write signal.

#### Decoder

A decoder converts an N-bit binary input into a one-hot output — exactly one of
2^N output lines is 1, the rest are 0. It's a DEMUX with the data input hardwired to 1.

**2-to-4 Decoder:**

```
  A1  A0  │ Y0  Y1  Y2  Y3
  ────────┼─────────────────
   0   0  │  1   0   0   0
   0   1  │  0   1   0   0
   1   0  │  0   0   1   0
   1   1  │  0   0   0   1
```

Built from AND and NOT gates:

```
Y0 = AND(NOT(A1), NOT(A0))
Y1 = AND(NOT(A1), A0)
Y2 = AND(A1, NOT(A0))
Y3 = AND(A1, A0)
```

Used everywhere: instruction decoding in CPUs (opcode → control signals), memory chip
select, interrupt priority encoding.

**3-to-8 Decoder:** same pattern with 3 inputs, 8 outputs. Each output is an AND of
all 3 input bits (or their complements).

#### Encoder

The inverse of a decoder: 2^N input lines (one-hot), N-bit binary output.

**4-to-2 Encoder:**

```
  I0  I1  I2  I3  │ A1  A0
  ────────────────┼────────
   1   0   0   0  │  0   0
   0   1   0   0  │  0   1
   0   0   1   0  │  1   0
   0   0   0   1  │  1   1
```

**Priority Encoder** — handles the case where multiple inputs are active. The
highest-priority active input wins, and a "valid" output indicates whether any
input is active at all.

**4-to-2 Priority Encoder** (I3 = highest priority):

```
  I0  I1  I2  I3  │ A1  A0  Valid
  ────────────────┼─────────────
   0   0   0   0  │  X   X    0     No input active
   1   0   0   0  │  0   0    1     I0 wins
   X   1   0   0  │  0   1    1     I1 wins over I0
   X   X   1   0  │  1   0    1     I2 wins over I0,I1
   X   X   X   1  │  1   1    1     I3 always wins
```

Used in interrupt controllers (which interrupt fires when multiple arrive
simultaneously?) and in FPGA carry chain logic.

#### Tri-state buffer

A tri-state buffer has three possible output states: 0, 1, or **high-impedance (Z)**.
High-impedance means the output is electrically disconnected — as if the wire were cut.

```
  Data  Enable │ Output
  ─────────────┼───────
    0      0   │   Z      Disconnected
    1      0   │   Z      Disconnected
    0      1   │   0      Active low
    1      1   │   1      Active high
```

Why this matters: in a shared bus (like a memory data bus), multiple devices connect
to the same wires. Only one device can drive the bus at a time. Tri-state buffers let
each device disconnect when it's not its turn, preventing electrical conflicts.

In FPGAs, tri-state buffers appear in I/O blocks where pins can be configured as
inputs (high-Z) or outputs (driven).

**Modeling Z in software:** we represent high-impedance as `None` (Python), `nil`
(Ruby), or a special sentinel value. The output type becomes `int | None` where `None`
means high-impedance.

## Public API

```python
# === Primitive Gates ===
# All functions take int (0 or 1) and return int (0 or 1)

def NOT(a: int) -> int: ...
def AND(a: int, b: int) -> int: ...
def OR(a: int, b: int) -> int: ...
def XOR(a: int, b: int) -> int: ...
def NAND(a: int, b: int) -> int: ...
def NOR(a: int, b: int) -> int: ...
def XNOR(a: int, b: int) -> int: ...

# Derived: build all gates from NAND only
def nand_not(a: int) -> int: ...
def nand_and(a: int, b: int) -> int: ...
def nand_or(a: int, b: int) -> int: ...
def nand_xor(a: int, b: int) -> int: ...

# Multi-input variants
def AND_N(*inputs: int) -> int: ...  # AND with N inputs
def OR_N(*inputs: int) -> int: ...   # OR with N inputs

# === Sequential Logic ===

def sr_latch(set_: int, reset: int, q: int = 0, q_bar: int = 1) -> tuple[int, int]: ...
def d_latch(data: int, enable: int, q: int = 0, q_bar: int = 1) -> tuple[int, int]: ...
def d_flip_flop(data: int, clock: int, ...) -> tuple[int, int, dict[str, int]]: ...
def register(data: list[int], clock: int, ...) -> tuple[list[int], list[dict[str, int]]]: ...
def shift_register(serial_in: int, clock: int, ...) -> tuple[list[int], int, list[dict[str, int]]]: ...
def counter(clock: int, reset: int = 0, ...) -> tuple[list[int], dict]: ...

# === Combinational Circuits ===

def mux2(d0: int, d1: int, sel: int) -> int: ...
    # 2-to-1 multiplexer: sel=0 → d0, sel=1 → d1

def mux4(d0: int, d1: int, d2: int, d3: int, sel: list[int]) -> int: ...
    # 4-to-1 multiplexer: sel is [s0, s1]

def mux8(inputs: list[int], sel: list[int]) -> int: ...
    # 8-to-1 multiplexer: sel is [s0, s1, s2]

def mux_n(inputs: list[int], sel: list[int]) -> int: ...
    # N-to-1 multiplexer (N must be power of 2)

def demux(data: int, sel: list[int], n_outputs: int) -> list[int]: ...
    # 1-to-N demultiplexer: routes data to selected output, others are 0

def decoder(inputs: list[int]) -> list[int]: ...
    # N-to-2^N decoder: input is N bits, output is 2^N bits (one-hot)

def encoder(inputs: list[int]) -> list[int]: ...
    # 2^N-to-N encoder: one-hot input, binary output

def priority_encoder(inputs: list[int]) -> tuple[list[int], int]: ...
    # Priority encoder: returns (binary_output, valid_flag)
    # Highest-index active input wins

def tri_state(data: int, enable: int) -> int | None: ...
    # Tri-state buffer: enable=1 → data, enable=0 → None (high-Z)
```

## Data Flow

```
Primitive gates:
  Input:  one or two integers, each either 0 or 1
  Output: one integer, either 0 or 1

Sequential logic:
  Input:  data bits + clock/enable signals + previous state
  Output: new output bits + new internal state

Combinational circuits:
  Input:  data bits + select/control bits
  Output: selected/decoded/encoded bits
```

Inputs outside {0, 1} should raise a ValueError with a clear message.
Tri-state buffer returns `None` for high-impedance state.

## Test Strategy

Logic gates are fully specified by their truth tables. Every gate gets tested against its complete truth table:

```python
def test_and_gate():
    assert AND(0, 0) == 0
    assert AND(0, 1) == 0
    assert AND(1, 0) == 0
    assert AND(1, 1) == 1
```

Additional tests:
- Verify all NAND-derived gates match their direct implementations
- Verify multi-input variants work for 2, 3, 4+ inputs
- Verify invalid inputs (2, -1, "a") raise ValueError
- Verify type hints are correct with mypy/pyright

Sequential logic tests:
- SR latch: all truth table entries including hold and invalid states
- D latch: transparent mode (enable=1) and opaque mode (enable=0)
- D flip-flop: rising edge capture, verify data doesn't leak through during hold
- Register: store and retrieve N-bit words, verify all bits captured simultaneously
- Shift register: shift in a known pattern, verify serial and parallel outputs
- Counter: count from 0 to max, verify overflow/wrap-around, verify reset

Combinational circuit tests:
- MUX2: exhaustive truth table (8 rows: 2 data × 1 select × 2 values each)
- MUX4: verify each select combination routes the correct input
- MUX8/MUX_N: verify select line routing for all positions
- DEMUX: verify data appears on correct output, all others are 0
- Decoder: verify one-hot output for all input combinations
- Encoder: verify binary output for all valid one-hot inputs
- Priority encoder: verify highest-priority input wins, valid flag correct
- Tri-state: verify output matches data when enabled, returns None when disabled
- Edge cases: MUX with all-0 inputs, all-1 inputs, decoder with all-0 input

## Future Extensions

- **Gate delay simulation**: Model propagation delay through gates (useful for understanding timing in real circuits)
- **Circuit visualization**: Render gate diagrams
- **Gate count tracking**: Count how many primitive gates a complex circuit uses
- **Barrel shifter**: Shift by arbitrary amounts in one step using MUX tree
- **Comparator**: N-bit equality and magnitude comparison using XNOR gates
- **Parity generator/checker**: XOR tree for error detection
