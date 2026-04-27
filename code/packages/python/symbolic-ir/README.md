# symbolic-ir

The universal symbolic expression IR — the shared tree representation
that every computer-algebra-system frontend compiles to and every CAS
backend consumes.

## What this package is

Six immutable node types that can represent any symbolic expression:

| Node | Purpose | Example |
|------|---------|---------|
| `IRSymbol(name)` | Named atom (variable, constant, or operation head) | `x`, `Pi`, `Add` |
| `IRInteger(value)` | Arbitrary-precision integer | `42`, `-7`, `10**200` |
| `IRRational(n, d)` | Exact fraction, always reduced | `1/2`, `-3/7` |
| `IRFloat(value)` | Double-precision float | `3.14` |
| `IRString(value)` | String literal | `"hello"` |
| `IRApply(head, args)` | Compound expression: `head(args...)` | `Add(x, 1)` |

Every compound expression is an `IRApply`. The head is always an
`IRSymbol` naming an operation. This Lisp-like uniformity is what lets a
single tree walker handle expressions from any CAS dialect.

## Why this exists

CAS systems like MACSYMA, Mathematica, Maple, and REDUCE all represent
expressions as trees with a head and arguments. Only the surface syntax
differs. By compiling every frontend to this single IR, we can share
every downstream operation (simplification, differentiation, evaluation,
rendering) across all dialects.

## Example

Representing the polynomial `x^2 + 2*x + 1`:

```python
from symbolic_ir import ADD, MUL, POW, IRApply, IRInteger, IRSymbol

x = IRSymbol("x")
expr = IRApply(ADD, (
    IRApply(POW, (x, IRInteger(2))),
    IRApply(MUL, (IRInteger(2), x)),
    IRInteger(1),
))
```

The uniform `IRApply(head, args)` shape means any tree walker — a
simplifier, a pretty-printer, a numeric evaluator — has exactly one
case to handle for compound expressions.

## Design choices

- **Frozen dataclasses.** Every node is immutable and hashable, so
  nodes can be dict keys (essential for rewrite-rule caches) and safely
  shared across threads.
- **No auto-canonicalization.** `IRApply(Add, (x, 1))` and
  `IRApply(Add, (1, x))` are distinct. Canonicalization is the VM's
  job — different backends may have different notions of "canonical."
- **Standard head singletons.** `ADD`, `MUL`, etc. are shared
  `IRSymbol` constants. Equality is by value (`==`), not identity,
  but using the singletons keeps equality checks cheap.

## Dependencies

None beyond the Python standard library.

## Next steps

- `symbolic-vm` — the generic tree-walking evaluator
- `macsyma-compiler` — compiles a parsed MACSYMA AST into this IR
