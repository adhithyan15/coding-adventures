# Changelog

## 0.8.0 — 2026-05-05

**Phase 27 — Polynomial inequality solving.**

New module `cas_solve/inequality.py` with one public function:

- ``try_solve_inequality(ineq_ir, var)`` — solves polynomial inequalities
  ``p(x) op 0`` (op ∈ {<, >, ≤, ≥}) for degrees 1–4 with rational
  coefficients.

**Algorithm**: normalise to ``f = lhs − rhs``, extract polynomial coefficients
via `symbolic_vm.polynomial_bridge.to_rational` (deferred import), find real
roots numerically (Durand-Kerner) with exact rational roots for degrees 1–2,
perform sign analysis in each interval between roots, and build the solution as
a list of IR comparison conditions.

**Output IR format**:

| Solution | IR form |
|----------|---------|
| ``(−∞, a)`` | ``Less(x, a)`` |
| ``[a, +∞)`` | ``GreaterEqual(x, a)`` |
| ``(a, b)`` | ``And(Greater(x,a), Less(x,b))`` |
| ``[a, b]`` | ``And(GreaterEqual(x,a), LessEqual(x,b))`` |
| all reals | ``GreaterEqual(IRInteger(0), IRInteger(0))`` |
| no solution | ``[]`` (empty list) |

**New export**: `try_solve_inequality` added to `cas_solve/__init__.py`.

---

## 0.7.0 — 2026-05-05

**Phase 26 — Transcendental equation solving.**

New module `cas_solve/transcendental.py` with a single public function:

- ``try_solve_transcendental(eq_ir, var)`` — dispatches across all recognised
  transcendental equation families and returns a list of IR solution nodes,
  or ``None`` if no pattern matches.

Families supported:

- **Trigonometric** (26a): `sin(ax+b) = c` → two periodic families with the
  free-integer constant ``FreeInteger`` (%k); `cos`, `tan` analogously.
- **Exponential/Logarithmic** (26b): `exp(ax+b) = c` → `x = (log(c)−b)/a`;
  `log(ax+b) = c` → `x = (exp(c)−b)/a`.
- **Lambert W** (26c): `f(x)·exp(f(x)) = c` with `f` linear → `f(x) = W(c)`.
  Uses the new ``LambertW`` IR head from `symbolic-ir` 0.13.0.
- **Hyperbolic** (26d): `sinh`, `cosh` (two branches), `tanh` with linear
  argument; inverted via `asinh`, `acosh`, `atanh`.
- **Compound substitution** (26e): detects when the equation is a polynomial
  in exactly one transcendental function of the variable (e.g.
  `sin(x)^2 + sin(x) = 0`) and solves by first solving the polynomial for the
  intermediate variable `u`, then inverting `f(x) = u` for each root.

The function is also exported from the package's `__init__.py`.

Dependency bump: `symbolic-ir` ≥ 0.13.0 (for `FREE_INTEGER`, `LAMBERT_W`).

---

## 0.6.0 — 2026-04-27

Phase 3 — Numeric root-finding and linear-system solver.

- ``nsolve_poly(coeffs, max_iter, tol)`` — Durand-Kerner iteration for all
  roots of a degree-n polynomial (float/complex coefficients).
- ``nsolve_fraction_poly(coeffs)`` — convenience wrapper accepting
  ``Fraction`` coefficients; returns ``IRFloat``/``IRApply`` IR nodes.
- ``roots_to_ir(roots)`` — converts ``complex`` roots to IR: near-real
  roots become ``IRFloat``; complex roots become
  ``Add(IRFloat(re), Mul(IRFloat(im), %i))`` expressions.
- ``solve_linear_system(equations, variables)`` — exact Gaussian
  elimination on a list of linear IR equations (``Equal(lhs, rhs)`` or
  ``expr = 0``); returns ``List(Rule(var, val), ...)`` or ``None`` for
  non-linear/singular systems.
- 22 new tests (13 Durand-Kerner + 9 linear-system), all green.
- Exported from ``cas_solve.__init__``.

## 0.4.0 — 2026-04-27

Phase 2 — Cubic and quartic closed-form solvers.

- ``solve_cubic(a, b, c, d)`` — closed form for monic cubics:
  - Rational-root theorem pass (rational roots → deflation to quadratic).
  - Cardano's formula for ``D > 0`` (one real + two complex roots).
  - Repeated-root case (``D = 0``).
  - Casus irreducibilis (``D < 0``, three real irrational roots) returns
    ``[]`` — cannot be expressed in real radicals.
  - Uses ``Cbrt`` IR head for symbolic cube roots.
- ``solve_quartic(a, b, c, d, e)`` — closed form for quartics via Ferrari:
  - Delegates to ``solve_cubic`` when leading coefficient is zero.
  - Rational-root theorem pass first.
  - Biquadratic special case (no odd-degree terms).
  - Full Ferrari method via resolvent cubic (requires rational resolvent
    root; returns ``[]`` otherwise).
- 21 new cubic tests + 13 new quartic tests.
- Type-checked, ruff- and mypy-clean.

## 0.1.0 — 2026-04-25

Initial release — Phase 1.

- ``solve_linear(a, b)`` — closed form for ``a·x + b = 0``.
- ``solve_quadratic(a, b, c)`` — closed form for ``a·x² + b·x + c = 0``,
  including complex roots via ``%i`` and exact ``Sqrt`` for irrational
  discriminants.
- ``SOLVE``, ``NSOLVE``, ``ROOTS`` head sentinels.
- Type-checked, ruff- and mypy-clean.

Deferred to Phase 2: cubic/quartic closed forms, Durand–Kerner numeric,
linear systems via Gaussian elimination.
