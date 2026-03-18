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

## Public API

```python
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
```

## Data Flow

```
Input:  one or two integers, each either 0 or 1
Output: one integer, either 0 or 1
```

Inputs outside {0, 1} should raise a ValueError with a clear message.

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

## Future Extensions

- **Gate delay simulation**: Model propagation delay through gates (useful for understanding timing in real circuits)
- **Circuit visualization**: Render gate diagrams
- **Gate count tracking**: Count how many primitive gates a complex circuit uses
