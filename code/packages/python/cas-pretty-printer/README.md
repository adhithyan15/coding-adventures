# cas-pretty-printer

Pretty-print symbolic IR back to source text. The package walks an
`IRNode` tree and emits a string. The walker is **shared across every
dialect**; what changes per dialect is a small `Dialect` object that
supplies operator spellings, function names, brackets, and a
precedence table.

## Quick start

```python
from cas_pretty_printer import pretty, MacsymaDialect
from symbolic_ir import IRSymbol, IRInteger, IRApply, ADD, POW

x = IRSymbol("x")
expr = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))

print(pretty(expr, MacsymaDialect()))
# x^2 + 1
```

## Dialects shipped

- `MacsymaDialect()` — Maxima/MACSYMA syntax: `x^2`, `f(x, y)`,
  `[a, b]`, `sin`/`cos`/`log`/`exp`.
- `MathematicaDialect()` — Mathematica syntax: `x^2`, `Sin[x, y]`,
  `{a, b}`.
- `MapleDialect()` — Maple syntax.
- `LispDialect()` — always-prefix debugging form: `(Add x y)`. Useful
  when building a new dialect.

## Custom dialects

Sub-class `BaseDialect` and override the spellings you care about.
Most dialects need fewer than 30 lines. See `mathematica.py` and
`maple.py` for examples.

## Operator precedence

The walker tracks a minimum-precedence parameter as it descends.
Parentheses are inserted only when a child's precedence is lower than
the context demands. Right-associativity for `Pow` is handled
correctly: `a^b^c` → `Pow(a, Pow(b, c))` round-trips without spurious
parens; `Pow(Pow(a, b), c)` prints as `(a^b)^c`.

## Sugar

Each dialect can register surface-syntax sugar:

- `Add(x, Neg(y))` → `x - y`
- `Mul(x, Inv(y))` → `x / y`
- `Mul(-1, x)`     → `-x`

Plain dialects (`LispDialect`) skip the sugar and print every node
verbatim.

## Extending the printer with new heads

Downstream packages that introduce new heads (e.g. `cas-matrix` adds
`Matrix`, `Determinant`, `Inverse`) can register custom formatters:

```python
from cas_pretty_printer import register_head_formatter

def format_matrix(node, dialect, fmt):
    rows = [", ".join(fmt(c) for c in r.args) for r in node.args]
    return "matrix(" + ", ".join(f"[{row}]" for row in rows) + ")"

register_head_formatter("Matrix", format_matrix)
```

The walker calls registered formatters before its built-in dispatch
when a head is recognized.

## Dependencies

- `coding-adventures-symbolic-ir`

That's it. The pretty-printer has no other dependencies — it's
deliberately a leaf in the package graph so any layer above can pull
it in cheaply.
