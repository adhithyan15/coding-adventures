# cas-substitution — `Subst` / `ReplaceAll` Substitution

> **Status**: New spec. Small Python package implementing the `Subst`
> head: replace a symbol or sub-expression with a value throughout an
> expression tree.
> Parent: `symbolic-computation.md`. Depends on `cas-pattern-matching`.

## Why this package exists

Substitution is the most-used operation in any CAS. Maxima's `subst`,
Mathematica's `/.` (`ReplaceAll`), Maple's `subs`, SymPy's `subs`,
Matlab's `subs` — they all do the same thing: walk an expression
tree and replace one sub-expression (or a name) with another.

The operation is small but central — every higher-level operation
(differentiation, integration, simplification, solving) calls it
internally.

## Reuse story

`subst` is dialect-agnostic: every CAS exposes some surface name for it
that maps to the same `Subst` head. New language frontends just add
their surface name to their runtime's name table.

## Scope

In:

- `Subst(value, var, expr)` — MACSYMA convention: replace every
  occurrence of `var` in `expr` with `value`.
- `Subst(equation, expr)` — replace `lhs(equation)` with `rhs(equation)`.
- `Subst(rule_or_list, expr)` — apply one or many rules.
- `ReplaceAll(expr, rule_or_list)` — Mathematica convention.
- Both leverage the matcher in `cas-pattern-matching`, so subst by a
  pattern (`Subst(Pattern("a", Blank()) + Pattern("b", Blank()),
  Sin(a)*Cos(b), expr)`) works out of the box.

Out:

- Sequential substitution rules — those route to `cas-simplify`'s
  `ReplaceRepeated`.
- Algebraic substitution (e.g., `subst(x = y + 1, x^2)` and have it
  notice that `x^2 = (y+1)^2`) — that's the same operation, just
  followed by `Expand`. Lives in `cas-simplify`.

## Public interface

```python
from cas_substitution import subst, replace_all, register_handlers

result = subst(
    value=IRInteger(2),
    var=IRSymbol("x"),
    expr=IRApply(POW, (IRSymbol("x"), IRInteger(2))),
)
# result is Pow(2, 2) — un-simplified; Simplify is a separate concern.
```

The package also exposes a `register_handlers(backend)` function that
installs the `Subst`, `ReplaceAll`, `Replace`, and `ReplaceRepeated`
handlers on a `SymbolicBackend` — call once at backend setup time.

## Algorithm

```python
def subst(value, var, expr) -> IRNode:
    if expr == var:
        return value
    if isinstance(expr, IRApply):
        head = subst(value, var, expr.head)
        args = tuple(subst(value, var, a) for a in expr.args)
        return IRApply(head, args)
    return expr  # leaf, not equal to var
```

For pattern-based subst, delegate to `cas-pattern-matching`'s
`apply_rule` at every subtree.

## Heads added

| Head              | Arity | Meaning                                   |
|-------------------|-------|-------------------------------------------|
| `Subst`           | 3     | `Subst(value, var, expr)` — MACSYMA form. |
| `ReplaceAll`      | 2     | `ReplaceAll(expr, rules)` — MMA form.     |
| `Replace`         | 2     | Single application of a rule.             |
| `ReplaceRepeated` | 2     | Apply rules until fixed point.            |

## Test strategy

- `subst(2, x, x^2 + 3*x + 1)` → `2^2 + 3*2 + 1` (un-simplified).
- `subst(2, x, sin(x))` → `sin(2)`.
- `subst(2, x, y)` → `y` (no occurrence).
- `subst([x = 2, y = 3], x + y)` → `2 + 3`.
- Pattern subst: `subst(Sin(a_)^2, 1 - Cos(a_)^2, ...)` rewrites the
  Pythagorean identity backwards.
- Coverage: ≥95%.

## Package layout

```
code/packages/python/cas-substitution/
  src/cas_substitution/
    __init__.py
    subst.py
    py.typed
  tests/
    test_subst_simple.py
    test_subst_equation.py
    test_subst_pattern.py
```

Dependencies: `coding-adventures-symbolic-ir`,
`coding-adventures-cas-pattern-matching`.
