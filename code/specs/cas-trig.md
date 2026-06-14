# cas-trig — Trigonometric Simplification and Expansion

> **Status**: New spec. Implements `TrigSimplify`, `TrigExpand`, and
> `TrigReduce` heads. Parent: `symbolic-computation.md`. Depends on
> `cas-simplify` and `cas-pattern-matching`.

## Why this package exists

`cas-simplify` deliberately excludes trig-specific rewriting (see its
"Future extensions" note) because the rule set is large and qualitatively
different from algebraic identities. Trig simplification is its own
mathematical discipline — Pythagorean identities, angle-addition formulas,
power-reduction formulas, co-function identities — and deserves a dedicated
home so the rule database can be extended, tested, and reasoned about in
isolation.

## Reuse story

All CAS frontends map to the same three heads:

| MACSYMA              | Mathematica           | Maple                   | IR head        |
|----------------------|-----------------------|-------------------------|----------------|
| `trigsimp(expr)`     | `TrigSimplify[expr]`  | `simplify(expr, trig)`  | `TrigSimplify` |
| `trigexpand(expr)`   | `TrigExpand[expr]`    | `expand(expr, trig)`    | `TrigExpand`   |
| `trigreduce(expr)`   | `TrigReduce[expr]`    | `combine(expr, trig)`   | `TrigReduce`   |

MACSYMA also has `trigfactor`, `half_angle`, `exponentialize`. These map
to the same heads with option flags, deferred to Phase 2.

## Scope

In:

- `TrigSimplify(expr)` — rewrite using Pythagorean identities until no
  further simplification is possible. Includes:
  - `sin²x + cos²x → 1` (and variants: `1 - sin²x → cos²x`, etc.)
  - `tan(x) → sin(x)/cos(x)` and its inverse when beneficial.
  - `sin(-x) → -sin(x)`, `cos(-x) → cos(x)`.
  - `sin(n·π) → 0`, `cos(n·π) → (-1)^n` for integer `n`.
  - Special values: `sin(π/6) → 1/2`, `cos(π/4) → √2/2`, etc.
- `TrigExpand(expr)` — expand compound angles and products to sums:
  - `sin(a + b) → sin(a)cos(b) + cos(a)sin(b)`
  - `cos(a + b) → cos(a)cos(b) - sin(a)sin(b)`
  - `sin(2x)   → 2·sin(x)·cos(x)`
  - `cos(2x)   → cos²(x) - sin²(x)`
  - `sin(n·x)` / `cos(n·x)` via Chebyshev recurrence for integer `n`.
- `TrigReduce(expr)` — reduce powers of trig functions to multiple
  angles (the inverse of `TrigExpand`):
  - `sin²(x) → (1 - cos(2x))/2`
  - `cos²(x) → (1 + cos(2x))/2`
  - `sin³(x) → (3·sin(x) - sin(3x))/4`
  - General `sinⁿ`, `cosⁿ` via the Chebyshev expansion.
  - `sin(x)·cos(x) → sin(2x)/2`

Out:

- `exponentialize` — write trig functions as complex exponentials
  (`e^{ix}`). Deferred; requires `cas-complex`.
- `half_angle` — half-angle formula substitution. Phase 2.
- `trigfactor` — product-to-sum rewriting. Phase 2.
- Hyperbolic trig (`sinh`, `cosh`, `tanh`) identities — separate phase
  (integration Phase 13 already adds hyperbolic IR heads; simplification
  rules follow the same pattern but are deferred here to keep scope clear).

## Public interface

```python
from cas_trig import (
    trig_simplify,    # IRNode → IRNode
    trig_expand,      # IRNode → IRNode
    trig_reduce,      # IRNode → IRNode
    build_trig_handler_table,  # () → dict[str, Handler]
)
```

These are pure functions that take and return IR nodes. They never modify
state. `build_trig_handler_table()` is called by `SymbolicBackend.__init__`
alongside `build_cas_handler_table()` so every frontend inherits the heads.

## Heads added

| Head            | Arity | Meaning                                      |
|-----------------|-------|----------------------------------------------|
| `TrigSimplify`  | 1     | Apply Pythagorean and special-value rules.   |
| `TrigExpand`    | 1     | Expand sums/multiples to products of singles.|
| `TrigReduce`    | 1     | Reduce powers to multiple-angle form.        |

`Sin`, `Cos`, `Tan`, `Csc`, `Sec`, `Cot` and their inverses already
exist as standard IR heads (used by the integration package since
Phase 4). This package adds only the three transformation heads above.

## Algorithm

### TrigSimplify

Fixed-point loop identical in structure to `cas_simplify.simplify`:

```python
def trig_simplify(expr: IRNode) -> IRNode:
    last = None
    while last != expr:
        last = expr
        expr = cas_simplify.canonical(expr)
        expr = _apply_trig_rules(expr)
        expr = cas_simplify.numeric_fold(expr)
    return expr
```

`_apply_trig_rules` runs the rule database through `cas-pattern-matching`.
The rule database (`rules.py`) is a list of `(pattern, replacement)`
pairs. Rules are tried in priority order: specific rules (sin²+cos²=1)
before general rules (sin(-x)=-sin(x)).

Key Pythagorean pattern:

```
Add(Pow(Sin(x_), 2), Pow(Cos(x_), 2))  →  1
Add(Pow(Sin(x_), 2), z_)  when z_ contains Pow(Cos(x_), 2)  →  ...
```

Special-value lookup table maps `(function, argument_mod_2pi)` pairs to
exact IR values using `IRRational` and `IRApply(SQRT, ...)` for irrational
values like `√2/2`. The table covers `π/6, π/4, π/3, π/2` and their
multiples.

### TrigExpand

Recursive tree walk. On each `IRApply`:

- `Sin(Add(a, b))` → expand via angle-addition formula, recurse.
- `Cos(Add(a, b))` → expand via angle-addition formula, recurse.
- `Sin(Mul(n, x))` for integer `n` → Chebyshev recurrence:
  `sin(nx) = 2cos(x)sin((n-1)x) - sin((n-2)x)`.
- `Cos(Mul(n, x))` for integer `n` → analogous Chebyshev.
- All other nodes: recurse into args, return.

Result is simplified by a pass of `cas_simplify.canonical` to flatten
the resulting expression.

### TrigReduce

Recursive tree walk. On `Pow(Sin(x), n)` or `Pow(Cos(x), n)` for positive
integer `n`, use the binomial theorem on the complex exponential form
to derive the multiple-angle expansion, then convert back to real form.

For `n = 2, 3, 4` the formulas are hard-coded (fastest and clearest).
For `n ≥ 5` the general formula is computed symbolically using the
identity `sinⁿ(x) = (e^{ix} - e^{-ix})^n / (2i)^n` evaluated with the
binomial theorem, then simplified to real form. This requires `cas-complex`
at runtime if complex intermediate values appear; Phase 1 hard-codes up
to `n = 6` to avoid that dependency initially.

## MACSYMA name table entries

```python
# These go in macsyma_runtime/name_table.py
TRIG_NAME_TABLE = {
    "trigsimp":   IRSymbol("TrigSimplify"),
    "trigexpand": IRSymbol("TrigExpand"),
    "trigreduce": IRSymbol("TrigReduce"),
}
```

## Test strategy

### TrigSimplify
- `TrigSimplify(Add(Pow(Sin(x), 2), Pow(Cos(x), 2))) = 1`.
- `TrigSimplify(Sub(1, Pow(Sin(x), 2))) = Pow(Cos(x), 2)`.
- `TrigSimplify(Sin(Neg(x))) = Neg(Sin(x))`.
- `TrigSimplify(Sin(Pi)) = 0`.
- `TrigSimplify(Cos(Mul(Pi, 2))) = 1`.
- Special values: `Sin(Div(Pi, 6)) = Rational(1, 2)`.
- Idempotent: applying twice gives the same result.
- Non-trig expressions pass through unchanged.

### TrigExpand
- `TrigExpand(Sin(Add(a, b))) = Add(Mul(Sin(a),Cos(b)), Mul(Cos(a),Sin(b)))`.
- `TrigExpand(Cos(Add(a, b))) = Sub(Mul(Cos(a),Cos(b)), Mul(Sin(a),Sin(b)))`.
- `TrigExpand(Sin(Mul(2, x))) = Mul(2, Sin(x), Cos(x))`.
- `TrigExpand(Cos(Mul(3, x)))` matches the triple-angle formula.
- Nested: `TrigExpand(Sin(Add(Mul(2,x), y)))` fully expands.

### TrigReduce
- `TrigReduce(Pow(Sin(x), 2)) = Mul(Rational(1,2), Sub(1, Cos(Mul(2,x))))`.
- `TrigReduce(Pow(Cos(x), 2)) = Mul(Rational(1,2), Add(1, Cos(Mul(2,x))))`.
- `TrigReduce(Mul(Sin(x), Cos(x))) = Mul(Rational(1,2), Sin(Mul(2,x)))`.
- `TrigReduce(Pow(Sin(x), 3))` matches the triple-angle formula.
- Round-trip: `TrigSimplify(TrigExpand(TrigReduce(expr)))` equals the
  original on polynomial-trig expressions.

Coverage: ≥85%.

## Package layout

```
code/packages/python/cas-trig/
  src/cas_trig/
    __init__.py
    rules.py              # Pythagorean + special-value rule database
    simplify.py           # TrigSimplify fixed-point loop
    expand.py             # TrigExpand recursive rewriter
    reduce.py             # TrigReduce power-to-multiple-angle
    special_values.py     # sin/cos/tan at rational multiples of pi
    handlers.py           # build_trig_handler_table()
    py.typed
  tests/
    test_trig_simplify.py
    test_trig_expand.py
    test_trig_reduce.py
    test_special_values.py
```

## Dependencies

`coding-adventures-symbolic-ir`,
`coding-adventures-cas-pattern-matching`,
`coding-adventures-cas-simplify`.

## Future extensions

- Phase 2: `TrigFactor` (product-to-sum), `HalfAngle`, `Exponentialize`
  (writes trig as complex exponentials; depends on `cas-complex`).
- Hyperbolic analogs: `HypSimplify`, `HypExpand` using `cosh²-sinh²=1`.
- Integration into `cas-simplify`'s `Simplify` head as an optional
  sub-pass gated by a `trig=True` option.
