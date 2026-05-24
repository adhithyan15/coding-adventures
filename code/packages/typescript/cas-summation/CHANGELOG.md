# Changelog

## 1.5.0 — 2026-05-23

**Phase 57 — Bounded × Log × Sqrt combination (TypeScript port).**

Ports Python ``cas-summation`` 1.5.0 (PR #4215).  Combines Log
(sub-polynomial) and Sqrt (half-polynomial) growth.  Effective
growth ``log(k)·k^{deg(P)/2}`` is strictly dominated by
``k^{deg(P)/2 + ε}`` for any ``ε > 0``.

### Added

- **`boundedLogSqrtHalfDegree(node, k)`** — returns ``deg(P)/2`` for
  ``Mul`` with exactly one ``Log(diverging)`` factor, exactly one
  ``Sqrt(positive-poly)`` factor, and any number of bounded factors.
  Returns ``undefined`` for one-only patterns (Phase 55 / Phase 56
  handle those) or two-of-either (conservative).

### Changed

- ``gVanishesAtInfinity`` adds Phase 57 branch after Phase 56.

### Tests

4 new ``summation: Phase 57 bounded × Log × Sqrt numerator`` cases.
Full suite: **73 passed** (was 69; +4).

## 1.4.0 — 2026-05-23

**Phase 56 — Bounded × Sqrt(diverging) numerator pattern (TypeScript port).**

Ports Python ``cas-summation`` 1.4.0 (PR #4167).  Bounded × sqrt
analogue of Phase 55's bounded × log.  Effective growth degree is
``deg(P)/2``; quotient vanishes when ``denDeg > deg(P)/2``
(polynomial) or denominator is non-polynomial diverging.

### Added

- **`boundedTimesSqrtHalfDegree(node, k)`** — returns ``deg(P)/2``
  (half-degree) for ``Mul`` of exactly one ``Sqrt(positive-poly)``
  factor and rest bounded.  Returns ``undefined`` for the no-Sqrt
  case, two-Sqrt case (conservative), or unrecognised factors.

### Changed

- ``gVanishesAtInfinity`` adds Phase 56 branch after Phase 55, with
  two denominator sub-cases (polynomial ``denDeg > halfDeg`` OR
  non-polynomial diverging dominates).

### Tests

3 new ``summation: Phase 56 bounded × sqrt numerator`` cases.
Full suite: **69 passed** (was 66; +3).

## 1.3.0 — 2026-05-23

**Phase 55 — Bounded×Log(diverging) numerator pattern (TypeScript port).**

Ports Python `cas-summation` 1.3.0 Phase 55 to TypeScript.  Adds
`isBoundedTimesLogInK` helper and a Phase 55 branch in `gVanishesAtInfinity`.
`bounded(k) × log(h(k))` grows sub-polynomially — dominated by any
polynomial or faster-growing denominator.

Bumps 1.2.0 → 1.3.0.

### Added

- **`isBoundedTimesLogInK(node, k)`** — Phase 55 helper. Returns true when
  `node` is a `Mul` with exactly one `Log(diverging)` factor and all remaining
  factors pass `isBoundedInK`. Requires exactly one log factor; two+ → false.

- **Phase 55 branch in `gVanishesAtInfinity`** — after Phase 54, before Phase 42.
  Closes `Div(Mul(bounded, Log(diverging)), den)` when `den` diverges.

- **5 new tests** in `describe("Phase 55 Bounded×Log(diverging) numerator")`:
  - `sin(k)·log(k) / k² closes`
  - `cos(k)·log(k) / k closes`
  - `sin(k)·cos(k)·log(k) / k³ closes`
  - `sin(k)·log(k²) / k³ closes`
  - `sin(k)·log(k) / 1 stays unevaluated` (constant denominator refused)

Total: 66 tests (was 61).

## 1.2.0 — 2026-05-23

**Phase 54 — Log×polynomial numerator pattern (TypeScript port).**

Ports Python `cas-summation` 1.2.0 Phase 54 to TypeScript.  Adds
`splitLogPolynomialFactor` helper and a Phase 54 branch in
`gVanishesAtInfinity`.  `log(h(k))` grows sub-polynomially so the
effective growth degree of `log(h) · P(k)` equals `deg(P)`.  Vanishes
when `den_deg > poly_deg` (strictly).

Bumps 1.1.0 → 1.2.0.

### Added

- **`splitLogPolynomialFactor(node, k)`** — Phase 54 helper.  Splits a
  `Mul` node into exactly one `Log(diverging)` factor and a polynomial
  part; returns `{ logFactor, polyDeg }` or `undefined`.

- **Phase 54 branch in `gVanishesAtInfinity`** — inserted after Phase 53
  and before the Phase 42 polynomial widening.  Closes
  `Div(Mul(Log(diverging), P), Q)` when `den_deg > poly_deg`.

- **5 new tests** (`describe "summation: Phase 54 Log×polynomial numerator"`):
  - `log(k)·k / k³ closes (poly_deg=1, den_deg=3)`
  - `log(k)·k² / k³ closes (poly_deg=2, den_deg=3)`
  - `log(k)·k / k² closes (poly_deg=1, den_deg=2)`
  - `log(k)·k² / k² stays unevaluated (equal degrees — diverges)`
  - `regression: plain log(k)/k³ still closes via Phase 50`

### Tests

61 passed (was 56; +5 net new — Phase 54).

---

## 1.1.0 — 2026-05-23

**Phase 53 — Sqrt × polynomial numerator pattern (TypeScript port).**

Extends ``gVanishesAtInfinity`` to recognise that
``Mul(Sqrt(P), polynomial_factors)`` numerators have effective growth
equal to ``deg(P)/2 + deg(Q)``.  Closes telescopes like
``sqrt(k)·k/k³`` and ``sqrt(k²)·k/k³`` that fall through all
earlier phases.

Builds on Phase 51 (0.9.0) which added the plain-``Sqrt`` case.
Bumps 1.0.0 → 1.1.0.

### Added

- **``sqrtPolyNumeratorEffectiveDegree(node, k)``** — returns
  ``deg(P)/2 + deg(Q)`` (a number) when ``node = Mul(Sqrt(P), Q_poly)``
  with exactly one Sqrt factor and all other factors polynomial.
  Returns ``undefined`` for plain ``Sqrt`` nodes (handled by Phase 51),
  non-Mul nodes, multiple Sqrt factors, non-polynomial non-Sqrt factors.

### Changed

- ``gVanishesAtInfinity`` adds a Phase 53 branch between Phase 52
  (bounded × polynomial) and Phase 42 (pure rational degree comparison):
  closes when ``den_deg > sqrtPolyNumeratorEffectiveDegree(num, k)``.

### Added — tests

5 new ``phase53_*`` cases:
- ``phase53_sqrt_k_times_k_over_k_cubed_closes`` — eff 3/2 < 3.
- ``phase53_sqrt_k_squared_times_k_over_k_cubed_closes`` — eff 2 < 3.
- ``phase53_sqrt_k_times_k_squared_over_k_cubed_closes`` — eff 5/2 < 3.
- ``phase53_sqrt_k_times_k_squared_over_k_squared_stays`` — eff 5/2 > 2.
- ``phase53_regression_sqrt_k_over_k_squared_still_closes_via_phase51`` — plain
  Sqrt bypasses Phase 53 and closes via Phase 51.

Full suite: **56 passed** (was 51; +5 net new).

## 1.0.0 — 2026-05-23

**Phase 52 — Bounded × polynomial numerator pattern (TypeScript port).**

Ports Python ``cas-summation`` 1.0.0.  Extends ``gVanishesAtInfinity``
to recognise that ``Mul(bounded, polynomial)`` numerators have effective
growth equal to the polynomial part's degree.  Closes telescopes like
``sin(k)·k/k³``, ``k·cos(k)/k²``, where the numerator mixes a bounded
factor with a non-trivial polynomial factor.

Bumps 0.9.0 → 1.0.0.

### Added

- **`splitBoundedPolynomialFactor(node, k)`** — partitions a ``Mul``
  node's factors into a bounded aggregate and a summed polynomial degree;
  returns ``undefined`` if any factor is neither bounded nor polynomial,
  or if no non-constant-in-k bounded factor exists (those go through
  Phase 42).

### Changed

- ``gVanishesAtInfinity`` now has a Phase 52 branch between Phase 51
  (sqrt numerator) and Phase 42 (degree-aware): when the numerator
  factors as ``bounded × polynomial`` with positive polynomial degree,
  the quotient vanishes iff the denominator's polynomial degree strictly
  exceeds the polynomial part's degree.

### Added — tests

`tests/cas-summation.test.ts`, `describe("summation: Phase 52 bounded × polynomial numerator")`:
- ``sin(k)·k/k³`` closes (bounded × deg 1 / deg 3).
- ``k·cos(k)/k²`` closes (factor order doesn't matter).
- ``sin(k)·k²/k³`` closes (deg 2 < 3).
- Regression: ``sin(k)·k²/k²`` stays unevaluated (degrees tie).
- Regression: ``k/k²`` still closes via Phase 42 (Phase 52 doesn't
  interfere when no bounded factor is present).

Full suite: **51 passed** (was 46; +5 net new).

## 0.9.0 — 2026-05-22

**Phase 51 — Sqrt(polynomial)/polynomial recogniser (TypeScript port).**

Ports Python ``cas-summation`` 0.9.0.  Extends ``gVanishesAtInfinity``
to recognise that ``sqrt(P(k))`` has effective polynomial degree
``deg(P)/2`` for large ``k``.  When the denominator's polynomial
degree exceeds this half-degree, the quotient vanishes.

Bumps 0.8.0 → 0.9.0.

### Added

- **`sqrtEffectiveHalfDegree(node, k)`** — returns ``deg(P)/2`` for
  ``Sqrt(P(k))`` with positive-leading-coefficient ``P``; undefined
  otherwise.

### Tests

3 new ``summation: Phase 51 sqrt/polynomial growth-rate`` cases.
Full suite: **46 passed** (was 43; +3 net new).

## 0.8.0 — 2026-05-22

**Phase 50 — Log/polynomial growth-rate recogniser (TypeScript port).**

Ports Python ``cas-summation`` 0.8.0.  Extends ``gVanishesAtInfinity``
to accept ``Div(Log(diverging), diverging)`` shapes via the squeeze
argument: ``log(h) → ∞`` at a logarithmic rate, denominator grows
strictly faster, so the quotient vanishes.

Builds on Phase 49 (0.7.0) which added ``isBoundedInK`` for bounded
× vanishing shapes.

### Added

- **`isLogOfDivergingInK(node, k)`** — recognises ``Log(h(k))``
  with ``h(k) → +∞``.  Sign-aware: delegates to
  ``hDivergesAtInfinity`` on the full ``Log(...)`` node so
  Phase 44's Log branch refuses ``Log(Mul(-1, k))``-style negative
  shapes for free.

### Changed

- ``gVanishesAtInfinity`` now has a Phase 50 branch after the Phase 49
  bounded check and before the Phase 42 degree-aware path.
- The Phase 49 ``regression: log(k)/k² stays unevaluated`` test is
  superseded and removed — ``log(k)/k²`` now closes via Phase 50.

### Added — tests

3 new ``summation: Phase 50 log/polynomial growth-rate`` cases:
- ``log(k)/k²`` closes.
- ``log(k²+1)/k³`` closes.
- Regression: ``log(Mul(-1, k))/k²`` stays unevaluated.

Full suite: **43 passed** (was 41; +2 net new — Phase 49 log regression
superseded by Phase 50 log-closes case).

## 0.7.0 — 2026-05-22

**Phase 49 — Bounded × vanishing recogniser (TypeScript port).**

Ports Python ``cas-summation`` 0.7.0.  Extends ``gVanishesAtInfinity``
to accept ``Div(bounded, diverging)`` shapes where the numerator is
uniformly bounded — covers telescopes like
``∑ [sin(k)/k² − sin(k+1)/(k+1)²] = sin(1)`` that the Phase 42
degree-aware path refused (``sin`` isn't a polynomial).

### Added

- **`isBoundedInK(node, k)`** — recogniser for uniformly bounded
  shapes: constants in ``k``, ``Sin(...)``, ``Cos(...)``, closures
  under ``Mul``/``Add``/``Neg``.

### Changed

- ``gVanishesAtInfinity`` now consults ``isBoundedInK`` on the
  numerator between the Phase 41 fast-path and the Phase 42
  degree-aware path.  If the numerator is bounded AND the
  denominator diverges, the quotient vanishes.

### Added — tests

`tests/cas-summation.test.ts` — new
``summation: Phase 49 bounded × vanishing`` block with 4 cases:

- ``∑ [sin(k)/k² − sin(k+1)/(k+1)²]`` closes.
- ``∑ [cos(k)/k³ − cos(k+1)/(k+1)³]`` closes.
- ``sin(k)·cos(k)/k²`` closes (Mul closure of bounded factors).
- Regression: ``log(k)/k²`` stays unevaluated (``Log`` isn't
  bounded).

Plus renamed
``transcendental numerator … falls through`` →
``transcendental numerator … closes via Phase 49`` (assertion
flipped).

Full suite: **41 passed** (was 37; +4 net new).

## 0.6.0 — 2026-05-22

**Phase 40+46 — Add-with-negation telescope normaliser (TypeScript port).**

Ports the Python helpers ``_extract_negation`` and
``_normalise_add_neg_to_sub`` (introduced in symbolic-vm 0.50/0.70).
Widens ``tryTelescoping`` to accept summands written in
``Add(g(k+1), Neg(g(k)))`` or ``Add(g(k+1), Div(-c, d))`` form by
rewriting them to the canonical ``Sub`` shape before the structural
match runs.

### Why this is useful in TS even without ``Apart``

The Python ``Apart`` step (Phase 40 + Phase 46 in ``symbolic-vm``)
emits ``Add(Div(-c, k+1), Div(c, k))``, which is exactly the shape
the new normaliser targets.  On the TS side ``cas-summation`` doesn't
own an ``Apart`` implementation, but users (or upstream pipelines)
who emit the same shape directly now get the telescope closure for
free — no churn at the call site required.

### Added

- **`extractNegation(node): IRNode | undefined`** — uniformly
  detects a negation in two recognised forms:
  1.  ``Neg(x)`` (top-level wrapper)               → ``x``
  2.  ``Div(c, d)`` with literal ``c < 0`` (numerator-folded sign)
      → ``Div(|c|, d)``.  Handles integer and rational numerators.
- **`normaliseAddNegToSub(node): IRNode`** — rewrites two-term
  ``Add`` containing a recognised negation into the equivalent ``Sub``
  shape (returns input unchanged when no rewrite applies, including
  the both-sides-negative case).

### Changed

- ``tryTelescoping`` now calls ``normaliseAddNegToSub`` on ``Add``
  inputs before the ``SUB`` head check.  Pure ``Sub`` and non-``Add``
  shapes are untouched (zero cost).

### Added — tests

`tests/cas-summation.test.ts` — new
``summation: Phase 40+46 Add-with-negation normaliser`` describe
block with 6 cases:

- ``Add(g(k+1), Neg(g(k)))`` closes to −1 (standard orientation).
- ``Add(Neg(g(k)), g(k+1))`` closes to −1 (operand-order swap).
- ``Add(g(k), Div(-1, k+1))`` closes to 1 (numerator-folded Neg,
  antisymmetric).
- ``Add(Div(-5, k+1), Div(5, k))`` closes to 5 (non-unit constant —
  the Python Phase 46 constant-numerator case).
- ``Add(Div(1/2, k), Div(-1/2, k+1))`` closes to 1/2 (rational
  numerator).
- ``Add(Neg(a), Neg(b))`` (both sides negative) intentionally
  stays unevaluated — no telescope to expose.

Full suite: **37 passed** (was 31; +6 net new).

### Still deferred

- ``Apart`` partial-fraction-decomposition handler.  Until ported,
  callers must pre-decompose any rational summand they want to feed
  through the telescope detector.
- Transcendental limit-finder (``sin(k)/k²``, …).

## 0.5.0 — 2026-05-22

**Phase 44 — Log divergence recogniser (TypeScript port).**

Ports Python `cas-summation` 0.6.0 (PR #3909).  Extends Phase 43's
`hDivergesAtInfinity` to also accept `Log(h(k))` where `h(k) → +∞`.

### Added

- New **Log branch** in `hDivergesAtInfinity` with three sub-cases:
  1. Polynomial inner: positive leading coefficient required.
  2. `Exp(h')` inner: always positive; defer.
  3. `Pow(b, h')` inner: require base `b > 1` *strictly positive*
     (not just `|b| > 1`; `Pow(-2, k)` value oscillates so
     `log((-2)^k)` not real-valued).

### Added — tests

4 new cases:
- `Log(k+1)` recognised.
- `Log(2^k)` recognised via Phase 43 Pow delegation.
- Regression: `Log(Pow(-2, k))` refused.
- Regression: `Log(Mul(-1, k))` refused.

Full suite: **31 passed** (27 prior + 4 net new).

## 0.4.0 — 2026-05-22

**Phase 43 — Transcendental vanishing-at-infinity (TypeScript port).**

Ports Python `cas-summation` 0.5.0 (PR #3899 in review).  Extends the
Phase 41/42 denominator recogniser to accept exponentially diverging
shapes so `∑_{k=0}^∞ [1/2^k − 1/2^(k+1)] = 1` and similar close.

### Added

- **`hDivergesAtInfinity(node, k)`** — union of Phase 41/42
  positive-degree polynomial check and three transcendental cases:
  1. `Exp(h(k))` with h positive-degree AND positive leading coeff.
  2. `Pow(b, h(k))` with rational `|b| > 1` AND h positive-degree
     with positive leading coefficient.
  3. `Mul(...)` where at least one factor diverges and the others
     are constant-in-k or also diverging.  Recursive.
- **`polynomialLeadingCoeffSignInK(node, k) -> 1 | -1 | undefined`**
  — returns the sign of the polynomial's leading coefficient in `k`,
  or `undefined` for non-polynomial / degree-0 / unknown-sign shapes.
  Required to refuse `exp(-k)`, `2^(-k)`, etc. (these vanish, not
  diverge).

### Changed

- `gVanishesAtInfinity` Phase 41 fast path now calls
  `hDivergesAtInfinity` instead of `isPositiveDegreePolynomialInK`
  directly, picking up the transcendental cases automatically.

### Added — tests

`tests/cas-summation.test.ts` — new
`summation: Phase 43 transcendental vanishing-at-infinity` describe
block with 6 cases:

- `∑_{k=0}^∞ [1/2^k − 1/2^(k+1)] = 1`.
- `∑_{k=1}^∞ [1/3^k − 1/3^(k+1)] = 1/3`.
- Base 1/2 falls through (`(1/2)^k → 0`, not ∞).
- `Mul` of polynomial × exponential `k · 2^k` closes (= 1/2).
- Regression: `2^(-k)` via `Mul(-1, k)` does NOT diverge → refuse.
- Regression: `2^(Neg(k))` does NOT diverge → refuse (NEG wrapper).

Full suite: **27 passed** (21 prior + 6 net new).

### Still deferred

- Apart-induced telescopes (e.g. `1/(k(k+1))`) — blocked on porting
  the `Apart` partial-fraction-decomposition handler to TypeScript.
- Transcendental limit-finder for shapes the polynomial-degree path
  doesn't cover (e.g. `sin(k)/k²`, `log(k)/k`).

## 0.3.0 — 2026-05-22

**Phase 41 + Phase 42 — Limit-aware infinite telescope (TypeScript port).**

Ports Python `cas-summation` 0.3.0 (PR #3880 ✅) and 0.4.0
(PR #3887 ✅) in one go.  Extends `evaluateSum`'s telescope detection
to handle `hi = %inf` when `g(k)` provably vanishes at infinity:

    ∑_{k=lo}^∞ [g(k+1) − g(k)]  =  −g(lo)   (standard orientation)
    ∑_{k=lo}^∞ [g(k) − g(k+1)]  =   g(lo)   (antisymmetric)

The vanishing-at-infinity check uses two tiers:

1.  **Phase 41 fast path** — `Div(constant-in-k, h(k))` with `h` a
    positive-degree polynomial in `k`.
2.  **Phase 42 widening** — `Div(P(k), Q(k))` where both are pure
    polynomials and `deg(P) < deg(Q)`.

Anything transcendental, improper, or non-Div falls through to the
unevaluated `Sum(...)`.

### Added

- **`isPositiveDegreePolynomialInK(node, k)`** — recogniser for ``k``,
  ``k^n`` (n ≥ 1), ``Add``, and ``Mul`` of these.
- **`polynomialDegreeInK(node, k) -> number | undefined`** — returns
  the polynomial degree of an IR node in ``k``, or undefined for
  non-polynomial shapes (Div, Sin, fractional Pow, …).
- **`gVanishesAtInfinity(g, k)`** — two-tier predicate combining the
  above.

### Changed

- The `infUpper` gate around the Phase 39 telescope branch is lifted;
  the dispatcher now runs telescope detection for both finite and
  infinite ranges and routes through the new vanishing-at-infinity
  check when `hi = %inf`.
- Existing "infinite upper bound falls through" test renamed to
  pin the Phase 41 guard against divergent telescopes
  (`g(k) = k` doesn't vanish).

### Added — tests

`tests/cas-summation.test.ts` — new
`summation: Phase 41+42 limit-aware infinite telescope` describe
block with 7 cases:

- `∑_{k=1}^∞ [1/k − 1/(k+1)] = 1` (Phase 41 antisymmetric).
- `∑_{k=1}^∞ [1/(k+1) − 1/k] = −1` (standard orientation).
- Higher starting index `∑_{k=2}^∞ … = 1/2`.
- Quadratic denominator `∑ 1/k² − 1/(k+1)² = 1`.
- Phase 42 proper rational
  `∑ k/(k²+1) − (k+1)/((k+1)²+1) = 1/2`.
- Improper rational `k/(k+1)` falls through (limit is 1).
- Transcendental `sin(k)/k²` falls through (non-polynomial).

Full suite: **21 passed** (14 prior + 7 net new).

### Still deferred

- Apart-induced telescopes (`1/(k(k+1))`) — blocked on porting the
  `Apart` partial-fraction-decomposition handler to TypeScript.
- Transcendental limit-finder (`sin(k)/k²`, `log(k)/k`, `1/exp(k)`).

## 0.2.0 — 2026-05-20

**Phase 39 — Telescoping sum recognition (TypeScript port).**

Mirrors Python `cas-summation` 0.2.0 (PR #3706 ✅ merged).

The `evaluateSum` dispatcher now detects structurally telescoping
summands of the form `f = g(k+1) − g(k)` (and the antisymmetric
`g(k) − g(k+1)`) and emits the closed form:

    ∑_{k=lo}^{hi} [g(k+1) − g(k)]  =  g(hi+1) − g(lo)
    ∑_{k=lo}^{hi} [g(k) − g(k+1)]  =  g(lo) − g(hi+1)

Detection is purely structural: substitute `k → k+1` in one half of
the `SUB` shape and compare against the other half after `evalFn`
normalisation.  No partial-fraction expansion is attempted — the
classic `1/(k(k+1))` example becomes telescoping only after an
explicit `Apart` step, left for a follow-on phase.  Infinite ranges
fall through (a future limit-aware phase will handle those).

### Added

- **`tryTelescoping(f, k, evalFn)`** in `src/index.ts` — returns
  `{ gExpr, sign }` when the SUB structure matches, where `sign = 1`
  for the standard `g(k+1) − g(k)` orientation and `-1` for the
  antisymmetric `g(k) − g(k+1)`.
- New dispatch step inserted between Faulhaber and classic-infinite,
  guarded on `!infUpper`.

### Added — tests

`tests/cas-summation.test.ts` — new `summation: Phase 39 telescoping`
describe block with 8 cases covering:

- Standard `(k+1)² − k²` telescope at concrete bounds → 24.
- Antisymmetric `k² − (k+1)²` orientation → −15.
- Linear `g(k) = k` (`f ≡ 1` counts terms).
- `g(k) = k + 5` (constant offset preserved through substitution).
- Non-telescoping `k² − k` falls through to numeric/Faulhaber.
- Constant-difference summand routes through step 1 (constant rule).
- Symbolic upper bound `n` produces a non-unevaluated tree.
- Infinite upper bound correctly stays unevaluated.

All 14 tests pass (6 prior + 8 net new).

## 0.1.0

- Add pure TypeScript summation and product evaluator.
- Cover geometric, Faulhaber, classic infinite-series, and product patterns.
