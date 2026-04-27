# Changelog

## 1.1.0 — 2026-04-27

**Comprehensive pipeline test coverage (Sections S and T).**

Adds 20 new end-to-end pipeline tests covering operations that were already
wired in `symbolic-vm` but had no MACSYMA surface coverage:

**Section S — Calculus**:
- `diff(x^3, x)`, `diff(x^2+2x+1, x)` — polynomial differentiation.
- `diff(sin(x), x)` → `cos(x)`.
- `diff(cos(x), x)` → `-sin(x)`.
- `diff(exp(x), x)` → `exp(x)`.
- `integrate(x^2, x)` — power rule.
- `integrate(sin(x), x)` → `-cos(x)` (verifies `Cos` in result).
- `integrate(cos(x), x)` → `sin(x)`.
- `integrate(exp(x), x)` → `exp(x)`.
- `integrate(3, x)` → `3*x` (constant rule).
- `integrate(x+1, x)` — linearity.

**Section T — Matrix + Numeric**:
- `matrix([1,2],[3,4])` → `Matrix` node with 2 rows.
- `determinant(matrix([1,2],[3,4]))` → `IRInteger(-2)`.
- `transpose(matrix([1,2],[3,4]))` → `Matrix` node.
- `gcd(12, 8)` → `IRInteger(4)`.
- `lcm(4, 6)` → `IRInteger(12)`.
- `mod(17, 5)` → `IRInteger(2)`.
- `floor(3.7)` → `IRInteger(3)`.
- `ceiling(3.2)` → `IRInteger(4)`.
- `abs(-5)` → `IRInteger(5)`.

Total tests: 131, coverage 98.6%.

## 1.0.0 — 2026-04-27

**Roadmap item A1 — Kronecker polynomial factoring surfaced through MACSYMA `factor`.**

Bumps dependency to `symbolic-vm>=0.27.0` which ships `cas-factor 0.2.0`
with Kronecker's algorithm.  No changes to `macsyma-runtime` source code;
the improvement flows through automatically since `factor` → `Factor` IR head
→ `factor_handler` → `factor_integer_polynomial` → Kronecker.

4 new pipeline tests in `test_cas_pipeline.py` (Section R):
- `factor(x^4 + 4)` → `Mul` (Sophie Germain: `(x²+2x+2)(x²-2x+2)`).
- `factor(x^4 + x^2 + 1)` → `Mul` (cyclotomic: `(x²+x+1)(x²-x+1)`).
- `factor(x^3 - 2x^2 + x - 2)` → `Mul` (mixed: linear `(x-2)` + `(x²+1)`).
- `factor(x^2 + 1)` → `Factor(x^2+1)` unevaluated (irreducible over Z).

## 0.9.0 — 2026-04-27

**Rational function operations wired into MACSYMA name table (A3).**

Adds `MACSYMA_NAME_TABLE` entries:
`collect→Collect`, `together→Together`, `ratsimp→RatSimplify`,
`partfrac→Apart`.

These map to four new IR heads in `symbolic-vm` 0.26.0:
- `collect(expr, x)` — collect terms by powers of x.
- `together(expr)` — combine fractions into one rational expression.
- `ratsimp(expr)` — cancel common polynomial factors.
- `partfrac(expr, x)` — partial fraction decomposition.

Also adds `COLLECT`, `TOGETHER`, `RAT_SIMPLIFY`, `APART` IR symbol constants
to `name_table.py`.

6 new pipeline tests in `test_cas_pipeline.py` (Section Q) cover the full
MACSYMA surface syntax end-to-end.

## 0.8.0 — 2026-04-27

**Trig operations wired into MACSYMA name table (B1).**

Adds `MACSYMA_NAME_TABLE` entries:
`trigsimp→TrigSimplify`, `trigexpand→TrigExpand`, `trigreduce→TrigReduce`.

5 new pipeline tests cover the MACSYMA surface syntax end-to-end:
- `trigsimp(sin(x)^2 + cos(x)^2)` → `1`.
- `trigsimp(sin(%pi))` → `0`.
- `trigsimp(cos(%pi))` → `-1`.
- `trigexpand(sin(2*x))` → expanded form.
- `trigreduce(sin(x)^2)` → multiple-angle form.

## 0.7.0 — 2026-04-27

**NSolve and linear system pipeline tests added (A2c / A2d).**

Adds MACSYMA name-table entries `nsolve→NSolve` and `linsolve→Solve`
(linear-system form).  4 new pipeline tests cover:
- `nsolve(x^3 - 6*x^2 + 11*x - 6, x)` → 3 numeric IRFloat roots.
- `nsolve(x^5 - 1, x)` → 5 roots.
- `linsolve([x+y=3, x-y=1], [x,y])` → `[Rule(x,2), Rule(y,1)]`.
- `linsolve([x+y+z=6, 2*x+y=5, z=3], [x,y,z])` → 3 rules.

## 0.6.0 — 2026-04-27

**Cubic and quartic solve pipeline tests added (A2a / A2b).**

5 new pipeline tests in `test_cas_pipeline.py` cover cubic and quartic
equation solving end-to-end through the MACSYMA surface syntax:
`solve(x^3 - 6*x^2 + 11*x - 6, x)` → `[1, 2, 3]`, etc.
These tests exercise the full pipeline: MACSYMA parser → compiler →
`solve_handler` → `solve_cubic` / `solve_quartic` from `cas-solve`.

## 0.5.0 — 2026-04-27

**Complex number MACSYMA names wired (B2).**

Adds `MACSYMA_NAME_TABLE` entries for complex-number operations:
`%i`→`ImaginaryUnit`, `realpart`→`Re`, `imagpart`→`Im`,
`conjugate`→`Conjugate`, `cabs`→`Abs`, `carg`→`Arg`,
`rectform`→`RectForm`, `polarform`→`PolarForm`.

Pre-binds `%i` to `ImaginaryUnit` in `MacsymaBackend.__init__` so the
imaginary-unit constant is available without a prior `%i : ImaginaryUnit`
assignment.

11 new pipeline tests in `test_cas_pipeline.py` cover the MACSYMA surface
syntax end-to-end (`%i`, `realpart`, `imagpart`, `conjugate`, `%i^n`).

## 0.4.0 — 2026-04-27

**Number theory MACSYMA names wired (B3).**

Adds `MACSYMA_NAME_TABLE` entries for all number-theory heads:
`primep`→`IsPrime`, `next_prime`→`NextPrime`, `prev_prime`→`PrevPrime`,
`ifactor`→`FactorInteger`, `divisors`→`Divisors`, `totient`→`Totient`,
`moebius`→`MoebiusMu`, `jacobi`→`JacobiSymbol`, `chinese`→`ChineseRemainder`,
`numdigits`→`IntegerLength`.

6 new pipeline tests in `test_cas_pipeline.py` cover the MACSYMA surface
syntax end-to-end.

## 0.3.0 — 2026-04-27

**MACSYMA completion roadmap items C2, C3, C4, C5 wired.**

Implements the language-layer bindings for the new IR heads added to
`symbolic-vm` 0.20.0, plus improvements to `ev` flag handling.

**Name-table additions** (`MACSYMA_NAME_TABLE`):
- `lhs` → `Lhs`  — left-hand side of an equation (C5).
- `rhs` → `Rhs`  — right-hand side of an equation (C5).
- `at`  → `At`   — point evaluation (C4).
- `makelist` corrected: now maps to `MakeList` (proper generative list)
  instead of `Range` (plain integer range).

**`ev` flag improvements** (C3):
- `expand` flag: applies `Expand` to the result.
- `factor` flag: applies `Factor` to the result.
- `float` flag: alias for `numer` (force floating-point collapse).
- Unknown flags continue to be silently ignored.

**Tests added**:
- 9 new pipeline tests in `test_cas_pipeline.py` covering `lhs`, `rhs`,
  `makelist` (3-arg, 4-arg, 5-arg), and `at` (single rule, multi-rule).
- 3 new ev tests in `test_ev.py` covering `float`, `expand`, `factor` flags.

## 0.2.0 — 2026-04-27

**Name table wired; constants pre-bound.**

This release completes the two missing connections that prevented the MACSYMA
REPL from dispatching algebraic operations to the CAS substrate.

**`backend.py`** — `MacsymaBackend.__init__` now pre-binds:
- `%pi` → `IRFloat(math.pi)`
- `%e`  → `IRFloat(math.e)`

Users can now type `%pi` and `%e` without defining them first.

**`language.py` (macsyma-repl)** — `extend_compiler_name_table(_STANDARD_FUNCTIONS)`
is now called at REPL module load time. This merges `MACSYMA_NAME_TABLE`
into the compiler's `_STANDARD_FUNCTIONS` dict so that `factor`, `expand`,
`simplify`, `solve`, `subst`, `limit`, `taylor`, `length`, `first`, etc.
all compile to canonical IR heads (`Factor`, `Expand`, `Simplify`, …) rather
than opaque user-function calls.

**Architecture note** (see also `symbolic-vm` 0.19.0): The substrate handlers
themselves (`Factor`, `Solve`, `Simplify`, `Length`, `Determinant`, `Limit`,
…) now live in `symbolic-vm`'s `SymbolicBackend` — the inner doll. The
`MacsymaBackend` (outer doll) only adds MACSYMA-specific operations:
`Display`, `Suppress`, `Kill`, `Ev`, and the two constant bindings above.
Future Maple and Mathematica backends will extend `SymbolicBackend` directly
and inherit all algebraic operations without touching any MACSYMA code.

## 0.1.0 — 2026-04-25

Initial release — Phase A skeleton.

- `MacsymaBackend` — `SymbolicBackend` subclass with MACSYMA-specific
  heads (`Display`, `Suppress`, `Kill`, `Ev`) and option flags.
- `History` — input/output table, resolves `%`, `%i1`, `%o1`, ...
  via a backend lookup hook.
- `Display` / `Suppress` heads (`;` vs `$` statement terminators).
  Identity handlers; the REPL inspects the wrapper before eval to
  decide whether to print.
- `Kill(symbol)` and `Kill(all)` handlers.
- `Ev(expr, ...flags)` — minimal first cut: only the `numer` flag
  is honored.
- `MACSYMA_NAME_TABLE` — extends `macsyma-compiler`'s standard-name
  map so identifiers like `expand`, `factor`, `subst`, `solve`,
  `taylor`, `limit` route to canonical heads (the substrate handlers
  may not yet exist; the user gets `Expand(...)` unevaluated until
  they do).
- Type-checked, ruff- and mypy-clean.
