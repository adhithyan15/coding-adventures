# Changelog

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
