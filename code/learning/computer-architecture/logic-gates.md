# Logic Gates — The Foundation of All Digital Computing

Everything a computer does — adding numbers, rendering graphics, running AI
models — ultimately reduces to billions of tiny switches flipping between
two states: **0** and **1**. These switches are organized into **logic gates**,
the simplest possible decision-making elements.

This document explains every fundamental gate, proves that a single gate type
(NAND) can build all others, covers multi-input chaining, DeMorgan's laws,
and introduces sequential logic (memory). It references the Python
implementation at `code/packages/python/logic-gates/`.

---

## Table of Contents

1. [What is a Logic Gate?](#1-what-is-a-logic-gate)
2. [The Seven Fundamental Gates](#2-the-seven-fundamental-gates)
3. [Functional Completeness — NAND Builds Everything](#3-functional-completeness--nand-builds-everything)
4. [Multi-Input Gates](#4-multi-input-gates)
5. [DeMorgan's Laws](#5-demorgans-laws)
6. [Sequential Logic — Memory from Feedback](#6-sequential-logic--memory-from-feedback)
7. [The Python Implementation](#7-the-python-implementation)

---

## 1. What is a Logic Gate?

### The Physical Analogy: Transistors

A **transistor** is a tiny electronic switch. It has three connections:

```
        Gate (control)
          |
          v
  Source ---[===]--- Drain
             ^
          Transistor
```

When you apply voltage to the **gate** terminal, electricity flows from
**source** to **drain** (the switch closes). Remove the voltage, and the
flow stops (the switch opens). A modern CPU has billions of these.

A **logic gate** is a circuit built from one or more transistors that
implements a boolean function. It takes one or two binary inputs (each 0
or 1) and produces a single binary output (also 0 or 1). The output is
completely determined by the inputs — no randomness, no hidden state, no
memory.

### Why Binary?

Computers use binary because transistors are most reliable as on/off
switches. A transistor that is ON (conducting current) represents **1**.
A transistor that is OFF (blocking current) represents **0**. You could
build a ternary computer (base-3), but distinguishing three voltage levels
is harder and more error-prone than distinguishing two. Binary gives us
two clean, unmistakable states.

### The Hierarchy of Abstractions

```
    Transistors (physical switches)
         |
         v
    Logic Gates (NOT, AND, OR, ...)      <-- this document
         |
         v
    Adders, Multiplexers, Decoders       <-- arithmetic-circuits.md
         |
         v
    ALU, Registers, Control Unit         <-- cpu-architecture.md
         |
         v
    CPU, GPU, TPU                        <-- the whole computer
```

---

## 2. The Seven Fundamental Gates

There are seven gates that appear over and over in digital design. The first
four (NOT, AND, OR, XOR) are the **primitive** gates. The other three (NAND,
NOR, XNOR) are **composite** gates — each is the inverse of a primitive.

### 2.1 NOT Gate (Inverter)

The simplest gate. One input, one output. It flips the bit.

**Truth Table:**

```
    Input | Output
    ------+-------
      0   |   1
      1   |   0
```

**Circuit Symbol:**

```
    a ──>o── output
         ^
         the circle means "invert"
```

**Transistor Implementation:**
A NOT gate uses a single transistor. When the input is HIGH (1), the
transistor pulls the output LOW (0). When the input is LOW (0), a pull-up
resistor brings the output HIGH (1).

```
    Vcc (power)
     |
     R  (pull-up resistor)
     |
     +-------- output
     |
    [T] <---- input
     |
    GND
```

When input = 1: transistor ON, output connects to GND = 0.
When input = 0: transistor OFF, output pulled to Vcc = 1.

### 2.2 AND Gate

Two inputs, one output. The output is 1 **only** if **both** inputs are 1.

**Truth Table:**

```
    A  B | Output
    -----+-------
    0  0 |   0      Neither is 1
    0  1 |   0      Only B is 1
    1  0 |   0      Only A is 1
    1  1 |   1      Both are 1   <-- the only 1 output
```

**Circuit Symbol:**

```
    a ──┐
        |D── output
    b ──┘
```

**Physical Analogy:** Two switches wired **in series**. Current flows only
when both switches are closed.

```
    Power ──[Switch A]──[Switch B]──> Light
```

### 2.3 OR Gate

Two inputs, one output. The output is 1 if **either** input is 1 (or both).

**Truth Table:**

```
    A  B | Output
    -----+-------
    0  0 |   0      Neither is 1   <-- the only 0 output
    0  1 |   1      B is 1
    1  0 |   1      A is 1
    1  1 |   1      Both are 1
```

**Circuit Symbol:**

```
    a ──\
         )── output
    b ──/
```

**Physical Analogy:** Two switches wired **in parallel**. Current flows if
either switch is closed.

```
            ┌──[Switch A]──┐
    Power ──┤              ├──> Light
            └──[Switch B]──┘
```

### 2.4 XOR Gate (Exclusive OR)

Two inputs, one output. The output is 1 if the inputs are **different**.
Unlike OR, XOR outputs 0 when both inputs are 1.

**Truth Table:**

```
    A  B | Output
    -----+-------
    0  0 |   0      Same
    0  1 |   1      Different
    1  0 |   1      Different
    1  1 |   0      Same
```

**Circuit Symbol:**

```
    a ──\
        =)── output
    b ──/
```

**Why XOR Matters for Arithmetic:**
XOR computes the **sum digit** of binary addition (ignoring the carry):

```
    0 + 0 = 0   XOR(0,0) = 0   match!
    0 + 1 = 1   XOR(0,1) = 1   match!
    1 + 0 = 1   XOR(1,0) = 1   match!
    1 + 1 = 10  XOR(1,1) = 0   match! (the 1 is the carry, handled by AND)
```

This is why XOR is the key gate in building adder circuits. See
`code/learning/hardware/arithmetic-circuits.md` for details.

### 2.5 NAND Gate (NOT AND)

The inverse of AND. Output is 0 **only** when both inputs are 1.

**Truth Table:**

```
    A  B | Output
    -----+-------
    0  0 |   1
    0  1 |   1
    1  0 |   1
    1  1 |   0      <-- the only 0 output
```

**Circuit Symbol:**

```
    a ──┐
        |D>o── output
    b ──┘
          ^
          circle = inversion
```

**Why NAND is special:** NAND is **functionally complete** — you can build
every other gate using only NAND gates. See Section 3 below for the full proof.

### 2.6 NOR Gate (NOT OR)

The inverse of OR. Output is 1 **only** when both inputs are 0.

**Truth Table:**

```
    A  B | Output
    -----+-------
    0  0 |   1      <-- the only 1 output
    0  1 |   0
    1  0 |   0
    1  1 |   0
```

**Circuit Symbol:**

```
    a ──\
         )>o── output
    b ──/
```

NOR is also functionally complete (like NAND), meaning you can build any
gate from NOR alone. In practice, NAND is more commonly used in CMOS
technology because NAND gates are slightly faster and smaller.

### 2.7 XNOR Gate (Exclusive NOR / Equivalence Gate)

The inverse of XOR. Output is 1 when the inputs are the **same**.

**Truth Table:**

```
    A  B | Output
    -----+-------
    0  0 |   1      Same
    0  1 |   0      Different
    1  0 |   0      Different
    1  1 |   1      Same
```

**Circuit Symbol:**

```
    a ──\
        =)>o── output
    b ──/
```

**Use Case:** Equality comparison. XNOR(a, b) = 1 means a equals b. To
compare two N-bit numbers, you XNOR each pair of bits and AND all the
results together. If every pair is equal, the final AND outputs 1.

### Summary Table of All Seven Gates

```
    A  B | NOT(A) | AND | OR | XOR | NAND | NOR | XNOR
    -----+--------+-----+----+-----+------+-----+-----
    0  0 |   1    |  0  |  0 |  0  |  1   |  1  |  1
    0  1 |   0    |  0  |  1 |  1  |  1   |  0  |  0
    1  0 |   -    |  0  |  1 |  1  |  1   |  0  |  0
    1  1 |   -    |  1  |  1 |  0  |  0   |  0  |  1
```

(NOT only uses input A; the B column is irrelevant for NOT.)

---

## 3. Functional Completeness — NAND Builds Everything

**Functional completeness** means a set of gates that can implement ANY
boolean function. The set {NAND} is functionally complete. This is not just
theoretical — real chip manufacturers (like those producing the TTL 7400
series since 1966) build entire processors from NAND gates because it
simplifies fabrication.

Here we prove NAND can build NOT, AND, OR, and XOR. Since these four can
build any boolean function, NAND alone can build anything.

### 3.1 NOT from NAND

**Construction:** `NOT(a) = NAND(a, a)`

Feed the same signal to both inputs of NAND:

```
    a ──┬──┐
        |  |D>o── output
        └──┘
```

**Proof by truth table:**

```
    a | NAND(a, a) | NOT(a) | Match?
    --+------------+--------+-------
    0 | NAND(0,0)=1|   1    |  yes
    1 | NAND(1,1)=0|   0    |  yes
```

### 3.2 AND from NAND

**Construction:** `AND(a, b) = NAND(NAND(a, b), NAND(a, b))`

In other words: take the NAND of a and b, then invert the result (using
the NOT-from-NAND trick).

```
    a ──┐            ┌──┐
        |D>o── w ──┬─|  |D>o── output
    b ──┘          └─|  |
                     └──┘
    Gate 1: w = NAND(a, b)
    Gate 2: output = NAND(w, w) = NOT(w) = AND(a, b)
```

**Proof by truth table:**

```
    a  b | NAND(a,b)=w | NAND(w,w) | AND(a,b) | Match?
    -----+-------------+-----------+----------+-------
    0  0 |      1      |     0     |    0     |  yes
    0  1 |      1      |     0     |    0     |  yes
    1  0 |      1      |     0     |    0     |  yes
    1  1 |      0      |     1     |    1     |  yes
```

**Cost:** 2 NAND gates.

### 3.3 OR from NAND (using DeMorgan's Law)

**Construction:** `OR(a, b) = NAND(NAND(a, a), NAND(b, b))`

First invert each input (using NOT-from-NAND), then NAND the results.

```
    a ──┬──┐
        |  |D>o── w1 ──┐
        └──┘            |D>o── output
    b ──┬──┐            |
        |  |D>o── w2 ──┘
        └──┘

    Gate 1: w1 = NAND(a, a) = NOT(a)
    Gate 2: w2 = NAND(b, b) = NOT(b)
    Gate 3: output = NAND(NOT(a), NOT(b)) = OR(a, b)
```

**Why this works — DeMorgan's Law:**

```
    NAND(NOT(a), NOT(b))
    = NOT(AND(NOT(a), NOT(b)))     [definition of NAND]
    = NOT(NOT(a)) OR NOT(NOT(b))   [DeMorgan's Law]
    = a OR b                        [double negation]
```

**Proof by truth table:**

```
    a  b | NOT(a) | NOT(b) | NAND(NOT(a),NOT(b)) | OR(a,b) | Match?
    -----+--------+--------+---------------------+---------+-------
    0  0 |   1    |   1    |         0           |    0    |  yes
    0  1 |   1    |   0    |         1           |    1    |  yes
    1  0 |   0    |   1    |         1           |    1    |  yes
    1  1 |   0    |   0    |         1           |    1    |  yes
```

**Cost:** 3 NAND gates.

### 3.4 XOR from NAND

**Construction:**

```
    Let N = NAND(a, b)
    XOR(a, b) = NAND(NAND(a, N), NAND(b, N))
```

This is the most complex construction, requiring 4 NAND gates:

```
    a ──┬────────────┐
        |            |D>o── N ──┬──────────────┐
        |       ┌────┘          |              |
    b ──┼───────┘               |              |
        |                       |              |
        ├───────┐               |              |
        |       |D>o── w1 ─────┤              |
        |  N ───┘               |              |
        |                       |D>o── output  |
    b ──┤                       |              |
        |       ┌───────────────┘              |
        └───────|D>o── w2 ────────────────────┘
           N ───┘

    Gate 1: N    = NAND(a, b)
    Gate 2: w1   = NAND(a, N)
    Gate 3: w2   = NAND(b, N)
    Gate 4: output = NAND(w1, w2)
```

**Proof by truth table:**

```
    a  b | N=NAND(a,b) | NAND(a,N) | NAND(b,N) | NAND(w1,w2) | XOR(a,b) | Match?
    -----+-------------+-----------+-----------+-------------+----------+-------
    0  0 |      1      |     1     |     1     |      0      |    0     |  yes
    0  1 |      1      |     1     |     0     |      1      |    1     |  yes
    1  0 |      1      |     0     |     1     |      1      |    1     |  yes
    1  1 |      0      |     1     |     1     |      0      |    0     |  yes
```

**Cost:** 4 NAND gates. This explains why XOR is more expensive in hardware
than AND or OR.

### Summary: NAND Gate Costs

```
    Gate  | NAND gates needed
    ------+------------------
    NOT   |        1
    AND   |        2
    OR    |        3
    XOR   |        4
    NOR   |        4  (OR from 3 NANDs, then NOT from 1 NAND)
    XNOR  |        5  (XOR from 4 NANDs, then NOT from 1 NAND)
```

---

## 4. Multi-Input Gates

Real circuits often need to AND or OR more than two values. For example:
"Are ALL four conditions true?" requires a 4-input AND.

Multi-input gates work by **chaining** 2-input gates:

```
    AND(a, b, c, d) = AND(AND(AND(a, b), c), d)
```

### AND chain (4 inputs)

```
    a ──┐
        |D── r1 ──┐
    b ──┘          |D── r2 ──┐
              c ───┘          |D── output
                         d ───┘
```

The output is 1 only if ALL four inputs are 1. If any single input is 0,
the 0 propagates through and the output is 0.

### OR chain (4 inputs)

```
    a ──\
         )── r1 ──\
    b ──/          )── r2 ──\
              c ──/          )── output
                        d ──/
```

The output is 1 if ANY input is 1.

### Associativity

AND and OR are **associative**, meaning the grouping doesn't matter:

```
    AND(AND(a, b), AND(c, d)) = AND(AND(AND(a, b), c), d)
```

This means hardware designers can organize the chain as a tree for lower
latency:

```
    Tree structure (2 levels of delay):

        a ──┐              c ──┐
            |D── r1 ──┐       |D── r2 ──┐
        b ──┘          |   d ──┘          |D── output
                       └──────────────────┘

    Chain structure (3 levels of delay):

        a ──┐
            |D── ──┐
        b ──┘      |D── ──┐
              c ───┘      |D── output
                     d ───┘
```

The tree is faster (2 gate delays vs 3) but uses the same number of gates.

In the Python implementation, `AND_N` and `OR_N` use Python's `reduce()`
to chain the 2-input gate across all inputs. See
`code/packages/python/logic-gates/src/logic_gates/gates.py`.

---

## 5. DeMorgan's Laws

Augustus DeMorgan discovered these identities in the 1800s, long before
electronic computers existed. They are fundamental to digital logic design.

### The Two Laws

```
    Law 1:  NOT(A AND B)  =  (NOT A) OR (NOT B)
    Law 2:  NOT(A OR B)   =  (NOT A) AND (NOT B)
```

In English:
- **Law 1:** "It's NOT the case that both are true" is the same as "at
  least one is false."
- **Law 2:** "It's NOT the case that either is true" is the same as "both
  are false."

### Proof of Law 1 via Truth Table

```
    A  B | A AND B | NOT(A AND B) | NOT A | NOT B | (NOT A) OR (NOT B) | Match?
    -----+---------+--------------+-------+-------+--------------------+-------
    0  0 |    0    |      1       |   1   |   1   |         1          |  yes
    0  1 |    0    |      1       |   1   |   0   |         1          |  yes
    1  0 |    0    |      1       |   0   |   1   |         1          |  yes
    1  1 |    1    |      0       |   0   |   0   |         0          |  yes
```

Every row matches. Law 1 is proven.

### Proof of Law 2 via Truth Table

```
    A  B | A OR B | NOT(A OR B) | NOT A | NOT B | (NOT A) AND (NOT B) | Match?
    -----+--------+-------------+-------+-------+---------------------+-------
    0  0 |   0    |      1      |   1   |   1   |          1          |  yes
    0  1 |   1    |      0      |   1   |   0   |          0          |  yes
    1  0 |   1    |      0      |   0   |   1   |          0          |  yes
    1  1 |   1    |      0      |   0   |   0   |          0          |  yes
```

Every row matches. Law 2 is proven.

### Why DeMorgan's Laws Matter

1. **Gate conversion:** You can convert between AND/OR by adding inverters.
   If your chip only has NAND gates, DeMorgan tells you how to build OR.

2. **Simplification:** Complex boolean expressions can be simplified by
   applying DeMorgan's laws, reducing the number of gates needed.

3. **NAND/NOR equivalence:**
   - NAND(a, b) = NOT(AND(a, b)) = (NOT a) OR (NOT b)  [by Law 1]
   - NOR(a, b)  = NOT(OR(a, b))  = (NOT a) AND (NOT b) [by Law 2]

### Generalized DeMorgan's Laws

The laws extend to any number of inputs:

```
    NOT(A AND B AND C AND ... AND N) = (NOT A) OR (NOT B) OR ... OR (NOT N)
    NOT(A OR B OR C OR ... OR N)     = (NOT A) AND (NOT B) AND ... AND (NOT N)
```

"The negation of a conjunction is the disjunction of the negations" (and
vice versa). This generalizes to N inputs.

---

## 6. Sequential Logic — Memory from Feedback

Everything above is **combinational** logic: the output depends only on the
current inputs. Remove the inputs, and the output disappears. There is no
memory.

**Sequential** logic is fundamentally different. Sequential circuits can
**remember** their previous state. This is what makes computers possible —
without memory, there are no variables, no registers, no stored programs.

### The Key Insight: Feedback

Memory arises from **feedback** — wiring a gate's output back into its own
input. When you cross-couple two NOR gates (each feeding its output into
the other's input), you create a stable loop that "latches" into one of
two states and stays there.

### 6.1 SR Latch (Set-Reset Latch)

The simplest memory element. Built from just two NOR gates with cross-coupled
feedback.

**Circuit Diagram:**

```
                 ┌───────────┐
    Reset ──────>| NOR       |>───┬──── Q
                 └───────────┘    |
                       ^          |
                       |          |  feedback
                       |          |  (cross-coupling)
                       v          |
                 ┌───────────┐    |
    Set ────────>| NOR       |>───┴──── Q_bar
                 └───────────┘
```

Each NOR gate's output feeds into the OTHER NOR gate's input. This creates
two stable states:

**Truth Table:**

```
    S  R | Q     Q_bar | Action
    -----+--------------+----------------------------------
    0  0 | Q     Q_bar | HOLD -- remember previous state
    1  0 | 1       0   | SET -- store a 1
    0  1 | 0       1   | RESET -- store a 0
    1  1 | 0       0   | INVALID -- both forced low
```

**How it works step by step (setting the latch):**

1. Start in reset state: Q=0, Q_bar=1
2. Apply Set=1, Reset=0
3. Bottom NOR: NOR(1, Q=0) = NOR(1,0) = 0, so Q_bar becomes 0
4. Top NOR: NOR(0, Q_bar=0) = NOR(0,0) = 1, so Q becomes 1
5. Stable state reached: Q=1, Q_bar=0
6. Now remove Set (Set=0, Reset=0): the HOLD state
7. Top NOR: NOR(0, Q_bar=0) = 1. Q stays 1.
8. Bottom NOR: NOR(0, Q=1) = 0. Q_bar stays 0.
9. The latch **remembers** the 1 even though Set has been removed.

This is memory. The feedback loop sustains the state.

**Why S=R=1 is invalid:** Both NOR gates receive a 1, forcing both outputs
to 0. This violates the invariant that Q and Q_bar should be complements.
Releasing both simultaneously creates a race condition.

### 6.2 D Latch (Data Latch)

The SR latch has a problem: the caller must carefully avoid S=R=1. The
**D Latch** solves this by deriving S and R from a single data input D,
using a NOT gate to guarantee they are never both 1.

An **enable** signal controls when the latch listens:

```
                        ┌─────────┐
    Data ──┬───────────>| AND     |>── Set ──┐
           |            └─────────┘          |
           |                ^                |
    Enable─┼────────────────┤                |
           |                |           ┌─────────┐
           |            ┌───┘           | SR      |── Q
           |            |               | Latch   |
           └──>o── NOT ─┤               |         |── Q_bar
                        |               └─────────┘
                   ┌────v─────┐              |
                   | AND      |>── Reset ────┘
            Enable>|          |
                   └──────────┘

    S = AND(Data, Enable)
    R = AND(NOT(Data), Enable)
```

**Truth Table:**

```
    D  E | Q     Q_bar | Action
    -----+--------------+----------------------------------
    X  0 | Q     Q_bar | HOLD -- latch is opaque
    0  1 | 0       1   | Store 0 -- latch is transparent
    1  1 | 1       0   | Store 1 -- latch is transparent
```

When Enable=1, output follows input (transparent). When Enable=0, output
holds (opaque). The beauty is that S and R can never both be 1 simultaneously.

### 6.3 D Flip-Flop (Edge-Triggered Memory)

The D Latch is transparent the entire time Enable is high. In a pipeline,
this causes data to ripple through multiple stages uncontrollably. We need
to capture data at a **precise instant** — the clock edge.

The **D Flip-Flop** uses a **master-slave** configuration: two D Latches
in series with opposite enable signals.

```
    ┌─────────────────────────────────────────────────┐
    |                                                 |
    |  Data ──> [D Latch (Master)] ──> [D Latch (Slave)] ──> Q
    |                |                       |        |
    |           NOT(Clock)                Clock      |
    |                                                 |
    └─────────────────────────────────────────────────┘
```

**How it works:**

```
    Clock = 0:  Master is transparent (captures data)
                Slave is opaque (holds previous value)

    Clock = 1:  Master is opaque (holds captured data)
                Slave is transparent (outputs master's value)

    Rising edge (0 -> 1):
        Master closes (holds the data it just captured)
        Slave opens (passes master's value to output)
        -> Data captured at the EXACT moment of the rising edge
```

**Timing Diagram:**

```
    Clock:  ____/----\____/----\____
    Data:   ==X=============X======
                 ^               ^
                 |               |
            Captured here   Captured here

    Output: ----[old]------[new]----
                      ^
                      Changes on rising edge
```

**Why edge-triggering matters:** In a CPU pipeline, thousands of flip-flops
must all sample their inputs at the same instant. Edge-triggering ensures
this. Without it, data from one pipeline stage could leak into the next
before the current stage finishes — chaos.

### The Memory Hierarchy Built from Flip-Flops

```
    SR Latch           2 NOR gates           raw 1-bit memory
        |
        v
    D Latch            SR + AND + NOT        controlled 1-bit memory
        |
        v
    D Flip-Flop        2 D Latches           edge-triggered 1-bit memory
        |
        v
    Register           N flip-flops          N-bit word storage
        |               in parallel
        v
    Shift Register     chained flip-flops    serial-to-parallel conversion
        |
        v
    Counter            register + adder      binary counting
```

A 32-bit CPU register is literally 32 D flip-flops side by side, all sharing
the same clock signal. A modern GPU has millions of flip-flops organized
into register files.

---

## 7. The Python Implementation

The implementation lives at:

```
    code/packages/python/logic-gates/
    |-- src/logic_gates/
    |   |-- __init__.py
    |   |-- gates.py          # All 7 gates + NAND-derived versions + multi-input
    |   |-- sequential.py     # SR latch, D latch, D flip-flop, register, shift register, counter
```

### Design Decisions

**Gates as pure functions:** Each gate is a Python function that takes
integer inputs (0 or 1) and returns an integer output (0 or 1). No classes,
no objects, no state. This mirrors hardware: a gate is a pure function of
its inputs.

```python
    def AND(a: int, b: int) -> int:
        return 1 if a == 1 and b == 1 else 0
```

**Strict input validation:** The `_validate_bit()` function rejects booleans
(`True`/`False`), floats (`1.0`), and out-of-range integers. This catches
bugs where someone accidentally passes a comparison result instead of a
bit value.

**Composite gates call primitives:** `NAND(a, b)` is implemented as
`NOT(AND(a, b))`, directly mirroring the hardware. The NAND-derived gates
(`nand_not`, `nand_and`, `nand_or`, `nand_xor`) use ONLY the `NAND`
function, proving functional completeness in executable code.

**Multi-input via reduce:** `AND_N(*inputs)` uses Python's `functools.reduce`
to chain the 2-input `AND` across all inputs, exactly as hardware chains
gates.

**Sequential logic simulates feedback:** Since Python functions don't have
physical feedback loops, the SR latch simulates feedback with an iteration
loop that recomputes both NOR gates until the outputs stabilize
(convergence). For an SR latch, this always converges in 2-3 iterations.

### Running the Code

```bash
    cd code/packages/python/logic-gates
    pip install -e .
    python -c "from logic_gates import AND, OR, NOT; print(AND(1,1))"
    # Output: 1
```

### Running Tests

```bash
    cd code/packages/python/logic-gates
    pytest tests/ -v
```
