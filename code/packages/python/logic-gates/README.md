# Logic Gates

**Layer 1 of the computing stack** — the foundation of all digital logic.

## What is a logic gate?

A logic gate is the simplest possible computing element. It takes one or two binary inputs (each either 0 or 1) and produces one binary output (0 or 1). The output is completely determined by the inputs — no state, no memory, no randomness.

In real hardware, logic gates are built from transistors — tiny electronic switches. A modern CPU contains billions of transistors organized into billions of logic gates. But at the conceptual level, every computation reduces to these simple 0-or-1 operations.

## The gates

### NOT (Inverter)

The simplest gate — it has one input and flips it. If the input is 0, the output is 1. If the input is 1, the output is 0.

```
Input → Output
  0   →   1
  1   →   0
```

**Real-world analogy:** A light switch. If the light is off (0), flipping the switch turns it on (1), and vice versa.

### AND

Takes two inputs. The output is 1 **only if both inputs are 1**. If either input is 0, the output is 0.

```
A  B  → Output
0  0  →   0
0  1  →   0
1  0  →   0
1  1  →   1
```

**Real-world analogy:** Two switches in series (one after the other). Current flows (output = 1) only if both switches are closed (both = 1).

### OR

Takes two inputs. The output is 1 **if either input is 1** (or both).

```
A  B  → Output
0  0  →   0
0  1  →   1
1  0  →   1
1  1  →   1
```

**Real-world analogy:** Two switches in parallel (side by side). Current flows if either switch is closed.

### XOR (Exclusive OR)

Takes two inputs. The output is 1 **if the inputs are different**. Unlike OR, XOR outputs 0 when both inputs are 1.

```
A  B  → Output
0  0  →   0
0  1  →   1
1  0  →   1
1  1  →   0
```

**Why it matters:** XOR is the key gate for addition. In binary, 1 + 1 = 10 (the sum digit is 0, carry is 1). That sum digit is exactly what XOR computes.

### NAND (NOT AND)

The opposite of AND — output is 0 only when both inputs are 1. NAND is special because **every other gate can be built from NAND alone**. This property is called "functional completeness." In real chip manufacturing, entire processors are built from NAND gates because they are the cheapest to produce.

```
A  B  → Output
0  0  →   1
0  1  →   1
1  0  →   1
1  1  →   0
```

### NOR (NOT OR)

The opposite of OR — output is 1 only when both inputs are 0. Like NAND, NOR is also functionally complete.

```
A  B  → Output
0  0  →   1
0  1  →   0
1  0  →   0
1  1  →   0
```

### XNOR (Exclusive NOR)

The opposite of XOR — output is 1 **when inputs are the same**.

```
A  B  → Output
0  0  →   1
0  1  →   0
1  0  →   0
1  1  →   1
```

**Use case:** Equality comparison. XNOR(a, b) = 1 means a and b are equal.

## NAND-derived gates

This package also includes implementations of all gates built exclusively from NAND operations, proving functional completeness:

- `nand_not(a)` — NOT from NAND: `NAND(a, a)`
- `nand_and(a, b)` — AND from NAND: `NOT(NAND(a, b))`
- `nand_or(a, b)` — OR from NAND: `NAND(NOT(a), NOT(b))`
- `nand_xor(a, b)` — XOR from NAND: built from 4 NAND gates

## Where it fits

```
[Logic Gates] → Arithmetic → CPU → ARM/RISC-V → Assembler → Lexer → Parser → Compiler → VM
```

This package is used by the **arithmetic** package to build half adders, full adders, and the ALU.

## Installation

```bash
uv add coding-adventures-logic-gates
```

## Usage

```python
from logic_gates import AND, OR, NOT, XOR, NAND, AND_N

AND(1, 1)    # 1 — both inputs are 1
AND(1, 0)    # 0 — one input is 0
OR(0, 1)     # 1 — at least one input is 1
NOT(1)       # 0 — inverted
XOR(1, 0)    # 1 — inputs are different
XOR(1, 1)    # 0 — inputs are the same
AND_N(1, 1, 1, 0)  # 0 — one input is 0
```

## Spec

See [01-logic-gates.md](../../../specs/01-logic-gates.md) for the full specification.
