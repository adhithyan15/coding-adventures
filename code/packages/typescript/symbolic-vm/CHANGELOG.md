# Changelog

## [0.20.0] — 2026-05-29

**Track K2 — n-variate Hensel factor bridge (TypeScript port).**

Wires the new `tryNVariateHensel` from cas-factor 0.3.0 into the
`Factor(...)` IR handler.  Mirrors the Python Track K1 bridge in
`symbolic-vm/cas_handlers.py` (PR #5590).

Algorithm (n ≥ 3, generic — not per-arity):

1. Identify all free variables in the input (`findNVariables`, bounded
   at 8 distinct symbols so a pathological input can't allocate
   gigantic sparse-dict keys).
2. Convert to an `NPoly` via `irToNpoly`.  Returns undefined for
   floats, foreign symbols, transcendentals (Sin/Log/…), or non-integer
   exponents.
3. Call `tryNVariateHensel`.  On success, convert each factor back to
   IR via `npolyToIr` using **left-nested binary Add/Mul** (the
   primitive Add/Mul handlers are strictly binary, so n-ary Apply
   nodes with three or more children would crash).
4. Hook into `factor_handler` AFTER the bivariate Hensel path, BEFORE
   the unevaluated-wrapper fallback.

Catches `x³ + y³ + z³ − 3xyz = (x+y+z)(x²+y²+z²−xy−yz−zx)`,
`(x+y+z)(x+2y+3z) = x²+3xy+4xz+2y²+5yz+3z²`, and similar trivariate
cases.  Falls through cleanly on `x² + y² + z² + 1` (irreducible),
`sin(x) + y + z` (transcendental), and so on.

### Added

- `tryNVariateHenselIr` — top-level IR glue mirroring Python
  `_try_n_variate_hensel_ir`.
- `findNVariables`, `irToNpoly`, `npolyToIr`, `foldBinary` — helpers
  mirroring `_find_n_variables`, `_ir_to_npoly`, `_npoly_to_ir`, and
  the left-nested-binary-fold convention.
- `tests/n_variate_factor.test.ts` — 6 end-to-end pipeline tests
  exercising `Factor(...)` over the VM: sum-of-cubes identity, linear
  product round-trip, irreducible fall-through, transcendental safety,
  bivariate regression, univariate regression.

### Changed

- `cas-factor` minimum bumped to 0.3.0 (n-variate Hensel landed there).
- `factor_handler` dispatch order: univariate → bivariate Hensel →
  n-variate Hensel → unevaluated wrapper.

## [0.19.0] — 2026-05-29

**Track G2 — symbolic-coefficient Weierstrass lift (TypeScript port).**

Generalises the Phase-34/35/36/37 Weierstrass substitution
``∫ c / (a + b·trig(α·x + β)) dx`` from concrete rational ``a, b`` to
symbolic ones.  When the numeric pattern returns `undefined` because
either coefficient is a free IR expression, the new helper consults
`vm.assumptions` for the sign of the discriminant ``a² − b²`` and,
upon finding a declared inequality / equality, emits the matching
arctan / log / degenerate closed form with symbolic
``Sqrt(a² − b²)`` (or ``Sqrt(b² − a²)``) in the result.  When no
assumption pins down the sign, the integral is left unevaluated.

This depends on the compound-relation extension to
`cas-simplify.AssumptionContext` shipped in cas-simplify 0.2.0 (same
PR, Track G2).  Mirrors Python `symbolic-vm` 0.74.0.

### Added

- New `assumptions: AssumptionContext` field on `VM` — published to
  Weierstrass helpers via a module-level current-assumptions mirror
  of Python's `_CURRENT_VM` ContextVar.
- Handlers `Assume(rel)` / `Forget(rel)` / `ForgetAll()` registered
  on the symbolic backend, threading user-declared facts through to
  `vm.assumptions`.  Both relational heads are added to the
  hold-evaluate set so the relation argument reaches the handler
  intact.
- `tryWeierstrassSymbolicCoefficients` — symbolic dispatcher,
  invoked after the numeric helper returns `undefined`.
- `weierstrassParseAPlusBSincosSymbolic` — symbolic sibling of the
  numeric parser.
- Branch emitters `tryWeierstrass{Arctan,Log,Degenerate}Symbolic`.
- New dependency on `@coding-adventures/cas-simplify`.

### Regression

The numeric Weierstrass path is tried first and unchanged; the
symbolic path explicitly bails out when both ``a`` and ``b`` are
numeric, so concrete-coefficient integrals continue to use the
arithmetic-folded numeric closed forms.  All 38 existing
Phase 34 / 35 / 36 / 37 tests still pass; 218 / 218 in the broader
TS symbolic-vm suite.

## [0.18.0] - 2026-06-06

### Added

- Port the Python Phase 23 `Exp(c*x^2)` integration fallback for exact
  rational, nonzero `c`, returning `Erf` for negative coefficients and
  `Erfi` for positive coefficients.

## [0.17.0] — 2026-06-06

### Added

- Port the Python Phase 23 Fresnel integration fallback for
  `Integrate(Sin(a*x^2), x)` / `Integrate(Cos(a*x^2), x)` and
  `q*%pi*x^2` variants into the TypeScript VM.
- Tighten the previous IBP fallthrough tests so `sin(x^2)` and `cos(x^2)`
  must now return `FresnelS` / `FresnelC` forms instead of accepting an
  unevaluated `Integrate(...)`.

## [0.16.0] — 2026-05-28

**Track E2 — generic tabular integration-by-parts fallback (TypeScript
port).**  Mirrors the Python `ibp_tabular.py` reference (Track E1) and
closes the cross-language gap for the `Integrate` handler.

When every shape-specific handler in `integrateIndefinite` has returned
`undefined` for a `Mul`-shaped integrand, the new `tryIbpTabular`
fallback makes a last-ditch attempt by **generic tabular IBP**:

```
For f = u(x) · w(x) with u polynomial in x:
  ∫ u·w dx = Σ_{k=0}^{N-1} (-1)^k · u^(k)(x) · I^(k+1)(w)
```

where N = deg(u) + 1.  The I-column entries `∫w, ∫∫w, ..., ∫^N w` come
from the recursive `integrateIndefinite` callback; any step that fails
to close abandons the partition.  Bounded by `IBP_MAX_FACTORS = 5`
(number of flattened Mul factors) and `IBP_MAX_POLY_DEGREE = 8` (degree
of the polynomial column).

### Added

- `tryIbpTabular(f, x, integrateFn, diffFn, simplifyFn)` — top-level
  fallback.  Returns the closed-form antiderivative or `undefined`.
- `ibpFlattenMul(node)` — flattens nested-binary `Mul(a, Mul(b, c))`
  trees so the IBP search isn't fooled by parse-tree grouping.
- `ibpMultiplyIr(factors)` — rebuilds a left-associative `Mul` chain.
- `ibpPolynomialDegree(node, x)` — returns the polynomial degree in x
  (`-1` for zero, `undefined` for non-polynomial).
- `ibpContainsIntegrate(node)`, `ibpIsZero(node)`, `ibpTrySplit(...)`,
  `ibpCombinations(n, k)` — implementation helpers.

### Changed

- `integrate()` handler now invokes `tryIbpTabular` as the **last**
  fallback before returning the unevaluated `Integrate(...)` form.
  Closed-form results are passed through `vm.eval` for simplification.

### Test plan

Six tests in `tests/ibp-tabular.test.ts`:

1. `∫ x·sin(x) dx` closes via tabular IBP — verified numerically
   against `sin(1) − cos(1)`.
2. `∫ x²·eˣ dx` closes via tabular IBP — verified against `2e² − 2`.
3. `∫ x³·cos(x) dx` closes — verified against trapezoidal rule.
4. Fallthrough: `∫ 1/x dx → log(x)` (IBP short-circuits — head is DIV).
5. Fallthrough: `∫ sin(x²) dx` stays unevaluated or returns Fresnel —
   IBP fabricates no bogus elementary form.
6. Regression: `∫ cos(x²) dx` (Fresnel family) still stays unevaluated
   after the IBP port lands.

## [0.15.0] — 2026-05-28

**Track D2 — bivariate Hensel lifting in `Factor` (TypeScript port).**

Wires the new `@coding-adventures/cas-factor` 0.2.0 `tryBivariateHensel`
into the `Factor` head's multivariate fall-through chain.  When none of
the existing pattern handlers (perfect square/cube, difference of
squares, cubic identity, grouping, common-factor) recognise the input,
the handler now converts the IR to `BiPoly`, calls
`tryBivariateHensel`, and emits a `Mul(...)` of the lifted factors.
Mirrors the Python `_try_bivariate_hensel_ir` glue in
`symbolic-vm/cas_handlers.py`.

### Added

- `findTwoVariables(node)` — walks the IR tree, returns the first two
  distinct free variables or `undefined` (third variable, transcendental
  constant, etc. all disqualify).
- `irToBipoly(node, x, y)` — converts the polynomial subset of IR
  (`Add`, `Sub`, `Mul`, `Pow`, `Neg`, `Integer`, `Rational`, symbol) to
  a sparse `BiPoly`.  Returns `undefined` for floats, transcendentals,
  non-integer or negative exponents, foreign symbols.
- `bipolyToIr(p, x, y)` — converts a `BiPoly` back to IR with
  deterministic descending-degree term order.
- `tryBivariateHenselIr(inner)` — the top-level glue.

### Changed

- `factorHandler` — when the multivariate pattern path (`Apply`-level
  pattern recognisers) finishes without producing a factorisation, the
  handler now tries `tryBivariateHenselIr` before falling through to
  unevaluated `Factor(...)`.

## [0.14.0] — 2026-05-28

**Track B3 — Apart for repeated linear factors (Phase 48, TypeScript port).**

Lifts the multiplicity > 1 bail introduced in Track B1.
``Apart(P(x)/Q(x), x)`` now decomposes rational functions whose denominator
factors as ``∏_r (x − r)^{m_r}`` for *rational* ``r`` with arbitrary
multiplicity.  Each pole ``r`` of multiplicity ``m`` contributes terms
``A_{r,1}/(x − r) + A_{r,2}/(x − r)² + … + A_{r,m}/(x − r)^m`` where the
coefficients come from the Taylor expansion of
``φ(t) = P(r + t)/Q(r + t)`` around ``t = 0`` with
``Q(x) = den(x)/(x − r)^m``.  Then ``A_{r, m − j} = φ_j``.

This mirrors the Phase 48 algorithm added to Python ``symbolic-vm`` in PR
\#3927.  Acceptance: ``Apart(1/(k²(k+1)²), k)`` decomposes to
``2/(k+1) + 1/(k+1)² − 2/k + 1/k²`` (left-associated, roots sorted
ascending), matching the Python reference byte-for-byte.

Denominators that still contain an irreducible quadratic factor on top of
the rational roots continue to bail to the unevaluated ``Apart(...)``
form — partial fractions over the rationals can't go further there.

### Added

- ``polyTaylorExpandAroundR`` — Taylor-expand a ``PolyQ`` around a
  rational point ``r`` to ``length`` coefficients.  Uses the binomial
  identity ``poly(r+t)_j = ∑_{i≥j} c_i · C(i, j) · r^(i−j)`` with exact
  arbitrary-precision ``BigInt`` arithmetic.
- ``polySeriesDiv`` — formal power-series division ``N(t)/D(t)`` to
  ``length`` terms via the standard recurrence
  ``Q_j = (N_j − ∑_{k≥1} D_k · Q_{j−k}) / D_0``.  Returns ``undefined``
  when ``D(0) = 0`` (defensive guard against a repeated-root miscount).
- ``buildApartTerm`` — IR builder for ``A / (x − r)^power`` with
  ``±1`` numerator elision (matches the formatting in
  ``apartSimpleRoots``).
- ``binomialBig`` — exact ``BigInt`` binomial helper used by the
  Taylor expansion.

### Changed

- ``apartProper`` — Phase 48 generic path lifted in: when any
  multiplicity > 1, compute ``Q(x) = den(x)/(x − r)^m`` per root via
  successive division, Taylor-expand ``num`` and ``Q`` around ``r``,
  series-divide, and emit ascending-power terms via ``buildApartTerm``.
  Phase 1 simple-roots fast path retained (cheaper than Taylor + series
  division and preserves the existing B1 regression-test IR shapes).
- ``polyQRationalRoots`` — the ``a₀ = 0`` (x = 0 is a root) branch now
  sorts its returned roots ascending, matching the non-zero-root path
  and the Python ``sorted(roots)`` reference.  B1's simple-root tests
  never exercised this branch; Phase 48 needs a stable order across
  multi-root denominators including ``x = 0``.

### Removed

- The ``mult > 1 → undefined`` bail in ``apartProper`` — the new code
  path handles the repeated-root case directly.

### Out of scope (deferred)

- Irreducible quadratic factors (``Apart`` over the rationals only).
- Algebraic-number roots beyond Q — would require an irrational-roots
  extension.

## [0.13.0] — 2026-05-28

**Track B1 — Apart simple-roots partial-fraction decomposition (TypeScript port).**

Ports the Phase 1 simple-root subset of Python's ``apart_handler`` from
``symbolic-vm/cas_handlers.py``.  ``Apart(P(x)/Q(x), x)`` now decomposes
rational functions whose denominator has only *distinct rational* roots
using the residue formula ``A_i = P(r_i) / Q'(r_i)``.  Improper fractions
(deg P ≥ deg Q) get a polynomial-division step first, then Apart on the
proper remainder.  Repeated roots (Phase 48 in the Python tree) and
denominators with irreducible quadratic factors leave the expression
wrapped in ``Apart(...)`` for downstream pipelines to handle.

This unblocks the deferred TS port of the Phase 40 / 46 Apart-retry
telescope chain in ``cas-summation``.

### Added

- ``apartHandler`` registered under the ``"Apart"`` head in the symbolic
  backend's handler table.
- Self-contained ``RatQ`` (BigInt rational) coefficient type plus
  polynomial primitives ``polyQNormalize`` / ``polyQDegree`` /
  ``polyQEvaluate`` / ``polyQDeriv`` / ``polyQDivmod`` /
  ``polyQRationalRoots`` / ``polyQRootMultiplicities``.
- IR ↔ polynomial bridges ``toRational`` and ``fromPolynomial``,
  mirroring ``polynomial_bridge.py`` (left-associated ``Add`` chains,
  ±1 coefficient elision, zero-term skipping).
- ``apartSimpleRoots`` + ``apartProper`` implementing the residue-formula
  decomposition.  ``apartProper`` bails to ``undefined`` (caller emits
  unevaluated ``Apart(...)``) when *any* multiplicity > 1 — Phase 48 is
  explicitly out of scope for this PR.
- 6 new test cases in ``tests/apart.test.ts`` mirroring the Track B1
  acceptance cases in ``code/specs/macsyma-finish-plan.md``.

### Out of scope (deferred to follow-on tracks)

- Repeated linear factors (Phase 48 algorithm) — Track B3.
- Apart-retry telescope chain (Phase 40 + 46 composition) — Track B2.

## [0.12.0] — 2026-05-22

**Phase 47 — Nested-Add flattening (TypeScript port).**

Ports the Python ``symbolic-vm`` 0.71.0 Add-handler fix.  When either
binary ``Add`` operand is itself an ``Add(...)`` apply, the handler
now flattens the tree, sums numeric literals once, and rebuilds a
left-associated chain.  Example:

    Add(Add(k, 1), 1)  →  Add(k, 2)
    Add(Add(Add(k, 1), 1), 1)  →  Add(k, 3)

Why it matters: makes ``Add`` canonical for any consumer that
compares trees structurally.  In particular, downstream
``cas-summation`` users get reliable structural-equality matches in
the telescope detector even when ``Apart`` (in Python) or hand-
written shifted summands (anywhere) produce nested ``Add`` forms.

### Changed

- ``buildHandlerTable``'s ``ADD`` entry now wraps the existing
  ``binaryNumeric`` handler with a pre-pass that:
  1. Detects nested ``Add`` operands.
  2. Walks the tree via the existing ``flattenAddTerms`` helper.
  3. Partitions leaves into numerics vs symbolics.
  4. Sums numerics once (priority handled by ``addNumeric``).
  5. Rebuilds a left-associated chain
     ``Add(...non_literals, lit_sum)`` (dropping the literal if it's
     zero, collapsing to the bare symbol if only one operand remains).

Strict mode (``simplify=false``) keeps the original binary
semantics.

### Added — tests

`tests/symbolic-vm.test.ts` — new ``Phase 47: nested-Add flattening``
describe block with 7 cases:

- ``Add(Add(k, 1), 1)`` → ``Add(k, 2)``.
- Triply-nested ``Add(Add(Add(k, 1), 1), 1)`` → ``Add(k, 3)``.
- ``Add(Add(k, 2), 3)`` → ``Add(k, 5)`` (constant folding).
- ``Add(Add(k, 1), -1)`` → bare ``k`` (literal zeroes out).
- ``Add(Add(x, y), z)`` — no literals — stays as left-associated
  chain (pin: no spurious reordering).
- ``Add(k, 1)`` — non-nested — untouched (no rebuild).
- Regression: ``Add(0, x)`` still simplifies to ``x``.

Full suite: **161 passed** (was 154; +7 net new).

## [0.11.0] — 2026-05-20

### Added — Phase 38: Weierstrass closed forms lifted to linear trig arguments

Mirrors Python `symbolic-vm` 0.63.0 (PR #3690).

The previous Phases 34–37 closed Weierstrass for
`∫ c / (a + b·trig(x)) dx` in all discriminant regimes (`a² > b²` arctan,
`a² = b²` degenerate, `a² < b²` log) but only when the trig argument was
the bare variable `x`.  Phase 38 generalises every branch to accept any
linear-in-`x` rational argument `α·x + β` (with `α, β ∈ ℚ`, `α ≠ 0`).

The mathematics is a single inner change of variable: with
`u = α·x + β` we have `du = α · dx`, so

    ∫ c / (a + b·sin(α·x + β)) dx
        = (1/α) · ∫ c / (a + b·sin u) du  (Phase 34/36/37 closed form in u)

The closed form is the existing one with `tan((α·x + β)/2)` substituted
for `tan(x/2)` and the outer constant scaled by `1/α`.  When `α = 1`
and `β = 0`, the new code path is bit-for-bit identical to the Phase
34–37 behaviour — full backwards compatibility.

### Added

- **`weierstrassParseLinearInX(node, x)`** in `src/index.ts` — parses a
  node into `{ alpha, beta }` Numeric pair (Int/Rat) when it represents
  `α·x + β`.  Handles bare `x`, scalar multiples, ADD/SUB with any
  operand ordering, and leading `Neg` wrappers.  Rejects nonlinear
  (`x²`) and pure-constant shapes by returning `undefined`.  `α = 0`
  is filtered out so callers may rely on `α ≠ 0` throughout.
- **`weierstrassBuildLinearArgIR(α, β, x)`** — builds the IR for
  `α·x + β`, collapsing trivial cases (`α=1, β=0 → x`, etc.) so the
  emitted `tan(arg/2)` carries the simplest equivalent argument.
- **`weierstrassParseConstTimesTrigLinear`** — supersedes the Phase 34
  bare-`x` predecessor.  Returns `{ c, head, alpha, beta }` for any
  shape matching `c·sin(α·x + β)` or `c·cos(α·x + β)`.

### Changed

- **`weierstrassParseAPlusBSincos`** — now returns
  `{ a, b, trigHead, alpha, beta }` instead of `{ a, b, trigHead }`.
- **`tryWeierstrassDegenerate`, `tryWeierstrassLogForm`,
  `tryWeierstrassOneOverLinearTrig`** — accept an `argNode: IRNode`
  parameter representing `α·x + β` and substitute it into the
  `tan(arg/2)` construction.  The outer `c ← c/α` scaling is applied
  once at the dispatcher entry, so each branch's closed form is
  otherwise unchanged.

### Added — tests

`tests/phase34-weierstrass.test.ts` — 10 new cases:

- `∫ 1/(2 + sin 2x) dx` (promoted from the prior Phase 34 deferral).
- `∫ 1/(2 + cos 3x) dx` — α = 3 cos variant.
- `∫ 1/(2 + sin(x + 1)) dx` — pure phase shift.
- `∫ 1/(2 + sin(2x + 1)) dx` — full α = 2, β = 1 case.
- Rational α = 1/2 and negative α = −2.
- Degenerate `(1 + cos 2x)` and log-form `(1 + 2·sin 2x)` under
  substitution.
- Fallthrough tests for nonlinear `sin(x²)` and symbolic `sin(α·x)`.

All 154 tests pass.

### Still deferred

- Symbolic coefficients (`a`, `b`, `α`, or `β` non-numeric) — needs an
  assumption context to decide discriminant sign.
- Trig argument involving `x²` or other nonlinear forms — out of scope
  for Weierstrass.

## [0.10.0] — 2026-05-20

### Changed — Phase 37: Weierstrass log form cos branch covers `b < −|a|`

Mirrors Python `symbolic-vm` 0.62.0 (PR #3683).

Phase 36 (0.9.0) emitted the log-form closed solution for `a² < b²`
but explicitly deferred the cos branch with `b < −|a|`.  A closer
derivation shows the same formula

    (c/D) · log|(D + (b−a)·tan(x/2)) / (D − (b−a)·tan(x/2))|

already handles both sign regimes correctly when wrapped in `Abs`.

`tryWeierstrassLogForm` cos branch: removed the
`subNumeric(b, absNumeric(a))` and `subNumeric(b, a)` positivity guards.
The remaining caller-side guarantee (`disc < 0`, i.e. `b² > a²`) is
sufficient.  `absNumeric` is retained (still referenced) for any future
symmetric guards.

Tests: 3 new in `tests/phase34-weierstrass.test.ts`:
- Promoted: `∫ 1/(1 − 2·cos x) dx` (formerly deferred)
- `∫ 1/(−1 − 3·cos x) dx` (negative a, negative b)
- `∫ 5/(1 − 2·cos x) dx` (numerator scaling)

Full suite: **145 tests pass** (143 prior + 3 new − 1 promoted).

## [0.9.0] — 2026-05-20

### Added — Phase 36: Weierstrass log form for `a² < b²`

Ports Python `symbolic-vm` 0.61.0 (PR #3672) Phase 36 to TypeScript.
Closes the deferred `a² < b²` branch of Phase 34 by emitting the
explicit log-form closed solution.

After the substitution `u = tan(x/2)` the quadratic in `u` has two
distinct real roots; partial fractions give:

    ∫ c/(a + b·sin x) dx = (c/D)·log|(a·tan(x/2)+b−D)/(a·tan(x/2)+b+D)| + C
    ∫ c/(a + b·cos x) dx = (c/D)·log|(D+(b−a)·tan(x/2))/(D−(b−a)·tan(x/2))| + C

where `D = √(b²−a²) > 0`. The sin formula is valid for any nonzero
rational `a`. The cos formula requires `b > |a|` strictly; the
symmetric `b < −|a|` case is deferred.

The log argument is wrapped in `Abs()` (`sym("Abs")`) because the inner
rational changes sign across the integrand's singularities; this lets
the closed form be evaluated numerically across the full domain.

#### Added

- **`tryWeierstrassLogForm(c, a, b, trigHead, x)`** — Phase 36 helper.
  Plumbed into the existing Phase 34 dispatcher in place of the prior
  `disc < 0 → undefined` arm.
- **`absNumeric(v)`** — numeric absolute value helper for Int/Rat/Float.

#### Tests

5 new `it()` cases in `tests/phase34-weierstrass.test.ts` + 1 promoted
from fallthrough:

- ∫ 1/(1 + 2·sin x) dx — sin branch closes
- ∫ 1/(1 + 2·cos x) dx — cos branch with `b > |a|`
- ∫ 1/(−1 + 2·sin x) dx — sin with `a < 0`
- ∫ 3/(1 + 2·sin x) dx — numerator scaling
- ∫ 1/(3 + 5·sin x) dx — perfect-square `|disc|=16` folds Sqrt away
- ∫ 1/(1 − 2·cos x) dx — `b < |a|` cos branch still deferred

Full suite: **143 tests pass** (138 prior + 5 net new).

## [0.8.0] — 2026-05-18

### Added — Phase 35: degenerate `a² = b²` Weierstrass cases

Closes the four degenerate branches that Phase 34 (0.7.0) deliberately
deferred:

    ∫ 1/(a + a·sin x) dx = -2 / (a · (tan(x/2) + 1))
    ∫ 1/(a − a·sin x) dx =  2 / (a · (1 − tan(x/2)))
    ∫ 1/(a + a·cos x) dx =  tan(x/2) / a
    ∫ 1/(a − a·cos x) dx = -1 / (a · tan(x/2))     (= -cot(x/2)/a)

Each formula is exact — no `Sqrt`, no `Atan` — because the
post-substitution quadratic in `u = tan(x/2)` factors as `a(u ± 1)²`
for sin and reduces to `2a` (constant) or `2a·u²` for cos.

#### Added (`src/index.ts`)

- **`tryWeierstrassDegenerate(c, a, b, trigHead, x)`** — Phase 35
  helper.  Pattern-matches the four `(b == a, b == -a) × (SIN, COS)`
  combinations and emits the corresponding closed form via `app(DIV, ...)`
  with `tan(x/2)` inside.  Returns `undefined` for `a == 0` (zero
  denominator).

- **`isZeroNumeric(v)`** — strict-zero predicate complementing
  `isPositiveNumeric`.  Used to detect `disc == 0` after the existing
  `subNumeric(mulNumeric(a, a), mulNumeric(b, b))`.

- **`eqNumeric(a, b)`** — exact equality on `Numeric` (int/rat),
  used to test `b == a` and `b == -a`.  Float values are conservatively
  rejected so they only ever take the strictly-positive arctan path.

- Updated `tryWeierstrassOneOverLinearTrig` (Phase 34) to call
  `tryWeierstrassDegenerate` when `isZeroNumeric(disc)` and to return
  `undefined` (defer) when `!isPositiveNumeric(disc)` (i.e. `disc < 0`,
  log form, still open).

#### Tests (`tests/phase34-weierstrass.test.ts`)

5 new `it()` cases under a new `Phase 35: degenerate a² = b² cases`
describe block + 1 promoted from fallthrough to closed form:

- ∫ 1/(2 − 2·sin x) dx — sin, b = −a.
- ∫ 1/(1 + cos x) dx — cos, b = a → tan(x/2).
- ∫ 1/(1 − cos x) dx — cos, b = −a → −cot(x/2).
- ∫ 5/(2 + 2·sin x) dx — numerator coefficient scaling.
- ∫ 1/(3/2 + (3/2)·cos x) dx — rational a = b.
- Promoted: ∫ 1/(1 + sin x) dx now closes (was previously asserted
  to stay unevaluated).

Each verifies the closed form via numerical differentiation, avoiding
the `tan(x/2)` pole at `x = π` and the `1/(1−cos x)` pole at `x = 0`.

Full suite: 84 passed (79 prior + 5 net new).

### Note on version sequencing

This release builds on the in-flight Phase 34 (PR #3473, 0.7.0) which
itself jumped 0.5.0 → 0.7.0 to leave 0.6.0 for the in-flight Phase
29-33 port (PR #3468).  Final order when all four PRs land:
0.5.0 → 0.6.0 → 0.7.0 → 0.8.0.

## [0.7.0] — 2026-05-18

### Added — Phase 34: Weierstrass substitution for ∫ 1/(a + b·sin/cos x) dx

Ports the Python `symbolic-vm` 0.59.0 Phase 34 work to TypeScript.  The
substitution `u = tan(x/2)` produces `sin(x) = 2u/(1+u²)`,
`cos(x) = (1−u²)/(1+u²)`, `dx = 2/(1+u²) du` and reduces the two
canonical denominator shapes to a rational function in `u` that
integrates to an arctan whenever `a² > b²` (denominator never zero on
ℝ).  Closed forms:

    ∫ 1/(a + b·sin x) dx  =  (2/√(a²−b²)) · arctan((a·tan(x/2) + b)/√(a²−b²))
    ∫ 1/(a + b·cos x) dx  =  (2/√(a²−b²)) · arctan(√((a−b)/(a+b)) · tan(x/2))

For exact-rational `a, b` satisfying `a² > b²` (and `a > 0` for the cos
branch) the integrator now closes the form directly.  A numerator
constant `c` simply scales the result.

#### Deferred to a later phase

- `a² < b²` — log form on `(a·tan(x/2)+b ± √(b²−a²))` (sign analysis).
- `a² = b²` — degenerate, reduces to a rational in `tan(x/2)`.
- `a ≤ 0` for the cos case — `(a−b)/(a+b)` sign analysis.
- Symbolic `a` or `b` — discriminant sign undecidable without an
  assumption context (the TS port has no assumption system).
- Non-bare trig arguments (e.g. `sin(2x)`) — composition with a future
  linear-substitution phase will pick this up.

#### Added

- **`tryWeierstrassOneOverLinearTrig(integrand, x)`** — Phase 34 entry
  point.  Matches `Div(c, Add(a, ...))` shapes where the `Add` resolves
  to `a + b·sin(x)` or `a + b·cos(x)` and `c, a, b` are exact rationals.
  Wired into the `DIV` branch of `integrateIndefinite` after the
  existing constant-numerator and `1/x` cases.

- **`weierstrassParseAPlusBSincos(node, x)`** — structural matcher
  returning `{ a, b, trigHead }` for the four canonical operand
  orderings (Add/Sub × constant-left/right).  Reuses the existing
  `toNumeric` / `negNumeric` helpers.

- **`weierstrassParseConstTimesTrigX(node, x)`** — matches `c·sin(x)`,
  `c·cos(x)`, `sin(x)`, `cos(x)`, and their Neg-wrapped variants.

- **`weierstrassSqrtFractionIR(f)`** — emits `Sqrt(p/q)` IR, folding to
  a clean rational when both `p` and `q` are perfect integer squares
  (uses the existing `bigIntIsqrt` helper).

- **`isPositiveNumeric(v)`** — strict-positive predicate for Numeric.

#### Tests

`tests/phase34-weierstrass.test.ts` (14 cases mirroring Python's
`test_phase34_weierstrass.py`):

- Closed-form structure: ∫ 1/(2 + sin x) contains Atan in the body.
- Numeric-derivative verification at 5–7 sample points across the
  open interval where tan(x/2) is finite.
- Perfect-square discriminant folds Sqrt away (a=5, b=3 → disc=16; cos
  case has ratio 1/4 as well).
- Numerator coefficient scales the closed form (∫ 3/(2 + sin x)).
- Rational coefficients (a=3/2, b=1/2; disc=2).
- Operand-order robustness (∫ 1/(sin x + 2) still closes).
- Four fallthrough guarantees: a²<b², a²=b², non-bare argument,
  symbolic `a`.
- Regression: ∫ sin(x) dx = −cos(x) unchanged; ∫ 1/cos(x) dx is NOT
  misinterpreted as a Weierstrass case.

## [0.6.0] — 2026-05-18

### Added — Phases 29–33: algebraic simplification rules

Extends the symbolic backend with five new rule families that fire on
every re-evaluation of the affected expressions.

#### Phase 29 — Abs and Sqrt algebraic rules

**`Abs(x)`** (new handler — `Abs` head previously fell through unhandled):
- Numeric fold: `Abs(-3) → 3`, `Abs(3/4) → 3/4`.
- Idempotency: `Abs(Abs(x)) → Abs(x)`.
- Negation strip: `Abs(-x) → Abs(x)` (detects `Neg` head).
- Mul-neg strip: `Abs(Mul(-1, x)) → Abs(x)`.
- Even-power identity: `Abs(x^{2k}) → x^{2k}` for integer 2k ≥ 2.

**`Sqrt(x)`** (replaces numeric-only `elementary()` factory):
- Perfect-square detection: `sqrt(4) → 2`, `sqrt(9) → 3`, etc.
- Even-exponent rewrite `sqrt(x^{2k})`:
  - k even → `x^k`  (e.g. `sqrt(x^4) = x^2`)
  - k odd → `Abs(x^k)` (e.g. `sqrt(x^2) = |x|`, `sqrt(x^6) = |x^3|`)

#### Phase 30 — Log / Exp cancellation rules

**`Log(x)`**:
- `log(exp(x)) → x`  (structural cancellation, unconditional for real domain).
- Special value `log(1) → 0` preserved.

**`Exp(x)`**:
- `exp(log(x)) → x`.
- `exp(n·log(x)) → x^n`  (handles both `Mul(n, log(x))` and `Mul(log(x), n)`).
- Special value `exp(0) → 1` preserved.

**Regression note**: the derivative of `x^x` now simplifies to `x^x·(log(x)+1)`
instead of `exp(x·log(x))·(log(x)+1)` because `exp(x·log(x))` is eagerly reduced
to `x^x`.  The test expectation was updated accordingly.

#### Phase 31 — Trig / hyperbolic negation symmetry and arc-cancellation

**Odd functions** (`sin`, `tan`, `sinh`, `tanh`):
- `f(-x) → -f(x)` with recursive descent so double negations collapse.

**Even functions** (`cos`, `cosh`):
- `f(-x) → f(x)` (Neg stripped).

**Arc-cancellation** (all six primary trig/hyperbolic functions):
- `sin(asin(x)) → x`,  `cos(acos(x)) → x`,  `tan(atan(x)) → x`
- `sinh(asinh(x)) → x`,  `cosh(acosh(x)) → x`,  `tanh(atanh(x)) → x`

#### Phase 32 — Inverse trig / hyperbolic odd symmetry

**Odd** (`asin`, `atan`, `asinh`, `atanh`):
- `f(-x) → -f(x)`.

**Acos reflection** (`acos`):
- `acos(-x) → %pi − acos(x)` (`%pi` is `IRSymbol("%pi")`).

**Acosh** has no symmetry rule (domain `[1, ∞)`) and keeps numeric-fold only.

#### Phase 33 — Trig exact values at rational multiples of π

`sin(q·%pi)`, `cos(q·%pi)`, and `tan(q·%pi)` return exact algebraic values
when `q` is a rational number with denominator in `{1, 2, 3, 4, 6}`.

**`tryPiMultiple(arg)`** detects:
- Float ≈ q·π (denominators 1, 2, 3, 4, 6).
- Structural: `%pi`, `Neg(%pi)`, `Mul(n, %pi)`, `Div(%pi, n)`,
  `Div(Mul(n, %pi), d)` (both Mul orderings).

**Lookup tables** (period 2π for sin/cos, period π for tan):

| q | sin(q·π) | cos(q·π) | tan(q·π) |
|---|----------|----------|----------|
| 0 | 0 | 1 | 0 |
| 1/6 | 1/2 | √3/2 | √3/3 |
| 1/4 | √2/2 | √2/2 | 1 |
| 1/3 | √3/2 | 1/2 | √3 |
| 1/2 | 1 | 0 | undefined |
| 2/3 | √3/2 | −1/2 | −√3 |
| 3/4 | √2/2 | −√2/2 | −1 |
| 5/6 | 1/2 | −√3/2 | −√3/3 |
| 1 | 0 | −1 | 0 |

`tan(π/2)` is left unevaluated (undefined).

**Tests added** (54 new tests across all 5 phases):
- Phase 29: 9 tests (abs fold/idempotency/strip/even-power; sqrt perfect-square/even-power)
- Phase 30: 7 tests (log/exp cancellation and power form)
- Phase 31: 12 tests (trig+hyperbolic symmetry and arc-cancellation)
- Phase 32: 6 tests (inverse trig odd symmetry + acos reflection)
- Phase 33: 20 tests (sin/cos/tan π-multiples including negative q and regression)

## [0.5.0] — 2026-05-18

### Added — Phase 28: general IBP for poly×log(Q) and poly×atan(Q)

Extends symbolic integration to handle products of a polynomial `P(x)` with
`log(Q(x))` or `atan(Q(x))` where `Q(x)` is a **non-linear** polynomial with
rational coefficients.  Uses the IBP formula:

  ∫ P·log(Q) dx  =  R·log(Q) − ∫ R·Q′/Q dx
  ∫ P·atan(Q) dx =  R·atan(Q) − ∫ R·Q′/(1+Q²) dx

where R = ∫P (polynomial antiderivative, constant = 0).

**New functions:**

- `tryLogPolyProduct(transcendental, poly, x)` — Phase 28 log IBP handler;
  skips linear Q (deferred to Phase 3) and delegates the residual to
  `integrateRationalSimple`.
- `tryAtanPolyProduct(transcendental, poly, x)` — Phase 28 atan IBP handler;
  skips linear Q (deferred to Phase 11 if/when implemented) and delegates
  the residual to `integrateRationalSimple`.
- `integrateRationalSimple(N_ir, D_ir, x)` — targeted rational function
  integrator for Phase 28 residuals.  After polynomial long division:
  - **Case A**: remainder = c·D′ → c·log(D)
  - **Case B**: constant remainder / quadratic ax²+b with rational √(b/a)
                → r₀/(a₂·√(a₀/a₂))·atan(x/√(a₀/a₂))
- `closeRemainderOverD(R, D, D′, D_ir, x)` — attempts Cases A/B for the
  post-division remainder polynomial.
- `evalNumericNode(node)` — evaluates a closed IR numeric expression
  (handling MUL/DIV/NEG/ADD/SUB of exact rationals) to a `Numeric` value;
  used by `rpFromCoeffsMap` to extract rational coefficients from compound
  coefficient nodes produced by `toPolynomialCoeffs`.

**Rational polynomial arithmetic helpers** (used internally by Phase 28):
`rc`, `rcAdd`, `rcSub`, `rcMul`, `rcDiv`, `rcToIR`, `rpDeg`, `rpCoeff`,
`rpAdd`, `rpMul`, `rpDeriv`, `rpIntegrate`, `rpDiv`, `rpToIR`,
`rpFromCoeffsMap`, `rpProportional`, `bigIntSqrt`, `rcSqrt`, `isLinearIn`.

**Dispatch wiring:**
- MUL branch: after Phase 27, tries `tryLogPolyProduct(a,b,x)` and
  `tryAtanPolyProduct(a,b,x)` (and symmetric variants) for both-depend cases.
- Bare function path: `∫ log(Q) dx` (P=1) and `∫ atan(Q) dx` (P=1) are
  detected via head checks before the final `return undefined`.

**Examples that now evaluate:**
- `∫ log(x²+1) dx` = x·log(x²+1) − 2x + 2·atan(x)
- `∫ x·log(x²+1) dx` = (x²/2)·log(x²+1) − x²/2 + ½·log(x²+1)
- `∫ x²·log(x²+1) dx` = (x³/3)·log(x²+1) − 2x³/9 + 2x/3 − (2/3)·atan(x)
- `∫ x·atan(x²) dx` = (x²/2)·atan(x²) − ¼·log(1+x⁴)

**Fallthrough cases** (correctly left unevaluated):
- `∫ atan(x²) dx` — residual 2x²/(1+x⁴) requires irrational partial fractions
- `∫ x²·atan(x²) dx` — same reason

**Tests added:**
- `Phase 28: ∫ log(x²+1) dx` — closed-form structure and numerical check
- `Phase 28: ∫ x·log(x²+1) dx` — closed-form and numerical check
- `Phase 28: ∫ x²·log(x²+1) dx` — numerical check
- `Phase 28: ∫ atan(x²) dx fallthrough` — stays unevaluated
- `Phase 28: ∫ x·atan(x²) dx` — closed-form structure and numerical check
- `Phase 28: regression — ∫ log(x) dx still handled by Phase 3`
- `Phase 28: regression — ∫ atan(x) dx not intercepted by Phase 28`

## [0.4.0] — 2026-05-16

### Added — Phase 26: log-power integration via IBP reduction

- `polyLogPowerTerm(k, n, x)` — closed form of `∫ xᵏ · log(x)^n dx` for
  integer k ≥ 0, n ≥ 1, using the IBP reduction formula:
  `G_{k,m}(x) = x^(k+1)/(k+1) · log(x)^m − m/(k+1) · G_{k,m-1}(x)`.
- `tryLogPowerProduct(transcendental, poly, x)` — handles `∫ Q(x) · log(x)^n dx`
  for integer n ≥ 2 by decomposing Q(x) into monomials and applying
  `polyLogPowerTerm` term-by-term.
- `toPolynomialCoeffs(expr, x)` — utility that extracts a `Map<degree, coeff>`
  polynomial coefficient map from an IR expression; handles constants, `x`,
  `x^k`, `c·f`, `f·c`, ADD, SUB, NEG.
- Integration of standalone `log(x)^n` (n ≥ 2) via `polyLogPowerTerm(0, n, x)`.

### Added — Phase 27: trig-of-log integration via u = log(x) substitution

- `trigLogIntegral(trigHead, k, x)` — closed form of `∫ xᵏ · trig(log(x)) dx`
  via the identity `∫ e^((k+1)u) trig(u) du` (with u = log x):
  - `∫ xᵏ sin(log x) dx = x^(k+1)·((k+1)sin(log x)−cos(log x))/((k+1)²+1)`
  - `∫ xᵏ cos(log x) dx = x^(k+1)·((k+1)cos(log x)+sin(log x))/((k+1)²+1)`
- `tryTrigLogProduct(transcendental, poly, x)` — handles `∫ Q(x)·sin(log(x)) dx`
  and `∫ Q(x)·cos(log(x)) dx` by decomposing Q(x) and applying `trigLogIntegral`
  term-by-term.
- Integration of standalone `sin(log(x))` and `cos(log(x))` (k = 0 case):
  - `∫ sin(log x) dx = x/2·(sin(log x)−cos(log x))`
  - `∫ cos(log x) dx = x/2·(sin(log x)+cos(log x))`

## [0.3.1] — 2026-05-14

**Bug fix: elliptic modulus extraction now handles pre-evaluated numeric `k²`.**

`modulusFromSquaredFactor` previously only recognised `Pow(k, 2)` as the squared
modulus factor.  The MACSYMA compiler (and TypeScript IR evaluator) eagerly folds
`(1/2)^2` → `IRRational(1/4)` and `0.5^2` → `IRFloat(0.25)` before the
integration handler runs, so the pattern was never matched.

Extended the recogniser to extract `k` from:
- `IRFloat(v)` — returns `IRFloat(√v)`; e.g. `0.25` → `0.5`
- `IRRational(p/q)` where both numerator and denominator are perfect squares
  — returns `IRRational(√p / √q)`; e.g. `1/4` → `1/2`
- `IRInteger(n)` where `n` is a perfect square — returns `IRInteger(√n)`;
  e.g. `4` → `2`
- Non-perfect-square rationals/integers — falls back to `Sqrt(k²)` (unevaluated)

Added a new helper `bigIntIsqrt(n)` for exact integer square root over `bigint`.

Result: `integrate(sqrt(1-(1/2)^2*sin(theta)^2), theta, 0, %pi/2)` now returns
`EllipticE(1/2)` instead of falling through unevaluated.

## [0.3.0] — 2026-05-14

- Added EllipticE (second kind) integration recognition:
  - `∫₀^(π/2) sqrt(1-k²sin²θ) dθ` → `EllipticE(k)` (complete)
  - `∫ sqrt(1-k²sin²θ) dθ` → `EllipticE(θ, k)` (incomplete)
- Added EllipticPi (third kind) complete integration recognition:
  - `∫₀^(π/2) 1/((1+n·sin²θ)·sqrt(1-k²sin²θ)) dθ` → `EllipticPi(n, k)`
- New helper functions: `ellipticSecondKindRadicand`, `completeEllipticSecondKind`,
  `incompleteEllipticSecondKind`, `extractCharacteristicN`, `ellipticThirdKindParams`,
  `completeEllipticThirdKind`

## [0.2.0] — 2026-05-14

### Added

- Added symbolic integration recognition for the canonical elliptic first-kind
  forms, returning `EllipticF(theta, k)` and complete `EllipticK(k)` nodes.
- Added a bivariate perfect-cube factoring foothold so `Factor` recognises
  four-term binomial cube expansions: `x^3 + 3x^2y + 3xy^2 + y^3` as
  `(x+y)^3` and `x^3 - 3x^2y + 3xy^2 - y^3` as `(x-y)^3`.
- Added a canonical symbolic `Factor` handler backed by `cas-factor`, including
  a small common-symbolic-factor extraction pass for multivariate expressions
  like `x^2*y - y`.
- Extended the common multivariate factoring foothold to extract shared integer
  content as well as symbolic powers, so `2*x*y + 2*x*z` factors to
  `2*x*(y+z)`.
- Added a bivariate perfect-square factoring foothold so `Factor` recognises
  expressions like `x^2 + 2*x*y + y^2` as `(x+y)^2`.
- Added a bivariate difference-of-squares factoring foothold so `Factor`
  recognises expressions like `x^2 - y^2` as `(x-y)*(x+y)`.
- Added a bivariate cubic-identity factoring foothold so `Factor` recognises
  expressions like `x^3 - y^3` and `x^3 + y^3` as their textbook two-factor
  decompositions.
- Added a four-term bilinear grouping factoring foothold so `Factor`
  recognises expressions like `x*y + x*z + y + z` as `(x+1)*(y+z)`.
- Added a symbolic-backend-only `D` handler for pure IR differentiation,
  including arithmetic, power, elementary, hyperbolic, and inverse hyperbolic
  chain rules.
- Added reciprocal hyperbolic `Coth`, `Sech`, and `Csch` numeric handlers and
  derivative chain rules expressed via `Sinh`/`Cosh`.

## [0.1.0] - 2026-05-08

### Added

- Initial pure TypeScript symbolic VM.
- Strict and symbolic backends.
- Arithmetic, elementary numeric, comparison, logic, assignment, definition,
  list, and user-function application handlers.
