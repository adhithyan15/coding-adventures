# Changelog

## 0.8.0 — 2026-04-28

**Add `Coth`, `Sech`, `Csch` head symbols — reciprocal hyperbolic functions (Phase 15).**

Three new `IRSymbol` singletons in `nodes.py`, exported from `__init__.py`:

- `COTH = IRSymbol("Coth")` — hyperbolic cotangent
- `SECH = IRSymbol("Sech")` — hyperbolic secant
- `CSCH = IRSymbol("Csch")` — hyperbolic cosecant

These are first-class heads (not expressed as `Inv(Sinh(...))` etc.) so that
evaluation handlers can short-circuit numerically and the differentiator can
emit compact symbolic derivatives using the existing `Sinh`/`Cosh` heads.

---

## 0.7.6 — 2026-04-28

**Add `Groebner`, `PolyReduce`, `IdealSolve` head symbols (cas-multivariate).**

- `GROEBNER = IRSymbol("Groebner")` — Gröbner basis computation head
- `POLY_REDUCE = IRSymbol("PolyReduce")` — polynomial reduction head
- `IDEAL_SOLVE = IRSymbol("IdealSolve")` — polynomial system solver head

All three exported from `symbolic_ir.__init__`.

---

## 0.7.5 — 2026-04-27

**Add `ALG_FACTOR` head symbol for algebraic-extension factoring.**

Added `ALG_FACTOR = IRSymbol("AlgFactor")` in the new "Algebraic factoring"
group at the bottom of `nodes.py`, and exported it from `__init__.py`.

Required by `cas-algebraic` 0.1.0 and `symbolic-vm` 0.32.7, which implement
`algfactor(poly, sqrt(d))` — factoring of univariate polynomials over
quadratic algebraic extensions Q[√d].

---

## 0.7.4 — 2026-04-27

**Add `ODE2`, `C_CONST`, `C1`, `C2` head symbols for ODE solving.**

Added four new IR head constants to `nodes.py` for the `cas-ode` package (D3):

- `ODE2 = IRSymbol("ODE2")` — head for `ode2(eqn, y, x)` ODE solver.
- `C_CONST = IRSymbol("%c")` — first-order ODE integration constant.
- `C1 = IRSymbol("%c1")` — first integration constant for 2nd-order ODEs.
- `C2 = IRSymbol("%c2")` — second integration constant for 2nd-order ODEs.

All four are exported from `__init__.py`.

---

## 0.7.3 — 2026-04-27

**Add `FOURIER`, `IFOURIER` head symbols for Fourier transforms.**

Added two new IR head constants to the "Laplace / Fourier transforms" group at the bottom
of `nodes.py`, and exported both from `__init__.py`:

- `FOURIER = IRSymbol("Fourier")` — forward Fourier transform head F{f(t)}
- `IFOURIER = IRSymbol("IFourier")` — inverse Fourier transform head F⁻¹{F(ω)}

Required by `cas-fourier` 0.1.0 and `symbolic-vm` 0.32.4.

---

## 0.7.2 — 2026-04-27

**Add `DIRAC_DELTA`, `UNIT_STEP`, `LAPLACE`, `ILT` head symbols for Laplace transforms.**

Added four new IR head constants to the "Laplace / Fourier transforms" group at the bottom
of `nodes.py`, and exported all four from `__init__.py`:

- `DIRAC_DELTA = IRSymbol("DiracDelta")` — Dirac delta distribution δ(t)
- `UNIT_STEP = IRSymbol("UnitStep")` — Heaviside unit step function u(t)
- `LAPLACE = IRSymbol("Laplace")` — forward Laplace transform head
- `ILT = IRSymbol("ILT")` — inverse Laplace transform head

Required by `cas-laplace` 0.1.0 and `symbolic-vm` 0.32.3.

---

## 0.7.1 — 2026-04-27

**Add `MNEWTON` head symbol for Newton's method root finder.**

Added `MNEWTON = IRSymbol("MNewton")` to the "Numeric root-finding" group
at the bottom of `nodes.py`, and exported it from `__init__.py`. Required
by `cas-mnewton` 0.1.0 and `symbolic-vm` 0.32.2.

---

## 0.7.0 — 2026-04-27

**Phase 13 — Hyperbolic function head symbols.**

No new nodes were needed — `SINH`, `COSH`, `TANH`, `ASINH`, `ACOSH`, and
`ATANH` were already added in 0.5.0. This release bumps the version to align
with Phase 13 of the symbolic VM (0.32.0) and macsyma-compiler (0.7.0), which
fully implement hyperbolic function evaluation, differentiation, and
integration. Consumers that declare `symbolic-ir>=0.7.0` are guaranteed all
six hyperbolic head symbols are present and exported.

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
