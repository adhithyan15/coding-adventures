# Changelog

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
