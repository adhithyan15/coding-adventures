# Logic Gates

**Layer 1 of the computing stack** — the foundation of all digital logic.

## What this package does

Implements the fundamental logic gates that every digital circuit is built from:

| Gate | Description |
|------|-------------|
| NOT  | Inverts input: 0→1, 1→0 |
| AND  | Output 1 only if both inputs are 1 |
| OR   | Output 1 if either input is 1 |
| XOR  | Output 1 if inputs differ |
| NAND | NOT(AND) — functionally complete |
| NOR  | NOT(OR) |
| XNOR | NOT(XOR) — output 1 if inputs are same |

Also includes NAND-derived implementations (all gates built from NAND only) and multi-input variants.

## Where it fits

```
[Logic Gates] → Arithmetic → CPU → ARM → Assembler → Lexer → Parser → Compiler → VM
```

This package is used by the **arithmetic** package to build half adders, full adders, and the ALU.

## Installation

```bash
uv add coding-adventures-logic-gates
```

## Usage

```python
from logic_gates import AND, OR, NOT, XOR

AND(1, 1)  # 1
AND(1, 0)  # 0
OR(0, 1)   # 1
NOT(1)     # 0
XOR(1, 0)  # 1
```

## Spec

See [01-logic-gates.md](../../../specs/01-logic-gates.md) for the full specification.
