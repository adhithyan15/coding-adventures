# Changelog

## 0.6.0 — 2026-04-27

**Phase G — Control-flow head symbols.**

Added five new IR head constants to `nodes.py` (after `RULE`) and exported
all five from `__init__.py`:

- `WHILE = IRSymbol("While")` — `While(condition, body)` loop.
- `FOR_RANGE = IRSymbol("ForRange")` — `for x: a step s thru b do body`
  (5-ary: var, start, step, end, body).
- `FOR_EACH = IRSymbol("ForEach")` — `for x in list do body`
  (3-ary: var, list, body).
- `BLOCK = IRSymbol("Block")` — local scope with statement sequence
  (`Block(locals_list, stmt1, …, stmtN)`).
- `RETURN = IRSymbol("Return")` — early exit from a block/loop
  (`Return(value)`).

Required by the MACSYMA grammar extensions spec (`macsyma-grammar-extensions.md`)
and implemented in `symbolic-vm` 0.31.0 / `macsyma-compiler` 0.6.0.

## 0.5.0 — 2026-04-23

- Added `SINH = IRSymbol("Sinh")`, `COSH = IRSymbol("Cosh")`,
  `TANH = IRSymbol("Tanh")`, `ASINH = IRSymbol("Asinh")`,
  `ACOSH = IRSymbol("Acosh")`, and `ATANH = IRSymbol("Atanh")` to the
  elementary-functions group in `nodes.py` (after `ACOS`) and exported all
  six from `__init__.py`. Required by Phase 13 of the symbolic integration
  roadmap (hyperbolic function evaluation, differentiation, and integration).
  See `phase13-hyperbolic.md`.

## 0.4.0 — 2026-04-22

- Added `ASIN = IRSymbol("Asin")` and `ACOS = IRSymbol("Acos")` to the
  elementary-functions group in `nodes.py` (after `ATAN`) and exported both
  from `__init__.py`. Required by Phase 12 of the symbolic integration
  roadmap (`∫ P(x)·asin(ax+b) dx` and `∫ P(x)·acos(ax+b) dx`). See
  `phase12-poly-asin-acos.md`.

## 0.3.0 — 2026-04-20

- Added `TAN = IRSymbol("Tan")` to the elementary-functions group in
  `nodes.py` (between `COS` and `SQRT`) and exported it from
  `__init__.py`. Required by Phase 5 of the symbolic integration
  roadmap (tan and trig-power antiderivatives). See
  `phase5-trig-powers.md`.

## 0.2.0 — 2026-04-20

- Added `ATAN = IRSymbol("Atan")` to the elementary-functions group in
  `nodes.py` and exported it from `__init__.py`. Required by Phase 2e
  of the symbolic integration roadmap (arctan antiderivatives for
  irreducible quadratic denominators). See `arctan-integral.md`.

## 0.1.0 — 2026-04-19

Initial release.

- Six immutable node types: `IRSymbol`, `IRInteger`, `IRRational`,
  `IRFloat`, `IRString`, `IRApply`.
- `IRRational` normalization (gcd reduction, sign in numerator,
  division-by-zero validation).
- Standard head symbols (`ADD`, `MUL`, `POW`, `D`, `Integrate`, etc.)
  as module-level singletons.
- Full test suite covering construction, equality, hashing,
  immutability, and nested-tree round trips.
