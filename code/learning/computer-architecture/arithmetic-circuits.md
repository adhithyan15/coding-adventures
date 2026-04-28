# Arithmetic Circuits — From Bits to Addition and Beyond

Computers are, at their core, fancy adding machines. Every arithmetic
operation — addition, subtraction, multiplication, division — is built from
the same logic gates described in `logic-gates.md`. This document shows how.

We start with binary number representation, build a half adder, chain it
into a full adder, connect full adders into a ripple carry adder, and then
assemble the ALU (Arithmetic Logic Unit) — the computational heart of every
CPU and GPU.

Reference implementation: `code/packages/python/arithmetic/`

---

## Table of Contents

1. [Binary Number Representation](#1-binary-number-representation)
2. [Half Adder](#2-half-adder)
3. [Full Adder](#3-full-adder)
4. [Ripple Carry Adder](#4-ripple-carry-adder)
5. [The ALU](#5-the-alu)
6. [Status Flags](#6-status-flags)
7. [How Subtraction Works](#7-how-subtraction-works)

---

## 1. Binary Number Representation

### Unsigned Integers

In binary, each digit (bit) represents a power of 2, just like each digit
in decimal represents a power of 10:

```
    Decimal:  4   7   3
              |   |   |
              v   v   v
           4x100 + 7x10 + 3x1 = 473

    Binary:   1   0   1   1
              |   |   |   |
              v   v   v   v
           1x8 + 0x4 + 1x2 + 1x1 = 11
```

An N-bit unsigned integer can represent values from 0 to 2^N - 1:

```
    Bits | Range          | Max Value
    -----+----------------+----------
     4   | 0 to 15        |     15
     8   | 0 to 255       |    255
    16   | 0 to 65535     |  65535
    32   | 0 to ~4 billion| 4294967295
```

### LSB-First vs MSB-First

There are two conventions for ordering bits:

```
    MSB-first (most significant bit first):  1011  = 8+0+2+1 = 11
    LSB-first (least significant bit first): 1101  = 1+0+4+8 = 13... wait, no.
```

The Python implementation in this repo uses **LSB-first** (index 0 = least
significant bit). This matches how addition works — you start from the
rightmost digit and carry left:

```
    LSB-first list:  [1, 1, 0, 1]
                      ^  ^  ^  ^
                     2^0 2^1 2^2 2^3
                      1 + 2 + 0 + 8 = 11
```

This convention makes the ripple carry adder loop natural: index 0 is
processed first, and the carry ripples to higher indices.

### Two's Complement (Signed Integers)

How do you represent negative numbers with only 0s and 1s? The most common
scheme is **two's complement**.

**Key idea:** The most significant bit (MSB) has a NEGATIVE weight.

For a 4-bit number:

```
    Bit pattern:  b3   b2   b1   b0
    Value:       -8*b3 + 4*b2 + 2*b1 + 1*b0
```

**Examples (4-bit two's complement):**

```
    Binary | Decimal | Explanation
    -------+---------+------------------------------
    0000   |    0    |  0
    0001   |    1    |  1
    0010   |    2    |  2
    0011   |    3    |  2 + 1
    0100   |    4    |  4
    0101   |    5    |  4 + 1
    0110   |    6    |  4 + 2
    0111   |    7    |  4 + 2 + 1 (maximum positive)
    1000   |   -8    |  -8 (minimum negative)
    1001   |   -7    |  -8 + 1
    1010   |   -6    |  -8 + 2
    1011   |   -5    |  -8 + 2 + 1
    1100   |   -4    |  -8 + 4
    1101   |   -3    |  -8 + 4 + 1
    1110   |   -2    |  -8 + 4 + 2
    1111   |   -1    |  -8 + 4 + 2 + 1
```

**How to negate a number in two's complement:**

```
    Step 1: Flip all bits (NOT)
    Step 2: Add 1

    Example: negate 5 (0101)
    Step 1: NOT(0101) = 1010
    Step 2: 1010 + 1  = 1011 = -5

    Check: -8 + 2 + 1 = -5. Correct!
```

**Why two's complement is brilliant:** Addition works the same for signed
and unsigned numbers. The hardware doesn't need to know (or care) whether
the numbers are signed. The same adder circuit handles both.

---

## 2. Half Adder

The **half adder** adds two single bits and produces a sum and a carry. It's
called "half" because it cannot handle a carry input from a previous
addition — it only adds two bits.

### Truth Table

```
    A  B | Sum  Carry | Explanation
    -----+-----+------+--------------------
    0  0 |  0    0    | 0 + 0 = 0
    0  1 |  1    0    | 0 + 1 = 1
    1  0 |  1    0    | 1 + 0 = 1
    1  1 |  0    1    | 1 + 1 = 10 (binary)
```

The last row is the interesting one: 1 + 1 = 2, which in binary is "10" —
a sum digit of 0 and a carry of 1.

### Gate Diagram

```
    a ──┬──\
        |  =)── Sum  (XOR: the sum digit)
    b ──┼──/
        |
        ├──┐
        |  |D── Carry (AND: carry when both are 1)
        └──┘
```

**Implementation:**

```
    Sum   = XOR(a, b)
    Carry = AND(a, b)
```

Notice how XOR naturally computes the sum digit (1 when inputs differ, 0
when same), and AND naturally computes the carry (1 only when both are 1).

### Why It's Called "Half"

A half adder can only add two bits. But in multi-bit addition, every column
(except the first) must also add the carry from the previous column:

```
       Carry:   1 1
    Number A:   1 0 1 1
    Number B: + 0 1 1 0
    ---------+----------
    Result:   1 0 0 0 1
```

Column 2 (from the right) needs to add A=1, B=1, AND carry=1 from column 1.
A half adder can only handle A and B — it's missing the carry input. That's
why we need the **full adder**.

---

## 3. Full Adder

The **full adder** adds three single bits: A, B, and a carry-in from a
previous stage. It produces a sum and a carry-out.

### Truth Table

```
    A  B  Cin | Sum  Cout | Decimal: A+B+Cin = ?
    ----------+-----+-----+---------------------
    0  0   0  |  0    0   |  0+0+0 = 0 (00)
    0  0   1  |  1    0   |  0+0+1 = 1 (01)
    0  1   0  |  1    0   |  0+1+0 = 1 (01)
    0  1   1  |  0    1   |  0+1+1 = 2 (10)
    1  0   0  |  1    0   |  1+0+0 = 1 (01)
    1  0   1  |  0    1   |  1+0+1 = 2 (10)
    1  1   0  |  0    1   |  1+1+0 = 2 (10)
    1  1   1  |  1    1   |  1+1+1 = 3 (11)
```

### Built from Two Half Adders and an OR Gate

```
    a ──┐                              ┌──\
        | Half    partial_sum ────────>|   )── carry_out
    b ──┘ Adder                        └──/
        |                              ^  OR
        | partial_carry ──────┐        |
        |                     |        |
        v                     v        |
    partial_sum ──┐           |        |
                  | Half      |        |
    carry_in ─────┘ Adder     └────────┘
                  |
                  v
                 sum        carry_2
```

Step by step:

```
    1. Half-add A and B:
       partial_sum   = XOR(A, B)
       partial_carry = AND(A, B)

    2. Half-add partial_sum and carry_in:
       sum    = XOR(partial_sum, carry_in)
       carry2 = AND(partial_sum, carry_in)

    3. Compute final carry-out:
       carry_out = OR(partial_carry, carry2)
```

**Why OR for the carry?** A carry-out occurs if EITHER the first half-adder
produced a carry (both A and B were 1) OR the second half-adder produced a
carry (partial sum + carry_in overflowed). At most one of these can be 1,
so OR correctly combines them.

### Detailed Gate Diagram

```
    a ──┬──\                          ┌──\
        |  =)── ps ──┬──\            |   )── Cout
    b ──┼──/          |  =)── Sum    └──/
        |             |  /             ^
        ├──┐   Cin ───┘               |
        |  |D── pc ──────────┐        |
        └──┘                 |──OR────┘
                Cin ──┐      |
                      |D──c2─┘
                ps ───┘

    ps = XOR(a, b)          pc = AND(a, b)
    Sum = XOR(ps, Cin)      c2 = AND(ps, Cin)
    Cout = OR(pc, c2)
```

---

## 4. Ripple Carry Adder

To add two N-bit numbers, we chain N full adders together. The carry-out
of each stage feeds into the carry-in of the next stage.

### 4-Bit Ripple Carry Adder

```
    A[0] B[0]    A[1] B[1]    A[2] B[2]    A[3] B[3]
      |    |       |    |       |    |       |    |
      v    v       v    v       v    v       v    v
    +--------+   +--------+   +--------+   +--------+
    |  Full  |   |  Full  |   |  Full  |   |  Full  |
    |  Adder |   |  Adder |   |  Adder |   |  Adder |
    |   #0   |   |   #1   |   |   #2   |   |   #3   |
    +--------+   +--------+   +--------+   +--------+
    Cin| |Cout   Cin| |Cout   Cin| |Cout   Cin| |Cout
       | |  ______/ |  ______/ |  ______/ |    |
     0-+ |/         |/         |/         |    +-> Carry_out
       v v          v          v          v
    S[0]          S[1]       S[2]       S[3]
```

The carry **ripples** from right to left, one stage at a time. This is why
it's called a "ripple" carry adder.

### Worked Example: 5 + 3

```
    A = 5 = 0101 in binary  ->  LSB-first: [1, 0, 1, 0]
    B = 3 = 0011 in binary  ->  LSB-first: [1, 1, 0, 0]

    Stage 0: A[0]=1, B[0]=1, Cin=0
             Sum=0, Cout=1          (1+1=10, write 0, carry 1)

    Stage 1: A[1]=0, B[1]=1, Cin=1
             Sum=0, Cout=1          (0+1+1=10, write 0, carry 1)

    Stage 2: A[2]=1, B[2]=0, Cin=1
             Sum=0, Cout=1          (1+0+1=10, write 0, carry 1)

    Stage 3: A[3]=0, B[3]=0, Cin=1
             Sum=1, Cout=0          (0+0+1=01, write 1, carry 0)

    Result: [0, 0, 0, 1] LSB-first = 1000 binary = 8 decimal
    Carry out: 0

    5 + 3 = 8. Correct!
```

### The "Ripple" Delay Problem

The carry must propagate through ALL N stages before the final sum is valid.
For an N-bit adder, the worst-case delay is N gate delays for the carry
chain.

```
    Time ──>

    Stage 0:  [computing]  done
    Stage 1:  [waiting...] [computing]  done
    Stage 2:  [waiting......] [computing]  done
    Stage 3:  [waiting.........] [computing]  done
                                              ^
                                              Final answer ready here
```

For a 32-bit adder, that's 32 gate delays. At ~100 picoseconds per gate,
that's 3.2 nanoseconds — slow for a modern CPU running at 5 GHz (200ps per
cycle).

**Faster alternatives exist** (carry-lookahead, carry-select, Kogge-Stone)
that compute carries in parallel, reducing the delay to O(log N). Our
implementation uses the ripple carry approach for simplicity and clarity.

### Implementation

See `code/packages/python/arithmetic/src/arithmetic/adders.py`:

```python
    def ripple_carry_adder(a, b, carry_in=0):
        sum_bits = []
        carry = carry_in
        for i in range(len(a)):
            sum_bit, carry = full_adder(a[i], b[i], carry)
            sum_bits.append(sum_bit)
        return sum_bits, carry
```

The loop mirrors the hardware chain: each iteration is one full adder stage,
and the carry variable "ripples" from one iteration to the next.

---

## 5. The ALU

The **ALU** (Arithmetic Logic Unit) is the computational core of a CPU. It
takes two N-bit inputs and an operation code, and produces an N-bit result
plus status flags.

### Supported Operations

```
    Operation | What it does            | Implementation
    ----------+-------------------------+------------------------------
    ADD       | A + B                   | Ripple carry adder
    SUB       | A - B                   | A + NOT(B) + 1 (two's complement)
    AND       | A AND B (bitwise)       | AND gate on each bit pair
    OR        | A OR B (bitwise)        | OR gate on each bit pair
    XOR       | A XOR B (bitwise)       | XOR gate on each bit pair
    NOT       | NOT A (bitwise)         | NOT gate on each bit of A
```

### ALU Block Diagram

```
                   A (N bits)     B (N bits)
                      |              |
                      v              v
                  +---+--------------+---+
                  |                      |
    Op code ----->|         ALU          |
                  |                      |
                  +---+------+-----------+
                      |      |
                      v      v
                  Result   Flags
                 (N bits)  (Z, C, N, V)
```

### Bitwise Operations

For AND, OR, XOR, and NOT, the ALU simply applies the gate to each
corresponding pair of bits independently:

```
    A = [1, 0, 1, 1]
    B = [1, 1, 0, 0]

    A AND B = [AND(1,1), AND(0,1), AND(1,0), AND(1,0)]
            = [1,        0,        0,        0]

    A OR B  = [OR(1,1),  OR(0,1),  OR(1,0),  OR(1,0)]
            = [1,        1,        1,        1]

    A XOR B = [XOR(1,1), XOR(0,1), XOR(1,0), XOR(1,0)]
            = [0,        1,        1,        1]

    NOT A   = [NOT(1),   NOT(0),   NOT(1),   NOT(1)]
            = [0,        1,        0,        0]
```

No carry propagation needed — each bit is independent.

---

## 6. Status Flags

After every ALU operation, four status flags are computed. These flags are
critical for conditional branching ("if the result was zero, jump to X").

### The Four Flags

**Zero (Z):** Is the result all zeros?

```
    Set when: every bit of the result is 0
    Hardware: NOR across all result bits (or AND of all NOT(result bits))

    Example: 5 - 5 = 0  -->  Z = 1
    Example: 5 - 3 = 2  -->  Z = 0
```

**Carry (C):** Did unsigned addition overflow?

```
    Set when: the final carry-out of the adder is 1
    Meaning: the result doesn't fit in N bits (unsigned overflow)

    Example (4-bit): 15 + 1 = 16, but 16 doesn't fit in 4 bits
        1111 + 0001 = 0000 with carry = 1  -->  C = 1
```

**Negative (N):** Is the result negative (in two's complement)?

```
    Set when: the MSB (most significant bit) of the result is 1
    In two's complement, MSB=1 means the number is negative

    Example (4-bit): 3 - 5 = -2  -->  result = 1110  -->  N = 1
    Example (4-bit): 5 - 3 =  2  -->  result = 0010  -->  N = 0
```

**Overflow (V):** Did signed addition overflow?

```
    Set when: adding two positive numbers gives a negative result,
              or adding two negative numbers gives a positive result

    Detection: V = (A_sign == B_sign) AND (result_sign != A_sign)

    Example (4-bit signed): 7 + 1 = 8, but max signed 4-bit is 7
        0111 + 0001 = 1000 = -8  -->  V = 1 (positive + positive = negative!)

    Example (4-bit signed): -8 + (-1) = -9, but min signed 4-bit is -8
        1000 + 1111 = 0111 = 7   -->  V = 1 (negative + negative = positive!)
```

### Flag Summary Table

```
    Flag | Stands For | Set When                          | Used For
    -----+-----------+-----------------------------------+---------------------
      Z  | Zero      | Result == 0                       | Equality checks
      C  | Carry     | Unsigned overflow                 | Unsigned comparisons
      N  | Negative  | MSB of result is 1                | Sign detection
      V  | Overflow  | Signed overflow                   | Signed comparisons
```

### How Conditional Branches Use Flags

```
    Condition    | Flags Test       | Meaning
    -------------+------------------+----------------------------
    Equal        | Z == 1           | A - B == 0, so A == B
    Not equal    | Z == 0           | A - B != 0, so A != B
    Unsigned <   | C == 0           | A - B underflowed (unsigned)
    Unsigned >=  | C == 1           | A - B didn't underflow
    Signed <     | N != V           | Negative XOR Overflow
    Signed >=    | N == V           | Not (Negative XOR Overflow)
```

The CPU compares two numbers by subtracting them and checking the flags.
The subtraction result itself is often discarded — only the flags matter.

---

## 7. How Subtraction Works

### The Two's Complement Trick

The ALU doesn't need a separate subtraction circuit. Instead, it reuses the
adder:

```
    A - B = A + (-B) = A + NOT(B) + 1
```

**Step by step:**

```
    1. Flip all bits of B:         NOT(B)
    2. Add 1:                      NOT(B) + 1 = -B (two's complement negation)
    3. Add A:                      A + NOT(B) + 1 = A - B
```

The "+1" is achieved by setting the initial carry-in of the ripple carry
adder to 1 instead of 0.

### Worked Example: 7 - 3 (4-bit)

```
    A = 7 = 0111
    B = 3 = 0011

    Step 1: NOT(B) = NOT(0011) = 1100
    Step 2: Set carry_in = 1 (this adds the +1)

    Ripple carry adder: A + NOT(B) with carry_in=1

        Stage 0: 1 + 0 + 1 = 10  ->  sum=0, carry=1
        Stage 1: 1 + 0 + 1 = 10  ->  sum=0, carry=1
        Stage 2: 1 + 1 + 1 = 11  ->  sum=1, carry=1
        Stage 3: 0 + 1 + 1 = 10  ->  sum=0, carry=1

    Result: 0100 = 4. Carry out = 1.

    7 - 3 = 4. Correct!
```

### Why This Is Elegant

The same adder hardware handles both addition and subtraction. The ALU just
needs a multiplexer to choose between B and NOT(B), and a way to set
carry_in to 1 for subtraction:

```
                     A          B
                     |          |
                     |    SUB?--+--[NOT]--+
                     |          |         |
                     |       [MUX]--------+
                     |          |
                     v          v
                 +---+----------+---+
                 |                  |
    carry_in --->|  Ripple Carry    |
    (SUB? 1:0)  |  Adder           |
                 |                  |
                 +--------+---------+
                          |
                        Result
```

When SUB=0: adder sees (A, B, carry_in=0) = A + B.
When SUB=1: adder sees (A, NOT(B), carry_in=1) = A + NOT(B) + 1 = A - B.

### Implementation

See `code/packages/python/arithmetic/src/arithmetic/alu.py`:

```python
    def _twos_complement_negate(bits):
        inverted = [NOT(b) for b in bits]
        one = [1] + [0] * (len(bits) - 1)
        return ripple_carry_adder(inverted, one)
```

The `ALU.execute()` method for subtraction:

```python
    elif op == ALUOp.SUB:
        neg_b, _ = _twos_complement_negate(b)
        value, carry_bit = ripple_carry_adder(a, neg_b)
```

---

## Further Reading

- **Faster adders:** Carry-lookahead adders compute all carries in parallel,
  reducing delay from O(N) to O(log N). The Kogge-Stone adder is the fastest
  practical design.

- **Multiplication:** Built from shift-and-add (like long multiplication),
  or using Wallace tree multipliers for speed.

- **Division:** Restoring and non-restoring division algorithms, or
  Newton-Raphson iterative methods.

- **Floating-point arithmetic:** See `code/learning/hardware/floating-point.md`
  for how these integer circuits are extended to handle real numbers.
