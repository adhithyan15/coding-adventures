# Changelog

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
