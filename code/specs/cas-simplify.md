# cas-simplify — Canonical Form, `Simplify`, and `Expand`

> **Status**: New spec. The general-purpose simplifier and expander.
> Parent: `symbolic-computation.md`. Depends on `cas-pattern-matching`,
> `cas-substitution`, and `polynomial-bridge`.

## Why this package exists

Right now the symbolic VM applies *local* identity rules (`x+0 → x`,
`x*1 → x`) inline inside handlers. That's the tip of the iceberg of
what users mean by "simplify". Real simplification has to:

- Put expressions into a canonical form (commutative ops sorted,
  duplicates collapsed, integer arithmetic folded).
- Apply general rewrite rules from a curated database (trig
  identities, log/exp identities, identities involving inverse
  functions).
- Polynomial expand and collect.
- Common subexpression discovery.
- Numeric collapse for groups of literals (`2 + 3 + x` → `5 + x`).

This package factors all of that into one place. It is the workhorse
behind any user-facing `simplify(expr)` call in any CAS frontend.

## Reuse story

Every CAS has a `Simplify`/`simplify`/`fullsimplify` operation. The
underlying algorithms — sort-then-fold, identity rewrites, polynomial
expansion — are shared. A `mathematica-runtime` would map `Simplify[]`
and `FullSimplify[]` to this package's heads.

## Scope

In:

- `Simplify(expr)` — apply identity rules and canonicalization to a
  fixed point.
- `Expand(expr)` — distribute multiplication over addition,
  expand powers `(a + b)^n` for non-negative integer `n`, and trig
  expansion (sum-to-product, double-angle), gated by an option.
- `Collect(expr, var)` — collect like terms in a polynomial.
- `Together(expr)` — combine over a common denominator.
- `Apart(expr)` — partial fractions (delegates to existing
  `polynomial-bridge`).
- The **canonical-form** machinery used by all of the above:
  - Argument sorting for `Add` and `Mul` (deterministic ordering).
  - Constant folding.
  - Identity application (`x + 0`, `x * 1`, `x ^ 0`, `x ^ 1`,
    `x * 0`, `0 ^ 0` → `1` or error per backend flag).

Out:

- Trig-specific rewriters (`TrigSimplify`, `TrigExpand`,
  `TrigReduce`) — future package `cas-trig-simplify` if needed.
- Radical denesting — future.
- Logarithm combine/expand — partly in scope (basic), full version is
  future.

## Public interface

```python
from cas_simplify import (
    simplify,
    expand,
    collect,
    together,
    apart,
    canonical,            # the canonical-form pass alone
    register_handlers,
)

simplify(IRApply(ADD, (IRSymbol("x"), IRInteger(0))))
# IRSymbol("x")

expand(IRApply(POW, (IRApply(ADD, (IRSymbol("x"), IRInteger(1))), IRInteger(2))))
# (x + 1)^2 → x^2 + 2*x + 1
```

## Heads added

| Head        | Arity | Meaning                                         |
|-------------|-------|-------------------------------------------------|
| `Simplify`  | 1     | Apply identity rules to fixed point.            |
| `Expand`    | 1     | Distribute multiplication / expand powers.      |
| `Collect`   | 2     | Collect like terms in a variable.               |
| `Together`  | 1     | Common denominator.                             |
| `Apart`     | 1     | Partial fractions (rational fns).               |
| `Canonical` | 1     | Just the canonical-form pass (no rules).        |

## Algorithm

`Simplify` is a fixed-point loop:

```python
def simplify(expr):
    last = None
    cur  = expr
    while last != cur:
        last = cur
        cur  = canonical(cur)
        cur  = apply_identity_rules(cur)
        cur  = numeric_fold(cur)
    return cur
```

`canonical` does the boring deterministic part: walks the tree
post-order, sorts the args of every `Add`/`Mul`, flattens nested
`Add`/`Mul`, drops single-arg variants (`Add(x) → x`).

`apply_identity_rules` runs a curated rule list through
`cas-pattern-matching`. Initial rule set:

- `Add(x_, 0) → x`
- `Add(x_, x_) → 2*x`
- `Mul(x_, 1) → x`
- `Mul(x_, 0) → 0`
- `Mul(x_, x_) → x^2`
- `Pow(x_, 0) → 1`
- `Pow(x_, 1) → x`
- `Pow(0, x_) → 0` (with `x ≠ 0`)
- `Pow(1, x_) → 1`
- `Sub(x_, x_) → 0`
- `Div(x_, x_) → 1` (with `x ≠ 0`)
- `Log(Exp(x_)) → x`
- `Exp(Log(x_)) → x`
- `Sin(0) → 0`, `Cos(0) → 1`, `Tan(0) → 0`
- `Sin(Pi) → 0`, `Cos(Pi) → -1`

`numeric_fold` walks the tree and collapses groups of integer/rational
literals: `Add(2, 3, x)` → `Add(5, x)`.

`Expand` uses `polynomial-bridge` to convert sub-expressions to
polynomial form, multiply, and convert back.

## Test strategy

- All identity rules fire individually.
- Fixed point reached on `((x + 0) * 1)^1` → `x`.
- Numeric fold: `Add(1, 2, 3)` → `6`.
- Sort + flatten: `Add(c, Add(a, b))` → `Add(a, b, c)` after
  canonicalization.
- `Expand((x+1)^3)` → `x^3 + 3*x^2 + 3*x + 1`.
- `Collect(a*x + b*x + c, x)` → `(a + b)*x + c`.
- `Together(1/x + 1/y)` → `(x + y)/(x*y)`.
- `Apart(1/((x-1)*(x+1)))` → `1/(2*(x-1)) - 1/(2*(x+1))`.
- Coverage: ≥85%.

## Package layout

```
code/packages/python/cas-simplify/
  src/cas_simplify/
    __init__.py
    canonical.py         # canonical-form pass
    rules.py             # curated identity rules
    simplify.py          # the fixed-point loop
    expand.py            # polynomial / trig expansion
    collect.py
    together_apart.py
    py.typed
  tests/
    test_canonical.py
    test_simplify.py
    test_expand.py
    test_collect.py
    test_together_apart.py
```

Dependencies: `coding-adventures-symbolic-ir`,
`coding-adventures-cas-pattern-matching`,
`coding-adventures-cas-substitution`,
`coding-adventures-polynomial`,
`coding-adventures-polynomial-bridge`.

## Future extensions

- A separate `cas-trig-simplify` for trig identities.
- A separate `cas-radical-denesting` for `sqrt(3 + 2*sqrt(2)) → 1 + sqrt(2)`.
- Cost-function-driven simplification (Mathematica's `FullSimplify`
  picks the smallest result by an internal cost metric).
