# Arithmetic

**Layer 2 of the computing stack** — builds number computation from logic gates.

## What this package does

Implements the arithmetic circuits that perform binary addition and computation:

| Component | Description |
|-----------|-------------|
| Half Adder | Adds two single bits, produces sum and carry |
| Full Adder | Adds two bits plus carry-in, produces sum and carry-out |
| Ripple Carry Adder | Chains full adders to add multi-bit numbers |
| ALU | Arithmetic Logic Unit — performs add, subtract, AND, OR, XOR |

## Where it fits

```
Logic Gates → [Arithmetic] → CPU → ARM → Assembler → Lexer → Parser → Compiler → VM
```

This package is used by the **cpu-simulator** package to build the processing unit.

## Installation

```bash
uv add coding-adventures-arithmetic
```

## Usage

```python
from arithmetic import half_adder, full_adder, ripple_carry_adder

half_adder(1, 1)           # (sum=0, carry=1)
full_adder(1, 1, 1)        # (sum=1, carry=1)
ripple_carry_adder([1,0,1,0], [0,1,1,0])  # 8-bit addition
```

## Spec

See [02-arithmetic.md](../../../specs/02-arithmetic.md) for the full specification.
