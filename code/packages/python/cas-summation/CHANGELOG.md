# Changelog

## 0.9.0 — 2026-05-22

**Phase 51 — Sqrt(polynomial)/polynomial growth-rate recogniser.**

Closes the ``sqrt(k)/k²``-style gap noted as deferred in Phase 50.
``sqrt(P(k))`` has effective polynomial degree ``deg(P)/2`` for large
``k`` (assuming positive leading coefficient so the root is
real-valued).  When the denominator is a polynomial of strictly
higher degree, the quotient vanishes.

Bumps 0.7.0 → 0.9.0 (skipping 0.8.0 reserved for the in-flight
Phase 50 PR #3938).

### Added

- **`_sqrt_effective_half_degree(node, k)`** — returns
  ``Fraction(deg(P), 2)`` for ``node = Sqrt(P(k))`` with ``P``
  positive-degree and positive leading coefficient; ``None``
  otherwise.  Conservative: refuses ``Sqrt(negative-polynomial)``
  shapes whose root isn't real.

### Changed

- ``_g_vanishes_at_infinity`` adds a Phase 51 branch between the
  Phase 49 bounded check and the Phase 42 degree-aware path.  If the
  numerator is ``Sqrt(positive-poly)`` and the denominator's
  polynomial degree exceeds the sqrt's half-degree, the quotient
  vanishes.

### Added — tests

`tests/test_summation.py::TestEvaluateSumPhase51SqrtOverPolynomial`
— 5 new cases:

- ``sqrt(k)/k²`` closes (1/2 < 2).
- ``sqrt(k³)/k²`` closes (3/2 < 2 — tight margin).
- ``sqrt(k²)/k²`` closes (1 < 2).
- ``sqrt(k)/k`` closes (1/2 < 1 — also a half-degree edge case).
- ``sqrt(Mul(-1, k))/k²`` stays unevaluated (regression).

Full suite: **103 passed** (was 98; +5 net new).

### Still deferred

- ``sqrt`` of more exotic shapes (``Sqrt(Exp(k))``, etc.).
- General transcendental limit-finder.

## 0.7.0 — 2026-05-22

**Phase 49 — Bounded × vanishing recogniser.**

Extends ``_g_vanishes_at_infinity`` to accept
``Div(bounded, diverging)`` shapes where the numerator is uniformly
bounded.  Closes telescopes like ``∑ [sin(k)/k² − sin(k+1)/(k+1)²]
 = sin(1)`` that the previous Phase 42 degree-aware path refused
(``sin(k)`` isn't a polynomial, so its degree-in-k was ``None``).

### Added

- **`_is_bounded_in_k(node, k)`** — recogniser for uniformly
  bounded shapes:
  1.  constant in ``k``                                  → True
  2.  ``Sin(...)`` or ``Cos(...)`` (any inner argument)  → True
  3.  ``Mul(bounded, bounded)``                          → True
  4.  ``Add(bounded, bounded)``                          → True
  5.  ``Neg(bounded)``                                   → True
  6.  anything else (bare ``k``, ``Log(k)``, ``Exp(k)``, …) → False

  Conservative — when in doubt, returns False so the caller falls
  through to the unevaluated ``Sum(...)`` form.

### Changed

- ``_g_vanishes_at_infinity`` now consults ``_is_bounded_in_k`` on
  the numerator (before falling through to the Phase 42 degree-
  aware path).  If the numerator is bounded AND the denominator
  diverges (via the existing ``_h_diverges_at_infinity``), the
  quotient vanishes at infinity.

### Added — tests

`tests/test_summation.py::TestEvaluateSumPhase49BoundedNumerator`
— 5 new cases:

- ``test_sin_over_k_squared_closes`` —
  ``∑ [sin(k)/k² − sin(k+1)/(k+1)²]`` closes to ``sin(1)``.
- ``test_cos_over_k_cube_closes`` — analogous with ``cos`` / ``k³``.
- ``test_sin_cos_product_over_diverging`` — product of bounded
  factors is bounded (closure under ``Mul``).
- ``test_unbounded_numerator_still_refused`` — regression for
  Phase 42 path on ``k/k³`` (deg-difference catches it, not
  Phase 49).
- ``test_log_numerator_still_refused`` — regression: ``log(k)/k²``
  stays unevaluated.  The math limit IS 0 by squeeze, but
  ``Log(k)`` isn't bounded — the recogniser refuses correctly.

### Renamed

- ``test_transcendental_numerator_falls_through`` →
  ``test_transcendental_numerator_closes_via_phase49`` (assertion
  flipped from "stays unevaluated" to "now closes").

Full suite: **98 passed** (was 92 + 1 stale assertion that
required updating; +5 net new + 1 flipped).

### Still deferred

- Transcendental growth-rate recogniser for shapes like
  ``log(k)/k`` or ``log(k)/k²`` — these vanish by squeeze too,
  but require comparing growth rates (``log`` < any polynomial),
  not just boundedness.  Future phase.

## 0.6.0 — 2026-05-22

**Phase 44 — Log divergence in vanishing-at-infinity recogniser.**

Extends Phase 43's `_h_diverges_at_infinity` to also accept
``Log(h(k))`` shapes where ``h(k) → +∞`` (so ``log(h) → +∞``,
albeit at a logarithmic rate).

### Added

- New **Log branch** in `_h_diverges_at_infinity`.  Two cases:
  1. ``h(k)`` is a positive-degree polynomial in ``k`` — require
     **positive leading coefficient** explicitly.  The Phase 41/42
     polynomial-magnitude check accepts e.g. ``Mul(-1, k)`` whose
     magnitude diverges but whose value goes to ``-∞``, which would
     make ``log(h)`` complex / undefined.  The sign-aware helper
     (`_polynomial_leading_coeff_sign_in_k`, added in Phase 43)
     gives the right answer here.
  2. ``h(k)`` is itself ``Exp(...)`` or ``Pow(b, ...)`` — defer to
     `_h_diverges_at_infinity` recursively (those branches are
     already sign-aware and their values are positive by
     construction).

  Any other shape (``Log(constant)``, ``Log(transcendental ≠ Exp/Pow)``,
  ``Log(Mul(-1, k))``, …) is conservatively refused.

### Added — tests

`tests/test_summation.py` — new `TestEvaluateSumPhase44LogDivergence`
class with 4 cases:

- ``Log(k+1)`` recognised; antisymmetric telescope closes to a
  symbolic ``1/log(2)`` form.
- ``Log(2^k)`` recognised via the Phase 43 Pow delegation.
- ``Log(5)`` (finite constant) refused — never emits a wrong
  ``−1/log(5)`` closed form.
- ``Log(Mul(-1, k))`` refused (negative leading coefficient; Phase 44
  must not pretend ``log(-k)`` is real).

Full suite: **92 passed** (88 prior + 4 net new).

### Still deferred

- ``Log(non-polynomial non-Exp/Pow)`` shapes (e.g. ``Log(Sin(k) + k²)``).
- Cross-language port to TypeScript / Rust.

## 0.5.0 — 2026-05-22

**Phase 43 — Transcendental vanishing-at-infinity (`Exp(h)` and
`Pow(b, h)` shapes).**

Extends Phase 41/42's vanishing-at-infinity recogniser to accept
exponentially diverging denominators, so infinite telescopes like

    ∑_{k=0}^∞ [1/2^k − 1/2^(k+1)] = 1
    ∑_{k=1}^∞ [1/(k·2^k) − 1/((k+1)·2^(k+1))] = 1/2

close in one dispatch.

### Added

- **`_h_diverges_at_infinity(node, k)`** in `summation.py` — the
  union of the Phase 41/42 positive-degree polynomial recogniser and
  three new transcendental cases:
  1. ``Exp(h(k))`` with ``h`` a positive-degree polynomial in ``k``
     AND positive leading coefficient (so ``h → +∞``, not ``−∞``).
  2. ``Pow(b, h(k))`` with ``b`` a rational of magnitude > 1 and
     ``h`` positive-degree with positive leading coefficient.
  3. ``Mul(...)`` where at least one factor diverges and the others
     are constant in ``k`` or also diverging.  Recursive.
- **`_polynomial_leading_coeff_sign_in_k(node, k) -> int | None`** —
  returns the sign (``+1`` or ``−1``) of the polynomial's leading
  coefficient in ``k``, or ``None`` for non-polynomial / degree-0 /
  unknown-sign shapes.  Conservatively refuses on tied-degree ``Add``
  terms (where leading coefficients could cancel) and symbolic
  constants of unknown sign.  Required for the Exp / Pow branches
  above so we don't claim ``exp(-k)`` or ``2^(-k)`` diverges (they
  actually vanish).

### Changed

- `_g_vanishes_at_infinity` Phase 41 fast path now calls
  `_h_diverges_at_infinity` instead of
  `_is_positive_degree_polynomial_in_k` directly, picking up the
  transcendental cases automatically.  Phase 42 widening (proper
  rational `deg(P) < deg(Q)`) is unchanged.

### Added — tests

`tests/test_summation.py` — new
`TestEvaluateSumPhase43Transcendental` class with 7 cases:

- ``∑_{k=0}^∞ [1/2^k − 1/2^(k+1)] = 1``.
- ``∑_{k=1}^∞ [1/3^k − 1/3^(k+1)] = 1/3``.
- Negative base magnitude > 1: ``∑ [1/(-2)^k − …] = 1``.
- Base = 1 falls through (Step 1 constant rule fires first; pins the
  Phase 43 ``|b| > 1`` guard against accidental closure at b=1).
- Rational base 3/2 diverges → closed form.
- Base 1/2 falls through (denominator ``(1/2)^k → 0``, not ∞).
- ``Mul`` of polynomial × exponential (``k · 2^k``) diverges → closed
  form ``g(1) = 1/2``.

Plus 4 sign-aware regression tests (from the in-flight security review):

- ``exp(-k)`` and its symmetric pair MUST refuse (``-k`` has negative
  leading coefficient → ``exp(-k) → 0``, not ∞; closing the sum would
  silently emit a wrong answer).
- ``2^(-k)`` MUST refuse for the same reason.
- ``k · 2^(-k)`` MUST refuse — the Mul recursion propagates the
  child-level refusal.
- ``Exp(Neg(k))`` MUST refuse — same semantics as ``Exp(Mul(-1, k))``
  but written with the explicit ``NEG`` wrapper.

Full suite: **88 passed** (77 prior + 7 Phase 43 + 4 regression).

### Still deferred

- ``Log(h(k))`` divergence (``log(k) → ∞`` but only at logarithmic
  rate; needs explicit limit handling).
- Cross-language port to TypeScript / Rust.

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
