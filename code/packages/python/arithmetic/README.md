# Arithmetic

**Layer 2 of the computing stack** — builds number computation from logic gates.

## What is binary arithmetic?

Computers don't understand decimal numbers (0-9). They only understand binary — 0 and 1. To add two numbers, a computer breaks them into their binary digits (bits) and adds them one bit at a time, just like you add decimal numbers digit by digit with carrying.

```
  Decimal:  5 + 3 = 8
  Binary:   101 + 011 = 1000
```

This package builds addition from the ground up using only the logic gates from Layer 1.

## Worked example: 2 + 3 = 5

Let's trace how a computer adds 2 + 3, step by step.

### Step 1: Convert to binary

Each decimal number is represented as a sequence of bits (binary digits). Each bit position represents a power of 2:

```
Position:    3    2    1    0
Power of 2:  8    4    2    1
             ─    ─    ─    ─
Decimal 2:   0    0    1    0    →  0×8 + 0×4 + 1×2 + 0×1 = 2
Decimal 3:   0    0    1    1    →  0×8 + 0×4 + 1×2 + 1×1 = 3
```

### Step 2: Add column by column (right to left)

Just like decimal addition, we start from the rightmost column and work left, carrying when the sum exceeds what one digit can hold.

```
         Carry:  0    0    1    0
                 ─    ─    ─    ─
             A:  0    0    1    0    (2)
           + B:  0    0    1    1    (3)
                 ─    ─    ─    ─
        Result:  0    1    0    1    (5)
```

**Column 0 (rightmost):** A=0, B=1 → 0+1 = 1, carry 0
**Column 1:** A=1, B=1 → 1+1 = 10 in binary → sum=0, carry 1
**Column 2:** A=0, B=0, carry-in=1 → 0+0+1 = 1, carry 0
**Column 3:** A=0, B=0 → 0+0 = 0

Result: `0101` = 5. Correct!

### Step 3: How the hardware does it

Each column is processed by a **full adder** circuit. The adders are chained together — each one's carry output feeds into the next one's carry input:

```
    Column 3       Column 2       Column 1       Column 0
    A=0 B=0        A=0 B=0        A=1 B=1        A=0 B=1
     │   │          │   │          │   │          │   │
   ┌─┴───┴─┐      ┌─┴───┴─┐      ┌─┴───┴─┐      ┌─┴───┴─┐
   │  Full  │ carry│  Full  │ carry│  Full  │ carry│  Full  │
   │ Adder  │◄─────│ Adder  │◄─────│ Adder  │◄─────│ Adder  │◄── 0
   └───┬────┘  0   └───┬────┘  1   └───┬────┘  0   └───┬────┘
       │               │               │               │
     Sum=0           Sum=1           Sum=0           Sum=1

Result: 0    1    0    1  =  5  ✓
```

### What each full adder does internally

Let's zoom into Column 1 (A=1, B=1, carry-in=0):

```
    A=1 ──┐
           ├──→ XOR ──→ partial_sum=0 ──┐
    B=1 ──┘                              ├──→ XOR ──→ final_sum=0
           ┌──→ AND ──→ partial_carry=1  │
    A=1 ──┤                   Cin=0 ─────┘
    B=1 ──┘                              ┌──→ AND ──→ carry2=0
                          partial_sum=0 ─┤
                                 Cin=0 ──┘

    carry_out = OR(partial_carry=1, carry2=0) = 1

    Result: sum=0, carry_out=1  (correct: 1+1 = 10 in binary)
```

Every single operation above (XOR, AND, OR) is a logic gate from Layer 1.

## The circuits

### Half Adder — adding two bits

The simplest possible addition circuit. It adds two single bits and produces two outputs: a **sum** and a **carry**.

```
A  B  │  Sum  Carry
──────┼────────────
0  0  │   0     0     (0 + 0 = 0)
0  1  │   1     0     (0 + 1 = 1)
1  0  │   1     0     (1 + 0 = 1)
1  1  │   0     1     (1 + 1 = 10 in binary — sum is 0, carry the 1)
```

**How it's built:** The sum is `XOR(A, B)` and the carry is `AND(A, B)`. Just two logic gates.

**Why "half"?** Because it can only add two bits — it doesn't accept a carry-in from a previous addition. To chain multiple bits together, you need a full adder.

### Full Adder — adding two bits plus a carry

Extends the half adder by accepting a **carry-in** from the previous bit position. This is what lets us chain adders together for multi-bit numbers.

```
A  B  Cin │  Sum  Cout
──────────┼────────────
0  0   0  │   0     0
0  0   1  │   1     0
0  1   0  │   1     0
0  1   1  │   0     1
1  0   0  │   1     0
1  0   1  │   0     1
1  1   0  │   0     1
1  1   1  │   1     1
```

**How it's built:** Two half adders and an OR gate:
1. Half-add A and B → partial sum, partial carry
2. Half-add partial sum and carry-in → final sum, second carry
3. Final carry-out = OR(partial carry, second carry)

### Ripple Carry Adder — adding multi-bit numbers

To add two 8-bit numbers (like 5 + 3), we chain 8 full adders together. The carry output of each adder feeds into the carry input of the next one. The carry "ripples" from the least significant bit to the most significant bit.

```
     A3 B3      A2 B2      A1 B1      A0 B0
      │  │       │  │       │  │       │  │
    ┌─┴──┴─┐   ┌─┴──┴─┐   ┌─┴──┴─┐   ┌─┴──┴─┐
    │  FA  │◄──│  FA  │◄──│  FA  │◄──│  FA  │◄── 0 (initial carry)
    └──┬───┘   └──┬───┘   └──┬───┘   └──┬───┘
       S3         S2         S1         S0
```

**Example: 5 + 3 = 8**
```
  A = 0101  (5 in binary)
  B = 0011  (3 in binary)
  ─────────
  S = 1000  (8 in binary)
```

Each full adder processes one column, passing its carry to the next.

### ALU (Arithmetic Logic Unit) — the computational brain

The ALU is the part of a CPU that does actual computation. It takes two numbers, an operation code (add, subtract, AND, OR, XOR, NOT), and produces a result plus status flags.

**Operations:**
- **ADD**: A + B using the ripple carry adder
- **SUB**: A - B using two's complement (A + NOT(B) + 1)
- **AND**: Bitwise AND (useful for masking bits)
- **OR**: Bitwise OR (useful for setting bits)
- **XOR**: Bitwise XOR (useful for toggling bits)
- **NOT**: Bitwise NOT (flip all bits)

**Flags** (metadata about the result):
- **Zero**: Is the result all zeros?
- **Carry**: Did the addition overflow the bit width?
- **Negative**: Is the most significant bit 1? (means negative in two's complement)
- **Overflow**: Did signed arithmetic produce a wrong-sign result?

**What is two's complement?** It's how computers represent negative numbers. To negate a number: flip all bits (NOT), then add 1. For example, in 8-bit: `-3` = NOT(`00000011`) + 1 = `11111100` + 1 = `11111101`. This clever encoding makes subtraction the same as addition: `A - B = A + (-B) = A + NOT(B) + 1`.

## Where it fits

```
Logic Gates → [Arithmetic] → CPU → ARM/RISC-V → Assembler → Lexer → Parser → Compiler → VM
```

This package is used by the **cpu-simulator** to perform the actual computations during the fetch-decode-execute cycle.

## Installation

```bash
uv add coding-adventures-arithmetic
```

## Usage

```python
from arithmetic import half_adder, full_adder, ripple_carry_adder, ALU, ALUOp

# Half adder: 1 + 1 = 0 with carry 1 (binary 10)
half_adder(1, 1)  # (0, 1)

# Full adder: 1 + 1 + carry_in=1 = 1 with carry 1 (binary 11)
full_adder(1, 1, 1)  # (1, 1)

# Ripple carry: 5 + 3 = 8 (bits are LSB first)
a = [1, 0, 1, 0]  # 5 = 1*1 + 0*2 + 1*4 + 0*8
b = [1, 1, 0, 0]  # 3 = 1*1 + 1*2 + 0*4 + 0*8
result, carry = ripple_carry_adder(a, b)
# result = [0, 0, 0, 1]  → 8 = 0*1 + 0*2 + 0*4 + 1*8

# ALU: 1 + 2 = 3
alu = ALU(bit_width=8)
result = alu.execute(ALUOp.ADD, [1,0,0,0,0,0,0,0], [0,1,0,0,0,0,0,0])
# result.value = [1,1,0,0,0,0,0,0]  → 3
# result.zero = False, result.carry = False
```

## Spec

See [02-arithmetic.md](../../../specs/02-arithmetic.md) for the full specification.
