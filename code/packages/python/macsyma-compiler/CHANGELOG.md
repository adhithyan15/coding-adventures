# Changelog

## 0.8.0 — 2026-04-28

**Phase 15 — Reciprocal hyperbolic compiler mappings: `coth`, `sech`, `csch`.**

Three entries added to `_STANDARD_FUNCTIONS` in `compiler.py`:

```python
"coth": COTH,
"sech": SECH,
"csch": CSCH,
```

These mappings follow the same pattern as `"sinh": SINH` etc. from Phase 13.
`COTH`, `SECH`, `CSCH` are imported from `symbolic_ir` 0.8.0, which adds the
three new `IRSymbol` head singletons.

Depends on `symbolic-ir >= 0.8.0`.

---

## 0.7.0 — 2026-04-27

**Phase 13 — Hyperbolic function compiler mappings.**

No new compiler logic was needed — `"sinh": SINH`, `"cosh": COSH`,
`"tanh": TANH`, `"asinh": ASINH`, `"acosh": ACOSH`, and `"atanh": ATANH`
were already added to `_STANDARD_FUNCTIONS` in 0.5.0. This release bumps the
version to align with Phase 13 of the symbolic VM (0.32.0), and updates the
`symbolic-ir` dependency to `>=0.7.0`.

## 0.6.0 — 2026-04-27

**Phase G — Control-flow AST → IR compilation.**

Added six new compiler methods and registered them in the `_handlers`
dispatch table:

- `_compile_if_expr` — `if c then t [elseif c2 then t2] [else e]` →
  right-nested chain of `If(cond, then[, else])` IR nodes.
- `_compile_for_each_expr` — `for x in list do body` →
  `ForEach(x, list, body)`.
- `_compile_for_range_expr` — `for x [: a] [step s] (thru|while|unless) b do body` →
  `ForRange(x, a, s, b, body)` (defaults: `start=1`, `step=1`).
- `_compile_while_expr` — `while c do body` → `While(c, body)`.
- `_compile_block_expr` — `block([x: 0, y], s1, s2, …)` →
  `Block(List(…), s1, …)`. Prepends `List()` when no explicit locals list.
- `_compile_return_expr` — `return(expr)` → `Return(expr)`.

Also imported `BLOCK`, `FOR_EACH`, `FOR_RANGE`, `IF`, `RETURN`, `WHILE`
from `symbolic_ir` and bumped `symbolic-ir` dependency to `>=0.6.0`.

Grammar changes required: `macsyma.grammar` adds the six new productions;
`macsyma.tokens` adds the `unless` keyword.

## 0.5.0 — 2026-04-23

- Added `"sinh": SINH`, `"cosh": COSH`, `"tanh": TANH`, `"asinh": ASINH`,
  `"acosh": ACOSH`, and `"atanh": ATANH` to `_STANDARD_FUNCTIONS` in
  `compiler.py`, mapping the MACSYMA names to their canonical IR heads.
  Required by Phase 13 so that `integrate(sinh(x), x)`, `integrate(x*cosh(x), x)`,
  `integrate(asinh(x), x)`, etc. are correctly compiled before evaluation.
  Depends on `coding-adventures-symbolic-ir >= 0.5.0` which introduces the
  SINH/COSH/TANH/ASINH/ACOSH/ATANH heads.

## 0.4.0 — 2026-04-23

- Added `"asin": ASIN` and `"acos": ACOS` to `_STANDARD_FUNCTIONS` in `compiler.py`,
  mapping the MACSYMA names `asin`/`acos` to `IRSymbol("Asin")`/`IRSymbol("Acos")`.
  Required by `symbolic-vm` Phase 12 so that `integrate(asin(ax+b), x)` and
  `integrate(acos(ax+b), x)` are correctly compiled before evaluation.
  Depends on `coding-adventures-symbolic-ir >= 0.4.0` which introduces the ASIN/ACOS heads.

## 0.3.0 — 2026-04-22

- Added `"atan": ATAN` to `_STANDARD_FUNCTIONS` in `compiler.py`, mapping
  the MACSYMA name `atan` to `IRSymbol("Atan")`. Required by
  `symbolic-vm` Phase 9 so that `integrate(atan(ax+b), x)` is correctly
  compiled to `Integrate(Atan(...), x)` before evaluation.

## 0.2.0 — 2026-04-20

- Added `"tan": TAN` to `_STANDARD_FUNCTIONS` in `compiler.py`, mapping
  the MACSYMA name `tan` to `IRSymbol("Tan")`. Depends on
  `coding-adventures-symbolic-ir >= 0.3.0` which introduces the `TAN`
  head.

## 0.1.0 — 2026-04-19

Initial release.

- Compiles parsed MACSYMA ASTs to `symbolic_ir` trees.
- Flattens the grammar's precedence cascade into uniform
  `IRApply(head, args)` nodes.
- Rewrites standard MACSYMA functions (`diff`, `integrate`, `sin`,
  `cos`, `log`, `exp`, `sqrt`) to canonical IR heads.
- Distinguishes `:` (Assign) from `:=` (Define), and shapes function
  definitions as `Define(name, List(params), body)`.
- Flattens `and`/`or` chains into variadic `IRApply` forms.
