# Changelog

## 0.4.0 — 2026-05-22

**Phase 42 — Degree-aware vanishing-at-infinity recogniser.**

Widens Phase 41's narrow constant-numerator check to handle *any*
proper rational ``P(k)/Q(k)`` shape with ``deg(P) < deg(Q)``.  This
covers Apart outputs from any partial-fraction decomposition with
non-constant numerators — e.g. infinite telescopes built from
``k/(k²+1) − (k+1)/((k+1)²+1)`` close in one dispatch.

### Added

- **`_polynomial_degree_in_k(node, k) -> int | None`** in
  `summation.py` — returns the polynomial degree of an IR node in
  ``k`` (or ``None`` for non-polynomial shapes like ``Div``, ``Sin``,
  ``Pow(k, fractional)``).
- **Phase 42 widening branch** in `_g_vanishes_at_infinity`: when the
  numerator is *not* constant in ``k``, fall through to a
  degree-comparison check.  The function still returns ``True`` for
  Phase 41 fast-path shapes (constant numerator + positive-degree
  polynomial denominator) so Phase 41 remains a strict special case.

### Added — tests

`tests/test_summation.py` — new
`TestEvaluateSumPhase42DegreeAware` class with 5 cases:

- `test_proper_rational_k_over_k_squared_plus_1_minus_shift` —
  `∑_{k=1}^∞ [k/(k²+1) − (k+1)/((k+1)²+1)] = g(1) = 1/2`.
- `test_polynomial_degree_constant_numerator_still_works` — Phase 41
  fast-path regression: `∑_{k=1}^∞ [1/k − 1/(k+1)] = 1` still closes.
- `test_improper_rational_falls_through` — `g(k) = k/(k+1)` has equal
  degrees; limit is 1, not 0.  Phase 42 refuses; sum stays unevaluated.
- `test_super_improper_rational_falls_through` — `g(k) = k²/(k+1)` has
  deg(num) > deg(den); limit is +∞.  Sum stays unevaluated.
- `test_transcendental_numerator_falls_through` — `g(k) = sin(k)/k²`
  has a non-polynomial numerator; the degree comparison can't run, so
  Phase 42 conservatively refuses (transcendental limits deferred).

Full suite: **77 passed** (72 prior + 5 net new).

### Still deferred

- Transcendental limit-finder (`sin(k)/k²`, `log(k)/k`, `1/exp(k)`,
  …).  These require a real symbolic limit machine; out of scope for
  Phase 42's pure polynomial path.
- Cross-language port to TypeScript / Rust (blocked on porting
  `Apart` to those backends — see Phase 40 deferral).

## 0.3.0 — 2026-05-22

**Phase 41 — Limit-aware infinite telescope.**

Extends Phase 39 telescoping to handle `hi = %inf` when ``g(k)``
provably vanishes at infinity.  The classic motivating case is

    ∑_{k=1}^∞ 1/(k·(k+1))  =  1

which closes end-to-end through the symbolic-vm dispatcher as:

```
∑_{k=1}^∞ 1/(k(k+1))
  →  Apart                  (Phase 40, lives in symbolic-vm)
  →  ∑_{k=1}^∞ [1/k − 1/(k+1)]
  →  telescope detected     (Phase 39, antisymmetric)
  →  g(k) = 1/k vanishes    (Phase 41 limit check)
  →  g(1) − 0 = 1            (closed form)
```

The narrow vanishing-at-infinity recogniser handles only
``Div(constant-in-k, positive-degree-polynomial-in-k)`` shapes — every
output Apart can produce from a rational summand whose denominator
factors over ℚ into simple linear factors.  Anything else (where the
limit is undecidable without a deeper symbolic limit-finder) falls
through to the unevaluated `Sum(...)`.

### Added

- **`_g_vanishes_at_infinity(g, k)`** in `summation.py` — returns True
  for `Div(c, h(k))` shapes where `c` is constant in `k` and `h(k)` is
  a polynomial in `k` of strictly positive degree.
- **`_is_positive_degree_polynomial_in_k(node, k)`** — conservative
  walker recognising `k`, `k^n` (n ≥ 1), `Add`, and `Mul` of these.

### Changed

- **Step 4 of the `evaluate_sum` dispatcher** — the telescope detector
  now runs for both finite and infinite ranges.  Infinite ranges only
  emit a closed form when `_g_vanishes_at_infinity(g, k)` is True;
  otherwise they fall through to the unevaluated `Sum(...)`.
- **Existing `test_telescope_does_not_fire_for_infinite_upper`** is
  renamed to `test_telescope_does_not_fire_for_infinite_upper_when_g_grows`
  and its docstring updated to reflect that it now pins the Phase 41
  guard against accidental closure when `g(k)` grows rather than
  vanishes.

### Added — tests

`tests/test_summation.py` — new
`TestEvaluateSumPhase41InfiniteTelescope` class with 6 cases:

- Antisymmetric `∑_{k=1}^∞ [1/k − 1/(k+1)] = 1`.
- Standard orientation `∑_{k=1}^∞ [1/(k+1) − 1/k] = −1`.
- Higher starting index `∑_{k=2}^∞ [1/k − 1/(k+1)] = 1/2`.
- Quadratic denominator `∑_{k=1}^∞ [1/k² − 1/(k+1)²] = 1`.
- Constant-summand fallthrough (`SUB(c, c)` reduces to 0 via Step 1).
- Non-`Div` summand fallthrough (`g(k) = k` doesn't vanish; stays
  unevaluated — pins the Phase 41 guard against divergent telescopes).

Full `cas-summation` suite: **72 passed** (66 prior + 6 net new).
End-to-end via `symbolic-vm` Phase 40 + Phase 41 chain: a new
`test_phase40_plus_phase41_infinite_chain` test confirms
``∑_{k=1}^∞ 1/(k(k+1)) = 1`` as the single-dispatch closed form.

### Still deferred

- Wider `_g_vanishes_at_infinity` recogniser (e.g. ``deg(num) < deg(den)``
  rational shapes with non-constant numerator).
- Limits involving transcendental functions (`1/exp(k)`, etc.).
- Cross-language port to TypeScript / Rust (blocked on porting `Apart`
  to those backends — see Phase 40 deferral).

## 0.2.0 — 2026-05-20

**Phase 39 — Telescoping sum recognition.**

The dispatcher in `summation.py` now detects structurally telescoping
summands of the form `f = g(k+1) − g(k)` (and the antisymmetric
`g(k) − g(k+1)`) and emits the closed form

    ∑_{k=lo}^{hi} [g(k+1) − g(k)]  =  g(hi+1) − g(lo)
    ∑_{k=lo}^{hi} [g(k) − g(k+1)]  =  g(lo) − g(hi+1)

Detection is purely structural: we substitute `k → k+1` in one half of
the `SUB` shape and compare against the other half after VM
normalisation.  No partial-fraction expansion is attempted — the
classic `1/(k(k+1)) = 1/k − 1/(k+1)` example becomes telescoping only
*after* an explicit `Apart` step, which a follow-on phase will
compose.  The infinite case is left to a future limit-aware phase.

### Added

- **`_try_telescoping(f, k, vm)`** in `cas_summation/summation.py` —
  detects the structural telescope and returns `(g_expr, sign)` so the
  dispatcher can build `g(hi+1) − g(lo)` (sign +1) or `g(lo) − g(hi+1)`
  (sign −1).
- **Step 4 in the dispatch order** (between geometric/Faulhaber and
  classic infinite series) calls `_try_telescoping` for finite ranges
  and emits the closed form via the existing `cas_substitution.subst`
  helper.

### Added — tests

`tests/test_summation.py` — new `TestEvaluateSumTelescoping` class with
8 cases covering:

- Standard `(k+1)² − k²` telescope at concrete bounds.
- Antisymmetric `k² − (k+1)²` orientation.
- Linear `g(k) = k` (i.e. `f ≡ 1` telescopes to count).
- `g(k) = k + 5` (constant offset is preserved through the substitution).
- Negative result: telescope where `g(k+1) − g(k)` would be negative.
- Fallthrough: `k² − k` is **not** telescoping; falls back to
  Faulhaber/numeric.
- Constant-difference summand routes through Step 1 (constant rule),
  not the telescope.
- Symbolic upper bound `n` still produces a non-unevaluated tree.
- Infinite upper bound correctly stays unevaluated.

Full `cas-summation` suite: **66 passed** (58 prior + 8 net new).

## 0.1.1 — 2026-05-14

**Bug fix: geometric series now recognises `1/base^k` (division form) in addition to `base^k`.**

`_try_geometric` in `summation.py` previously only recognised `Pow(r, k)` as a geometric
base.  The MACSYMA input `sum(1/2^k, k, 0, inf)` compiles to `Sum(Div(1, Pow(2, k)), …)`,
which was not matched.

Extended the recogniser to also handle `Div(coeff, Pow(base, k))` by mapping it to
`coeff · (1/base)^k` and delegating to the existing infinite geometric series logic.
Result: `sum(1/2^k, k, 0, inf)` → `2`, `sum(1/3^k, k, 0, inf)` → `3/2`.

## 0.1.0 — 2026-05-04

**Initial release — Phase 25 symbolic summation.**

New package implementing closed-form evaluation of `sum(f, k, a, b)` and
`product(f, k, a, b)` for the most practically important summand families.

**Modules:**

- `poly_sum.py` — Faulhaber's polynomial formulas for Σ_{k=1}^n k^m, m=0..5,
  with general-bounds reduction `F(b,m) − F(a−1,m)`.
- `geometric_sum.py` — Geometric series (finite and infinite):
  `c·r^lo·(r^(n)−1)/(r−1)` and `c·r^lo/(1−r)`.
- `special_sums.py` — Classic convergent infinite series: Basel (π²/6, π⁴/90),
  Leibniz (π/4), Taylor for e and exp(x).
- `product_eval.py` — Finite products: factorial (`GammaFunc(n+1)`), constant
  factor, scaled factorial, numeric small products.
- `summation.py` — Main dispatcher: `evaluate_sum` + `evaluate_product`.

**Evaluation order in `evaluate_sum`:**
1. Constant summand → `f·(hi−lo+1)`
2. Geometric series → formula
3. Power of index → Faulhaber polynomial
4. Classic infinite series → table lookup
5. Numeric small range → direct computation
6. Fallback → unevaluated `SUM(f, k, lo, hi)`
