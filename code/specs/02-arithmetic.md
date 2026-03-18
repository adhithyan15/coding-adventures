# 02 — Arithmetic

## Overview

The arithmetic package builds number computation circuits from logic gates. Starting with a half adder (the simplest circuit that can add two bits), it builds up to a full ALU (Arithmetic Logic Unit) capable of addition, subtraction, comparison, and bitwise operations.

This is Layer 2 of the computing stack. It depends on the logic-gates package.

## Layer Position

```
Logic Gates → [YOU ARE HERE] → CPU → ARM → Assembler → Lexer → Parser → Compiler → VM
```

**Input from:** Logic gates (AND, OR, XOR, NOT).
**Output to:** CPU simulator (provides the ALU that the CPU uses for computation).

## Concepts

### Half Adder

Adds two single bits. Produces a sum and a carry.

```
A  B  │ Sum  Carry
──────┼───────────
0  0  │  0    0
0  1  │  1    0
1  0  │  1    0
1  1  │  0    1      ← 1+1=10 in binary (sum=0, carry=1)
```

Built from gates:
```
Sum   = XOR(A, B)
Carry = AND(A, B)
```

### Full Adder

Adds two bits plus a carry-in from a previous addition. This is what allows multi-bit addition.

```
A  B  Cin │ Sum  Cout
──────────┼──────────
0  0   0  │  0    0
0  0   1  │  1    0
0  1   0  │  1    0
0  1   1  │  0    1
1  0   0  │  1    0
1  0   1  │  0    1
1  1   0  │  0    1
1  1   1  │  1    1
```

Built from two half adders and an OR gate.

### Ripple Carry Adder

Chains N full adders together to add N-bit numbers. The carry output of each adder feeds into the carry input of the next. Called "ripple carry" because the carry signal ripples from least significant bit to most significant bit.

```
  A3 B3    A2 B2    A1 B1    A0 B0
   │  │     │  │     │  │     │  │
  ┌┴──┴┐   ┌┴──┴┐   ┌┴──┴┐   ┌┴──┴┐
  │ FA │◄──│ FA │◄──│ FA │◄──│ FA │◄── 0 (initial carry)
  └──┬─┘   └──┬─┘   └──┬─┘   └──┬─┘
     S3       S2       S1       S0
```

### ALU (Arithmetic Logic Unit)

The ALU takes two N-bit inputs, an operation code, and produces an N-bit output plus status flags. It is the computational heart of any CPU.

Operations:
- ADD: A + B
- SUB: A - B (implemented as A + NOT(B) + 1, using two's complement)
- AND: bitwise A AND B
- OR: bitwise A OR B
- XOR: bitwise A XOR B
- NOT: bitwise NOT A

Flags:
- **Zero**: result is all zeros
- **Carry**: addition overflowed the bit width
- **Negative**: most significant bit of result is 1 (in two's complement, this means negative)
- **Overflow**: signed overflow occurred

## Public API

```python
# Half adder
def half_adder(a: int, b: int) -> tuple[int, int]: ...
    # Returns (sum, carry)

# Full adder
def full_adder(a: int, b: int, carry_in: int) -> tuple[int, int]: ...
    # Returns (sum, carry_out)

# N-bit ripple carry adder
def ripple_carry_adder(a: list[int], b: list[int], carry_in: int = 0) -> tuple[list[int], int]: ...
    # a and b are lists of bits (LSB first), returns (sum_bits, carry_out)

# ALU
class ALU:
    def __init__(self, bit_width: int = 8) -> None: ...

    def execute(self, op: ALUOp, a: list[int], b: list[int]) -> ALUResult: ...

class ALUOp(Enum):
    ADD = "add"
    SUB = "sub"
    AND = "and"
    OR  = "or"
    XOR = "xor"
    NOT = "not"

@dataclass
class ALUResult:
    value: list[int]   # result bits
    zero: bool         # is result zero?
    carry: bool        # did addition overflow?
    negative: bool     # is result negative (MSB=1)?
    overflow: bool     # signed overflow?
```

## Data Flow

```
Input:  two lists of bits (each 0 or 1), an operation code
Output: a list of bits (result) and status flags
```

All bit lists are LSB-first (least significant bit at index 0).

## Test Strategy

- Half adder: exhaustive truth table (4 cases)
- Full adder: exhaustive truth table (8 cases)
- Ripple carry adder: test with known decimal values converted to binary (e.g., 5 + 3 = 8)
- ALU: test each operation with boundary cases (0+0, max+1 overflow, positive-negative, etc.)
- Verify subtraction works via two's complement (A - B = A + NOT(B) + 1)
- Verify all flags are set correctly

## Future Extensions

- **Carry lookahead adder**: Faster than ripple carry — computes carries in parallel
- **Multiplier**: Built from adders and shift operations
- **Divider**: More complex, involves iterative subtraction
- **Barrel shifter**: Shift bits left/right by arbitrary amounts in one step
