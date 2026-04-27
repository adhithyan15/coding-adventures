# Changelog

## 0.26.0 — 2026-04-27

**Roadmap item A3 — rational function operations (Collect, Together, RatSimplify, Apart, full Expand).**

`Expand` handler upgraded from a `canonical()`-only pass to full polynomial
distribution via the polynomial bridge.  Four new IR heads wired into
`SymbolicBackend` via `build_cas_handler_table()`:

- **`Expand`** (improved): calls `to_rational` + `from_polynomial` to distribute
  `Mul` over `Add` and expand integer powers for single-variable polynomials
  with rational coefficients. Falls back to `canonical` for multi-variable /
  transcendental expressions.
- **`Collect(expr, var)`**: groups terms by powers of `var` for single-variable
  polynomials with rational coefficients (same mechanism as `Expand` but takes
  an explicit variable argument). MACSYMA surface: `collect`.
- **`Together(expr)`**: combines a sum of rational functions into a single
  fraction `P(x)/Q(x)` with monic denominator. MACSYMA surface: `together`.
- **`RatSimplify(expr)`**: cancels the GCD of numerator and denominator,
  reducing the rational expression to lowest terms. MACSYMA surface: `ratsimp`.
- **`Apart(expr, var)`**: partial-fraction decomposition (Phase 1 — distinct
  rational linear factors only). Uses residue formula `A_i = P(r_i)/Q'(r_i)`.
  MACSYMA surface: `partfrac`. Falls back to unevaluated for irreducible
  quadratic or repeated factors.

**Dependencies**: `coding-adventures-polynomial` was already in
`pyproject.toml`; the new handlers import `gcd`, `monic`, `deriv`,
`evaluate`, `rational_roots`, `divmod_poly` directly from `polynomial`.

**New tests (18 in Section 14 of `test_cas_handlers.py`)** +
**6 new pipeline tests in `macsyma-runtime`**.

## 0.25.0 — 2026-04-27

**Roadmap item B1 (cas-trig) wired into SymbolicBackend.**

`cas-trig` is now a dependency. Its handler table is merged via
`_build_trig()` in `SymbolicBackend.__init__`.

**New IR heads** (3 total):
`TrigSimplify`, `TrigExpand`, `TrigReduce`.

- `TrigSimplify`: Pythagorean identity (`sin²+cos²→1`), sign rules
  (`sin(-x)→-sin(x)`, `cos(-x)→cos(x)`), and special-value lookup
  (`sin(π/6)→1/2`, etc.).
- `TrigExpand`: angle-addition formulas and Chebyshev recurrence for
  integer multiples (`sin(2x)→2sin(x)cos(x)`, `cos(3x)→...`).
- `TrigReduce`: power-to-multiple-angle reduction
  (`sin²(x)→(1-cos(2x))/2`, `cos³(x)→(3cos(x)+cos(3x))/4`, etc.).

**Dependencies updated:**
- `cas-trig>=0.1.0` added to `pyproject.toml`.

**New tests (5 in `test_cas_handlers.py`)** + **5 pipeline tests**.

## 0.24.0 — 2026-04-27

**Roadmap items A2c + A2d (NSolve and linear systems) wired in.**

- `solve_handler` extended to detect `Solve(List(eqs...), List(vars...))`
  and route it to `solve_linear_system` (Gaussian elimination, exact
  rational arithmetic). Returns `List(Rule(var, val), ...)`.
- `nsolve_handler` added for `NSolve(poly, var)`: Durand-Kerner iteration
  returning `IRFloat`/complex IR roots for any polynomial degree.
- `MACSYMA_NAME_TABLE` gains `"nsolve"→NSolve` and `"linsolve"→Solve`.
- `cas-solve>=0.6.0` dependency pin updated.
- 4 new tests in `test_cas_handlers.py` + 4 new pipeline tests.

## 0.23.0 — 2026-04-27

**Roadmap items A2a + A2b (cubic and quartic solvers) wired into `solve_handler`.**

The `Solve` handler in `cas_handlers.py` now supports degree-3 and degree-4
polynomials via `cas-solve`'s new `solve_cubic` and `solve_quartic`:

- **Degree 3**: routes through `solve_cubic` (rational-root theorem → Cardano).
  Returns a `List` of roots, or unevaluated for casus irreducibilis.
- **Degree 4**: routes through `solve_quartic` (rational-root theorem →
  biquadratic → Ferrari). Returns a `List` of roots, or unevaluated when the
  Ferrari resolvent has no rational root.
- Empty or "ALL" results are propagated as unevaluated expressions.

**Dependencies updated:**
- `cas-solve` bumped to `>=0.4.0` in `pyproject.toml`.

**New tests (4 in `test_cas_handlers.py`):**
`test_solve_cubic_three_rational`, `test_solve_cubic_one_rational_two_complex`,
`test_solve_quartic_four_rational`, `test_solve_degree_5_passthrough`.

## 0.22.0 — 2026-04-27

**Roadmap item B2 (cas-complex) wired into SymbolicBackend.**

`cas-complex` is now a dependency. Its handler table is merged into
`build_cas_handler_table()` via `**_build_complex()`, and two additional
integration points are set up in `SymbolicBackend.__init__`:

- `ImaginaryUnit` is pre-bound to itself so it evaluates as an inert
  symbol (rather than triggering the unresolved-symbol fall-through).
- A wrapper around the `Pow` handler routes `ImaginaryUnit^n` through
  `imaginary_power_handler` (reducing `i^n → {1, i, -1, -i}` via `n % 4`)
  before falling through to the standard power handler.
- `Abs` is extended: when its argument contains `ImaginaryUnit`, it
  delegates to `abs_complex_handler` (returning `sqrt(re² + im²)`).

**New IR heads** (7 total):
`Re`, `Im`, `Conjugate`, `Arg`, `RectForm`, `PolarForm`, `AbsComplex`.

## 0.21.0 — 2026-04-27

**Roadmap item B3 (cas-number-theory) wired into SymbolicBackend.**

The new `cas-number-theory` package is now a dependency and its handler
table is merged into `build_cas_handler_table()` via `**_build_nt()`.

**New IR heads** (10 total, all language-neutral):
`IsPrime`, `NextPrime`, `PrevPrime`, `FactorInteger`, `Divisors`,
`Totient`, `MoebiusMu`, `JacobiSymbol`, `ChineseRemainder`, `IntegerLength`.

## 0.20.0 — 2026-04-27

**Roadmap items C2, C4, C5 implemented** — three items from the MACSYMA
completion roadmap (`macsyma-completion.md`) are now live in `SymbolicBackend`.
All three are language-neutral IR heads; every future CAS frontend inherits
them automatically.

**New handlers installed on `SymbolicBackend`**:

- **`Lhs(Equal(a, b))` → `a`** (C5) — left-hand side of an equation.
- **`Rhs(Equal(a, b))` → `b`** (C5) — right-hand side of an equation.
- **`MakeList(expr, var, n)` / `MakeList(expr, var, from, to[, step])`** (C2)
  — generative list construction: evaluates `expr` for each integer value
  of `var` in the specified range.  Replaces the previous stub that mapped
  `makelist` → `Range`.
- **`At(expr, Equal(var, val))` / `At(expr, List(…))` → substitution then eval** (C4)
  — point evaluation; sugar over `Subst`.  Handles both single rules and
  lists of rules.

**Bug fix**: `MACSYMA_NAME_TABLE["makelist"]` previously routed to `Range`
(a plain integer range generator). It now routes to the correct `MakeList`
head, which evaluates an arbitrary expression over a range.

**New import**: `EQUAL` added to the `cas_handlers.py` imports from
`symbolic_ir`.

## 0.19.0 — 2026-04-27

**CAS substrate handlers wired into SymbolicBackend** — the universal inner
doll. Every CAS frontend that extends `SymbolicBackend` (MACSYMA, Maple,
Mathematica, …) now inherits the full algebraic operation set for free.
Language-specific quirks (Display/Suppress/Kill/Ev) remain in the language
backend subclass (the outer doll).

**New module**: `symbolic_vm/cas_handlers.py`

**New handlers installed on `SymbolicBackend`**:

- **Algebraic**: `Simplify`, `Expand`, `Factor` (Phase 1: integer-root
  factoring via rational-root theorem), `Solve` (linear and quadratic over Q),
  `Subst` (structural substitution + re-evaluation).
- **List operations**: `Length`, `First`, `Rest`, `Last`, `Append`, `Reverse`,
  `Range`, `Map`, `Apply`, `Select`, `Sort`, `Part`, `Flatten`, `Join`.
- **Matrix**: `Matrix`, `Transpose`, `Determinant`, `Inverse`.
- **Calculus**: `Limit` (direct-substitution Phase 1), `Taylor` (polynomial
  Taylor expansion).
- **Numeric**: `Abs`, `Floor`, `Ceiling`, `Mod`, `Gcd`, `Lcm`.

**Package dependencies added**: `cas-pattern-matching`, `cas-substitution`,
`cas-simplify`, `cas-factor`, `cas-solve`, `cas-list-operations`,
`cas-matrix`, `cas-limit-series`.

**Architecture note**: These handlers are the **inner doll** — universal
CAS operations that any symbolic algebra language can use unchanged. They
are explicitly *not* placed in `MacsymaBackend` so that future Maple and
Mathematica backends can extend `SymbolicBackend` directly and inherit
the complete algebraic substrate without touching any MACSYMA-specific code.

## 0.18.0 — 2026-04-23

Phase 13 of the integration roadmap — hyperbolic functions.

**New heads** (requires `symbolic-ir >= 0.5.0`): SINH, COSH, TANH, ASINH, ACOSH, ATANH.

**New capability**:
- `∫ P(x) · sinh(ax+b) dx` and `∫ P(x) · cosh(ax+b) dx` — tabular IBP with sign `(−1)^k`.
  Assembly: `C(x)·cosh(ax+b) + S(x)·sinh(ax+b)` for sinh; swapped for cosh.
- `∫ P(x) · asinh(ax+b) dx` — IBP + reduction formula `∫ tⁿ/√(t²+1) dt`.
  Final: `[Q(x)−B(ax+b)]·asinh(ax+b) − A(ax+b)·√((ax+b)²+1)`.
- `∫ P(x) · acosh(ax+b) dx` — same reduction formula with `√(t²−1)`.
  Final: `[Q(x)−B(ax+b)]·acosh(ax+b) − A(ax+b)·√((ax+b)²−1)`.
- `∫ tanh(ax+b) dx = (1/a)·log(cosh(ax+b))`.
- `∫ atanh(ax+b) dx = (ax+b)/a·atanh(ax+b) + (1/(2a))·log(1−(ax+b)²)`.

**New modules**: `sinh_poly_integral.py`, `asinh_poly_integral.py`.

**Differentiation rules**: all six hyperbolic functions wired into `_diff_ir`.

**poly×tanh and poly×atanh deferred** to a future phase.

**Tests**: 45 tests in `tests/test_phase13.py` using numerical finite-difference
verification. All correctness checks pass.

## 0.17.0 — 2026-04-23

Phase 12 of the integration roadmap — polynomial × asin/acos(linear) integration via IBP.

**New capability**: `∫ P(x) · asin(ax+b) dx` and `∫ P(x) · acos(ax+b) dx` for any
`P ∈ Q[x]` and `a ∈ Q \ {0}`, completing all three inverse-trig × polynomial families.

**Algorithm** (integration by parts):

- **asin IBP**: `u = asin(ax+b)`, `dv = P dx` → `du = a/√(1−(ax+b)²) dx`, `v = Q = ∫P dx`
  - Residual: `a · ∫ Q/√(1−(ax+b)²) dx = ∫ Q̃(t)/√(1−t²) dt`  (t = ax+b substitution)
  - Residual decomposed via reduction formula: `∫ Q̃/√(1−t²) dt = A(t)·√(1−t²) + B(t)·asin(t)`
  - Final result: `[Q(x) − B(ax+b)]·asin(ax+b) − A(ax+b)·√(1−(ax+b)²)`

- **acos IBP**: sign of `du` flips (`d/dx acos = −a/√`), giving
  - Final result: `Q(x)·acos(ax+b) + A(ax+b)·√(1−(ax+b)²) + B(ax+b)·asin(ax+b)`
  - The B·asin term is non-zero for deg(P) ≥ 1 — this is expected, not a bug.

**New module** `asin_poly_integral.py`:
- `asin_poly_integral(poly, a, b, x_sym)` — IBP closed-form for `∫ P(x)·asin(ax+b) dx`.
- `acos_poly_integral(poly, a, b, x_sym)` — IBP closed-form for `∫ P(x)·acos(ax+b) dx`.
- Private helpers: `_compose_to_t`, `_sqrt_integral_decompose`, `_poly_compose_linear`, `_compute_AB`.
- Reduction formula is memoized per monomial degree for efficiency.

**Dispatcher hooks** in `integrate.py`:
- `_try_asin_product` / `_try_acos_product` — check ASIN/ACOS head, validate linear arg and polynomial coefficient.
- Both hooks try both operand orders, inserted after Phase 11 in the MUL handler.
- Bare `asin/acos(linear)` cases (P = 1) handled in the elementary-function branch.
- Differentiation rules for `d/dx asin(u)` and `d/dx acos(u)` added to `_diff_ir`.

**VM handlers** in `handlers.py`:
- `asin(simplify)` and `acos(simplify)` handlers registered (numeric fold + symbolic passthrough).

**symbolic-ir**: bumped dependency to `>=0.4.0` (requires ASIN/ACOS head symbols).

**Limitations (future work)**:
- `∫ asin(g(x))` for non-linear `g`.
- `∫ asin(ax+b)^n dx` for `n ≥ 2`.
- `∫ asin(ax+b) · exp(x) dx` (mixed inverse-trig × exponential).

## 0.16.0 — 2026-04-22

Phase 11 of the integration roadmap — polynomial × arctan(linear) integration via IBP.

**New capability**: `∫ P(x) · atan(ax+b) dx` for any `P ∈ Q[x]` and `a ∈ Q \ {0}`.

**Algorithm** (integration by parts):
- `u = atan(ax+b)`, `dv = P(x) dx` → `v = Q(x) = ∫P dx` (polynomial antiderivative)
- Residual `∫ Q(x)/D(x) dx` resolved by polynomial long division `Q = S·D + R` (deg R < 2),
  where `D = (ax+b)² + 1`; polynomial part `T = ∫S dx` handled directly, remainder
  dispatched to `arctan_integral` (Phase 2e).
- Final result: `Q(x)·atan(ax+b) − a·T(x) − a·arctan_integral(R, D)`.

**New module** `atan_poly_integral.py`:
- `atan_poly_integral(poly, a, b, x_sym)` — full IBP closed-form IR for `∫ P(x)·atan(ax+b) dx`.
- `_integrate_poly` — ascending-coefficient polynomial antiderivative with Fraction arithmetic.

**Dispatcher hook** in `integrate.py`:
- `_try_atan_product(transcendental, poly_candidate, x)` — checks ATAN head, extracts linear
  arg via `_try_linear`, validates polynomial coefficient via `to_rational`.
- Tries both operand orders: `_try_atan_product(a, b, x) or _try_atan_product(b, a, x)`.
- Inserted after Phase 3e (poly × log) in the MUL handler.

**Special case**: `P = 1` (bare arctan) reduces to the same result as Phase 9 — verified.

**Limitations (future work)**:
- `∫ atan(g(x))` for non-linear `g` (e.g. `atan(x²)`, `atan(sin(x))`).
- `∫ atan(ax+b)^n dx` for `n ≥ 2`.
- `∫ R(x)·atan(ax+b) dx` for rational (non-polynomial) `R`.

## 0.15.0 — 2026-04-22

Phase 10 of the integration roadmap — generalized partial-fraction integration and
Rothstein–Trager performance fix.

**RT performance guard**:
- `_integrate_rational` now skips Rothstein–Trager for degree ≥ 6 denominators.
  RT's Sylvester-matrix determinant (Fraction arithmetic, 12×12) caused a 26,261-second
  hang on `(x²+1)(x²+4)(x²+9)` while always returning None. The guard is safe:
  degree-6 denominators with any irreducible quadratic factor always have irrational
  RT coefficients, so skipping is lossless.

**Three distinct irreducible quadratics** (`Q₁·Q₂·Q₃`, degree-6 squarefree):
- `_factor_triple_quadratic` — finite candidate search (rational divisors of the
  constant term × small linear-coefficient candidates) to factor a degree-6 poly
  into three monic irreducible quadratics over Q; delegates to `_factor_biquadratic`
  for the degree-4 quotient.
- Handles both diagonal (`1/((x²+1)(x²+4)(x²+9))`) and non-diagonal cases
  (`1/((x²+2x+2)(x²+4)(x²+9))`).

**Linear factors × two irreducible quadratics** (`Lᵐ·Q₁·Q₂`, degree 5–6):
- `_solve_pf_general` — D×D Gaussian elimination over Fraction for any list of
  coprime polynomial factors (total degree D = Σdᵢ).
- `_try_general_rational_integral` — Phase 10 driver: extracts rational linear
  factors, factors the quadratic remainder via `_factor_biquadratic` or
  `_factor_triple_quadratic`, solves the generalized partial-fraction system, then
  integrates linear pieces as logs and quadratic pieces via `arctan_integral`.
- Handles degree-5 (`1/((x−1)(x²+1)(x²+4))`) and degree-6
  (`1/((x−1)(x−2)(x²+1)(x²+4))`) cases.

**Limitations (future work)**:
- Denominators with four or more irreducible quadratic factors (degree ≥ 8).
- Denominators that are irreducible of degree 4 over Q (e.g. `x⁴+1`).
- Denominators of degree > 6 in general.

## 0.14.0 — 2026-04-22

Phase 9 of the integration roadmap — multi-quadratic partial-fraction integration.

Extends the rational-function route to handle denominators that are products of
**two distinct irreducible quadratic factors** over Q (no linear factors), closing
the gap left by Phases 2d–2f.

**Core — two-quadratic partial fractions**:
- Detects degree-4 squarefree denominators with no rational roots.
- Attempts to factor as `Q₁·Q₂` using a finite candidate search (rational divisors
  of the constant term, derived from the coefficient-match system).
- Solves the 4×4 partial-fraction linear system over Q by Gaussian elimination.
- Integrates each `(Aᵢx+Bᵢ)/Qᵢ` piece via the existing Phase 2e `arctan_integral`.
- Handles both pure-arctan outputs (`1/((x²+1)(x²+4))`) and mixed log+arctan outputs
  (`(x+1)/((x²+1)(x²+4))`).
- Non-diagonal quadratics (`x²+2x+5` etc.) are fully supported.

**Bonus — `∫ atan(ax+b) dx` table entry**:
- Added to the Phase 3 linear-arg dispatch alongside sin/cos/exp/log/tan.
- Result: `x·atan(ax+b) − (1/(2a))·log((ax+b)²+1)`.
- Covers all linear arguments including fractional coefficients.

**New helpers in `integrate.py`**:
- `_int_divisors`, `_rational_divisors` — finite candidate enumeration.
- `_factor_biquadratic` — splits degree-4 poly into two irreducible quadratics.
- `_solve_pf_2quad` — Gaussian elimination for the 4×4 partial-fraction system.
- `_try_multi_quad_integral` — Phase 9 driver; hooked into `_integrate_rational`
  after `mixed_integral` (Phase 2f).

New spec: `code/specs/phase9-multi-quad-partial-fraction.md`.

42 new tests (`tests/test_phase9.py`). Package at ~532 tests.

## 0.13.0 — 2026-04-22

Phase 8 of the integration roadmap — power-of-composite u-substitution.

Extends u-substitution (Phase 7) to handle integrands where the outer function
is a **power** of a composite: `POW(f(g(x)), n) · c·g'(x)` and `POW(g(x), n) · c·g'(x)`.

**Case A — `f(g(x))^n · c·g'(x)`**:
- Substitute `u = g(x)`, integrate `∫ f(u)^n du` via Phase 5 (sin/cos/tan reduction
  formulas for trig outers), back-substitute `u → g(x)`.
- Guard: `g` must not be bare `x` (Phase 5 handles) or linear with a≠0 (Phase 5 handles).

**Case B — `g(x)^n · c·g'(x)`**:
- Substitute `u = g(x)`, integrate `∫ u^n du` via Phase 1 power rule,
  back-substitute.
- Special case n=−1: `∫ u⁻¹ du = log(u)` → `log(g(x))`.
- Guard: `g` must not be bare `x` or linear with a≠0.

**Bonus — `(ax+b)^n` in the single-factor POW branch**:
- `∫ (ax+b)^n dx = (ax+b)^(n+1)/((n+1)·a)` for n≠−1.
- `∫ (ax+b)^(−1) dx = log(ax+b)/a`.
- Supports integer and symbolic exponents.

**`_diff_ir` extensions** (enables Case B for sums of functions):
- `NEG(f)`: `d/dx(−f) = −f'`
- `ADD(f, g)`: `d/dx(f+g) = f' + g'` (zero terms simplified)
- `SUB(f, g)`: `d/dx(f−g) = f' − g'`

New helpers in `integrate.py`: `_try_u_sub_pow_one`, `_try_u_sub_pow`.
Hook in MUL branch after Phase 7, before Phase 4c.

New spec: `code/specs/phase8-power-composite-usub.md`.

40 new tests (`tests/test_phase8.py`). Package at ~491 tests.

## 0.12.0 — 2026-04-20

Phase 7 of the integration roadmap — u-substitution (chain-rule reversal).

Handles integrands of the form `f(g(x)) · c·g'(x)` where `f` is a single-argument
function (SIN, COS, EXP, LOG, TAN, SQRT) and `c` is a rational constant.

**Algorithm**: for each factor pair (outer, gp_candidate):
1. Extract `g(x)` = argument of the outer function.
2. Skip if `g = x` (Phase 1) or `g` is linear (Phases 3–5).
3. Compute `g'(x)` symbolically via `_diff_ir`.
4. Check `gp_candidate = c · g'(x)` via `_ratio_const`.
5. Introduce dummy symbol `u`, compute `∫ F(u) du`, substitute `g(x)` back.

New helpers in `integrate.py`: `_poly_deriv`, `_poly_mul`, `_diff_ir`,
`_ratio_const`, `_subst`, `_try_u_sub_one`, `_try_u_sub`.

Hook placed in the MUL branch after Phase 6, before Phase 4c/4a — linear-arg
integrands are guarded away so earlier phases retain their cases.

New spec: `code/specs/phase7-u-substitution.md`.

44 new tests (`tests/test_phase7.py`). Package at 451 tests.

## 0.11.0 — 2026-04-21

Phase 6 of the integration roadmap — mixed trig powers `sinⁿ·cosᵐ`.

Three cases, each with a distinct algorithm:

**Phase 6a — n odd (cosine substitution)**:
- Substitute `u = cos(ax+b)`, `du = -a sin(ax+b) dx`.
- Write `sinⁿ⁻¹ = (1-cos²)^k` (k=(n-1)/2) and expand via the binomial theorem.
- Closed-form result: `-(1/a) · Σ C(k,j)(-1)^j / (m+2j+1) · cos^{m+2j+1}(ax+b)`
- No recursion — direct polynomial anti-differentiation.

**Phase 6b — m odd, n even (sine substitution)**:
- Substitute `u = sin(ax+b)`, `du = a cos(ax+b) dx`.
- Write `cosᵐ⁻¹ = (1-sin²)^k` (k=(m-1)/2) and expand.
- Closed-form result: `(1/a) · Σ C(k,j)(-1)^j / (n+2j+1) · sin^{n+2j+1}(ax+b)`

**Phase 6c — both even (IBP reduction on n)**:
- Reduction: `∫ sinⁿ cosᵐ dx = -sinⁿ⁻¹cosᵐ⁺¹/((n+m)a) + (n-1)/(n+m) · ∫ sinⁿ⁻² cosᵐ dx`
- Derived via IBP with Pythagorean substitution `cosᵐ⁺² = cosᵐ(1-sin²)`.
- Recurses on n: at n=0 delegates to `∫ cosᵐ dx` → Phase 5b.

New helpers in `integrate.py`: `_extract_trig_power`, `_try_sin_cos_power`,
`_sin_cos_odd_sin`, `_sin_cos_odd_cos`, `_sin_cos_even`.

New spec: `code/specs/phase6-sin-cos-powers.md`.

44 new tests (`tests/test_phase6.py`). Package at 407 tests, 90% coverage.

## 0.10.0 — 2026-04-20

Phase 5 of the integration roadmap — trig-power integration. Three sub-phases
covering `tan`, `sinⁿ`, `cosⁿ`, and `tanⁿ` for any integer `n ≥ 2`.

**Phase 5a — tan(ax+b)**:
- `∫ tan(ax+b) dx = −log(cos(ax+b)) / a` derived via substitution `u = cos(ax+b)`.
- Bare `∫ tan(x) dx = −log(cos(x))` handled in the Phase 1 elementary section.
- Extended linear-arg dispatch table from `{EXP, SIN, COS, LOG}` to include `TAN`.
- New helper `_tan_integral(a, b, x)` in `integrate.py`.

**Phase 5b — sinⁿ(ax+b) and cosⁿ(ax+b) reduction formulas** (`n ≥ 2`):
- `∫ sinⁿ(ax+b) dx = −sinⁿ⁻¹(ax+b)·cos(ax+b)/(n·a) + (n−1)/n · ∫ sinⁿ⁻²(ax+b) dx`
- `∫ cosⁿ(ax+b) dx =  cosⁿ⁻¹(ax+b)·sin(ax+b)/(n·a) + (n−1)/n · ∫ cosⁿ⁻²(ax+b) dx`
- Derived by integration by parts + the Pythagorean identity.
- Recursion terminates at `n=0` (→ `x`) and `n=1` (→ Phase 3 sin/cos result).

**Phase 5c — tanⁿ(ax+b) reduction formula** (`n ≥ 2`):
- `∫ tanⁿ(ax+b) dx = tanⁿ⁻¹(ax+b)/((n−1)·a) − ∫ tanⁿ⁻²(ax+b) dx`
- Derived using `tan² = sec² − 1`, making `∫ tanⁿ⁻² · sec² dx` exact.
- Recursion terminates at `n=0` (→ `x`) and `n=1` (→ Phase 5a tan result).

**POW base-case fixes** (needed for recursion correctness):
- `f^0 = 1` in the `POW` branch of `_integrate` now returns `x` directly.
- `f^1 = f` in the `POW` branch delegates to `_integrate(base, x)`.
- Both cases are also correct in isolation (not purely reduction plumbing).

New helpers in `integrate.py`: `_tan_integral`, `_try_trig_power`,
`_sin_power`, `_cos_power`, `_tan_power`.

Requires `coding-adventures-symbolic-ir >= 0.3.0` (adds `TAN` head) and
`coding-adventures-macsyma-compiler >= 0.2.0` (maps MACSYMA `tan` to `TAN`).
Requires `coding-adventures-symbolic-vm >= 0.10.0` for the `Tan` evaluation handler.

44 new tests (`tests/test_phase5.py`). Package at 363 tests, 90% coverage.

## 0.9.0 — 2026-04-20

Phase 4 of the integration roadmap — trigonometric integration. Three
sub-phases, each a clean layer on top of the existing integrator.

**Phase 4a — Polynomial × sin/cos** (`∫ p(x)·sin(ax+b) dx`,
`∫ p(x)·cos(ax+b) dx`):
- New module `symbolic_vm.trig_poly_integral`: `trig_sin_integral` and
  `trig_cos_integral` implement the **tabular IBP** formula. IBP applied
  `deg(p)+1` times yields two coefficient polynomials C and S:
  `∫ p·sin = sin·S − cos·C`, `∫ p·cos = sin·C + cos·S`.
- `_cs_coeffs` builds C and S from the derivative sequence of `p`, using
  `sign = (−1)^(k//2)` and divisor `a^(k+1)` for each index `k`.
- Wired into the `MUL` branch of `_integrate` as `_try_trig_product`.

**Phase 4b — Trig products and squares**:
- No new module; logic in `integrate.py` as `_try_trig_trig`.
- Applies the product-to-sum identities at the IR level:
  `sin·sin = [cos(u−v)−cos(u+v)]/2`, `cos·cos = [cos(u−v)+cos(u+v)]/2`,
  `sin·cos = [sin(u+v)+sin(u−v)]/2`. The resulting linear combination of
  bare sin/cos is recursively integrated by Phase 3 (cases 3b/3c).
- Handles all three orderings (sin·sin, cos·cos, sin·cos) by skipping the
  cos·sin ordering and relying on the swapped-argument retry in the caller.

**Phase 4c — Exp × sin/cos** (`∫ exp(ax+b)·sin(cx+d) dx`,
`∫ exp(ax+b)·cos(cx+d) dx`):
- New module `symbolic_vm.exp_trig_integral`: `exp_sin_integral` and
  `exp_cos_integral` implement the **double-IBP closed form**:
  `∫ exp·sin = exp·[a·sin − c·cos]/(a²+c²)`,
  `∫ exp·cos = exp·[a·cos + c·sin]/(a²+c²)`.
- Wired into the `MUL` branch as `_try_exp_trig`, before `_try_trig_product`.

Updated regression: `test_integrate_two_x_factors_unevaluated` renamed to
`test_integrate_poly_times_sin_now_closed_by_phase4` — Phase 4a now closes
`∫ x·sin(x) dx`.

Also updated `symbolic-computation.md` (Phase 4 description updated from
"Algebraic extensions" to the practical trig-integration scope).

39 new tests (`tests/test_phase4.py`). Package at 319 tests, 90% coverage.

## 0.8.0 — 2026-04-20

Phase 3 of the integration roadmap — transcendental integration for the
most common single-extension cases. Extends the integrator to handle
polynomials multiplied by `exp`, `log`, `sin`, or `cos` of a **linear**
argument `a·x + b`.

Five new cases, two algorithms:

- **Case 3a**: `∫ exp(ax+b) dx = exp(ax+b)/a` — generalises the
  existing Phase 1 `exp(x)` rule to any linear argument.
- **Case 3b**: `∫ sin(ax+b) dx = −cos(ax+b)/a` — generalises `sin(x)`.
- **Case 3c**: `∫ cos(ax+b) dx = sin(ax+b)/a` — generalises `cos(x)`.
- **Case 3d**: `∫ p(x)·exp(ax+b) dx` for `p ∈ Q[x]` — solved by the
  **Risch differential equation** `g′ + a·g = p` via back-substitution;
  result is `g(x)·exp(ax+b)`.
- **Case 3e**: `∫ p(x)·log(ax+b) dx` for `p ∈ Q[x]` — solved by
  **integration by parts** followed by polynomial long division; result
  is `[P(x) − P(−b/a)]·log(ax+b) − S(x)`.

New modules:
- `symbolic_vm.exp_integral`: `exp_integral(poly, a, b, x_sym)` —
  implements cases 3a and 3d.
- `symbolic_vm.log_integral`: `log_poly_integral(poly, a, b, x_sym)` —
  implements case 3e (and the `log(x)` case of 3e extends Phase 1's
  hard-coded result to arbitrary linear arguments).

`polynomial_bridge.py` gains a public `linear_to_ir(a, b, x)` helper
shared by both new modules.

`_integrate` in `integrate.py` gains:
- Extended elementary-function section recognising `EXP`/`SIN`/`COS`/
  `LOG` of linear arguments (cases 3a–3c, 3e-bare).
- Two new helper functions `_try_exp_product` and `_try_log_product`
  wired into the `MUL` branch to handle cases 3d and 3e.
- `_try_linear` helper that recognises `a·x + b` in the IR.

New spec `code/specs/phase3-transcendental.md` documents all five
cases with step-by-step algorithms and worked examples.

33 new tests (`tests/test_phase3.py`). Package at 280 tests, 89% coverage.

## 0.7.0 — 2026-04-20

Phase 2f of the integration roadmap — mixed partial-fraction integration
for denominators of the form L(x)·Q(x) where L is a product of distinct
linear factors over Q and Q is a single irreducible quadratic. Closes
rational-function integration for all denominators of this shape,
completing the most common class of textbook integrals (e.g.
`1/((x−1)(x²+1))`, `x/((x+2)(x²+4))`).

- New module `symbolic_vm.mixed_integral`:
  - `mixed_integral(num, den, x_sym) → IRNode | None` applies the
    Bézout identity to split `C/(L·Q)` into `C_L/L + C_Q/Q`, then
    delegates to Rothstein–Trager (Phase 2d) for the log part and
    `arctan_integral` (Phase 2e) for the arctan part. Returns `None`
    when the denominator does not match the L·Q shape (no rational
    roots, deg Q ≠ 2, or Q has rational roots).
- `Integrate` handler gains a Phase 2f step between the arctan check
  and the unevaluated fallback. The progress gate was extended to
  treat a successful `mixed_ir` result as progress.
- `rt_pairs_to_ir` moved from a private helper in `integrate.py` to a
  public function in `polynomial_bridge.py`, avoiding a circular import
  from `mixed_integral.py`. The private wrapper in `integrate.py` now
  delegates to it.
- New spec `code/specs/mixed-integral.md` documents the Bézout
  algorithm, worked example for `1/((x−1)(x²+1))`, and correctness
  derivation.
- 18 new tests (`tests/test_mixed_integral.py`): one-linear-one-
  quadratic (5), two-linear-one-quadratic (2), mixed numerators (2),
  fall-through guards (3), Bézout split identity verification (1), and
  end-to-end VM tests (5). Package at 247 tests, 90% coverage.

## 0.6.0 — 2026-04-20

Phase 2e of the integration roadmap — arctan antiderivatives for
irreducible quadratic denominators. Closes the gap left by
Rothstein–Trager: `1/(x²+1)` and its kin now produce closed-form
`arctan` output instead of staying as unevaluated `Integrate`.

- New module `symbolic_vm.arctan_integral`:
  - `arctan_integral(num, den, x_sym) → IRNode` applies the direct
    formula `A·log(E) + (2B/D)·arctan((2ax+b)/D)` for any proper
    rational function with an irreducible quadratic denominator
    `ax²+bx+c`. When `D = √(4ac−b²)` is rational (perfect square),
    the output carries only rational/integer leaves. When `D` is
    irrational, the IR carries `Sqrt(D²)` which the symbolic backend
    leaves unevaluated and the numeric backend folds.
- `Integrate` handler gains a Phase 2e step between RT and the
  unevaluated fallback: if RT returns `None` and the log-part
  denominator is a degree-2 irreducible polynomial, `arctan_integral`
  closes it. The progress gate was extended to treat a successful
  arctan result as progress (prevents infinite recursion).
- `atan` handler added to the VM handler table (evaluates `math.atan`
  numerically; leaves symbolic arguments unevaluated in symbolic mode).
- Depends on `coding-adventures-symbolic-ir ≥ 0.2.0` (adds `ATAN`).
- 25 new tests (`tests/test_arctan_integral.py`): pure imaginary
  denominators, completed-square denominators, mixed numerators
  (log + arctan), irrational discriminant (Sqrt in output), gating
  wrapper tests, and 6 end-to-end VM tests. The 1 existing test that
  expected an unevaluated `Integrate` for `1/(x²+1)` was updated to
  assert `Atan(x)`. Package at 229 tests, 90% coverage.

## 0.5.0 — 2026-04-19

Phase 2d of the integration roadmap — Rothstein–Trager. The log part
that Hermite reduction left as an unevaluated `Integrate` is now
emitted in closed form whenever every log coefficient happens to lie
in Q (the overwhelming majority of textbook cases). Integrands whose
coefficients escape Q — canonically `1/(x² + 1)` — still stay
unevaluated, awaiting a future `RootSum`/`RootOf` phase.

- New module `symbolic_vm.rothstein_trager`:
  - `rothstein_trager(num, den) → [(c_i, v_i), …] | None` produces
    the log-part pairs for ``∫ num/den dx = Σ c_i · log(v_i(x))`` or
    returns `None` when any coefficient escapes Q.
  - Builds the resultant ``R(z) = res_x(C − z·E', E) ∈ Q[z]`` by
    evaluation at ``deg E + 1`` nodes plus Lagrange interpolation —
    every internal arithmetic stays scalar over Q.
  - For each rational root ``α`` of ``R`` the log factor is
    ``v_α = monic(gcd(C − α·E', E))``; Rothstein's theorem guarantees
    the ``v_α`` are pairwise coprime and multiply back to monic(den).
- `Integrate` handler now routes the Hermite log-part through RT
  before falling back to unevaluated `Integrate`. The progress gate
  in `_integrate_rational` was generalised to treat a successful RT
  result as progress, so squarefree integrands like ``1/(x-1)`` now
  close in one step instead of bouncing into Phase 1.
- `_rt_pairs_to_ir` emits a left-associative binary `Add` chain of
  log terms; coefficients of ±1 collapse to bare `Log` / `Neg(Log)`,
  integer coefficients render as `IRInteger`, and non-integer
  rationals render as `IRRational`.
- Depends on `coding-adventures-polynomial ≥ 0.4.0` for the new
  `resultant` and `rational_roots` primitives.
- 12 new unit tests (`tests/test_rothstein_trager.py`) plus four
  end-to-end handler tests, bringing the package to 204 tests at
  90 % coverage. The RT module itself is at 100 %.

## 0.4.0 — 2026-04-19

Phase 2c of the integration roadmap — Hermite reduction. Rational
integrands now get their *rational part* in closed form; the log part
stays as an unevaluated `Integrate` with a squarefree denominator
(Rothstein–Trager, Phase 2d, will close it).

- New module `symbolic_vm.hermite`:
  - `hermite_reduce(num, den) → ((rat_num, rat_den), (log_num, log_den))`
    performs the classical Hermite reduction on a proper rational
    function over Q. The log-part denominator is guaranteed squarefree.
  - The correctness gate (and the universal unit-test invariant) is
    the re-differentiation identity
    `d/dx(rat_num / rat_den) + log_num / log_den == num / den`.
- `Integrate` handler grows a pre-check that routes rational
  integrands with non-constant denominators through
  `to_rational → polynomial division → hermite_reduce → from_polynomial`.
  Pure polynomials still go through the Phase 1 linear-recursion path
  (preserves the existing IR shape the rest of the test suite and
  downstream consumers are written against).
- Depends on `coding-adventures-polynomial ≥ 0.3.0` for the new
  `extended_gcd` primitive.
- `from_polynomial` now emits a left-associative binary `Add` chain —
  the arithmetic handlers are strictly binary, so n-ary applies tripped
  the arity check on the first `vm.eval`. The bridge tests were
  updated to the new shape.
- 21 new tests (15 unit-level Hermite, 6 end-to-end handler), bringing
  the package to 187 tests and 90 % coverage.

## 0.3.0 — 2026-04-19

Phase 2b of the integration roadmap — the IR ↔ polynomial bridge.

- New module `symbolic_vm.polynomial_bridge`:
  - `to_rational(f, x)` — recognises rational functions of the named
    variable `x` and returns `(numerator, denominator)` as `Polynomial`
    tuples with `Fraction` coefficients. Returns `None` for anything
    outside Q(x) (transcendentals, symbolic or fractional exponents,
    floats, free symbols).
  - `from_polynomial(p, x)` — emits the canonical IR tree for a
    polynomial at `x`, matching the shape the existing differentiator
    and Phase 1 integrator already produce.
- No cancellation of common factors: `(x² − 1)/(x − 1)` round-trips
  verbatim. Hermite reduction (Phase 2c) is the right place for that.
- Adds a dependency on `coding-adventures-polynomial`.
- 51 new tests, 100 % coverage on the bridge.

## 0.2.0 — 2026-04-19

First phase of the integration roadmap toward Risch.

- New `Integrate` handler on `SymbolicBackend` (parallel to `D`)
  implementing the "reverse derivative table" integrator:
  - Constant rule, power rule (including `x^(-1) → log(x)`),
    linearity (`Add`, `Sub`, `Neg`), constant-factor `Mul`,
    `∫(a/b) dx` for constant denominator, `∫(a/x) dx`,
    `∫a^x dx = a^x / log(a)`.
  - Elementary direct forms: `sin`, `cos`, `exp`, `sqrt`,
    `log` (the hard-coded integration-by-parts case).
- Anything outside the rule set stays as `Integrate(f, x)` unevaluated.
- End-to-end tests cover `integrate(x^2, x)`, `integrate(sin(x), x)`,
  and the `diff(integrate(f, x), x) → f` fundamental-theorem roundtrip.

## 0.1.0 — 2026-04-18

Initial release.

- Generic tree-walking `VM` over `symbolic_ir` nodes.
- `Backend` ABC with `lookup`, `bind`, `on_unresolved`,
  `on_unknown_head`, `rules`, `handlers`, `hold_heads`.
- `StrictBackend`: Python-like semantics; raises on unbound names or
  unknown heads; requires arithmetic operands to be numeric.
- `SymbolicBackend`: Mathematica-like semantics; leaves unbound names
  as free symbols; applies identity/zero laws; knows calculus.
- Shared handler table for arithmetic (`Add`, `Sub`, `Mul`, `Div`,
  `Pow`, `Neg`, `Inv`), elementary functions (`Sin`, `Cos`, `Exp`,
  `Log`, `Sqrt`), comparisons, logic, assignment, and definition.
- `D` handler on the symbolic backend implements sum, difference,
  product, quotient, power, and chain rules.
- User-defined functions via `Define(name, List(params), body)` —
  the VM detects the bound record and performs parameter substitution.
- `If` is a held head; only the chosen branch is evaluated.
- End-to-end tests cover the full pipeline (MACSYMA source → tokens
  → AST → IR → evaluated result).
