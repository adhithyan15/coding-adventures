# cas-simplify

Canonical form and identity-rule simplification for the symbolic IR.

## Quick start

```python
from cas_simplify import simplify, canonical
from symbolic_ir import IRApply, IRInteger, IRSymbol, ADD, MUL

x = IRSymbol("x")
expr = IRApply(MUL, (IRApply(ADD, (x, IRInteger(0))), IRInteger(1)))
simplify(expr)
# IRSymbol("x")
```

## Phase 1 scope

This release ships the foundational simplifier:

- **``canonical(expr)``** — pure structural normalization:
  - Flatten nested ``Add`` / ``Mul``: ``Add(a, Add(b, c))`` →
    ``Add(a, b, c)``.
  - Sort args of commutative heads (``Add``, ``Mul``) by a stable
    deterministic key.
  - Drop singleton variants: ``Add(x)`` → ``x``, ``Mul(x)`` → ``x``.
  - Drop empty containers: ``Add()`` → ``0``, ``Mul()`` → ``1``.
- **``simplify(expr)``** — fixed-point loop applying:
  1. Canonical pass.
  2. Numeric-fold (``Add(2, 3, x)`` → ``Add(5, x)``,
     ``Mul(2, 3, x)`` → ``Mul(6, x)``).
  3. Identity rules from a curated rule database (`x+0`, `x*1`,
     `x*0`, `x^0`, `x^1`, `Pow(0, x)`, `Pow(1, x)`,
     `Sub(x, x)`, `Div(x, x)`, ``Sin(0)``, ``Cos(0)``,
     ``Log(Exp(x))``, ``Exp(Log(x))``, ...).

Deferred to follow-up packages:

- ``Expand`` — polynomial / trig expansion (needs `polynomial-bridge`
  and decisions about what counts as "expanded").
- ``Collect`` — collect like terms in a variable.
- ``Together`` / ``Apart`` — common-denominator / partial-fraction
  helpers.
- Trig identities — full `cas-trig-simplify` package.

## Reuse story

Universal across CAS frontends — backs Maxima's ``simplify``,
Mathematica's ``Simplify[]``, Maple's ``simplify``, SymPy's
``simplify``. The canonical pass alone is also useful as a fast
"normalize for equality comparison" hook.

## Dependencies

- `coding-adventures-symbolic-ir`
- `coding-adventures-cas-pattern-matching`
