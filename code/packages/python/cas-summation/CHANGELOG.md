# Changelog

## 2.28.0 - 2026-05-29

### Added

- **Track I1 of `macsyma-truly-finish-plan.md`** — closed-form
  recogniser for canonical infinite series in the new module
  `cas_summation/series_closed_forms.py`.  Pattern-matches the
  summand against the classical zeta / eta / factorial Taylor
  shapes and emits the closed form when `hi = %inf`:
  - `sum(1/k^(2m), k, 1, %inf)` → `ζ(2m) · π^(2m)` for `m = 1..6`
    (Basel through `ζ(12) = 691·π¹²/638512875`).
  - `sum((-1)^(k-1)/k, k, 1, %inf)` → `log(2)` (Mercator).
  - `sum((-1)^(k-1)/k^(2m), k, 1, %inf)` → `η(2m) · π^(2m)` for
    `m = 1..3` (Dirichlet eta).
  - `sum(1/k!, k, 0, %inf)` → `%e`.
  - `sum(x^k/k!, k, 0, %inf)` → `exp(x)` (symbolic `x ≠ k`).
  - `sum((-1)^k · x^(2k)/(2k)!, k, 0, %inf)` → `cos(x)`.
  - `sum((-1)^k · x^(2k+1)/(2k+1)!, k, 0, %inf)` → `sin(x)`.
  - `sum(x^(2k)/(2k)!, k, 0, %inf)` → `cosh(x)`.
  - `sum(x^(2k+1)/(2k+1)!, k, 0, %inf)` → `sinh(x)`.
- New handler wired into the dispatcher between the existing
  `try_special_infinite` (Basel + Leibniz) and the Gosper /
  numeric-small-range paths; pre-existing tests stay on their
  original code paths.
- **One generic Bernoulli helper** computes `B_n` via the textbook
  recurrence `B_0 = 1; Σ_{j=0}^{n} C(n+1, j) · B_j = 0`.  Six
  even-zeta exponents (m = 1..6) and three even-eta exponents
  (m = 1..3) share the same code — no per-degree tables.  The
  recurrence depth is bounded by `n ≤ 12`, so the helper is
  provably terminating.
- All numeric work is exact (`fractions.Fraction`); the closed
  forms emerge as `π^(2m) / denom` IR shapes that match the
  parser-emitted forms verified by the test suite.

### Notes

- Falls through (returns `None`) for: odd zeta `ζ(2m+1)`, indices
  past `m > 6`, wrong lower bound (zeta requires `lo=1`, Taylor
  requires `lo=0`), finite upper bound, and any non-table summand
  (`sin(k)`, `log(k)`, etc.).

## 2.27.0 - 2026-05-29

### Added

- **Track H1 of `macsyma-truly-finish-plan.md`** — Gosper's algorithm
  for indefinite hypergeometric summation in the new module
  `cas_summation/gosper.py`.  Closes the spec's polynomial × `c^k` and
  polynomial × factorial families in symbolic-parameter form:
  - `sum(k*2^k, k, 1, N)` → `(N-1)*2^(N+1) + 2`
  - `sum(k*k!, k, 0, N)`  → `(N+1)! - 1`
- The new handler is wired into the dispatcher as a fallback *after*
  the existing constant / geometric / Faulhaber / telescope / classic-
  infinite paths and *before* the numeric small-range path, so all
  earlier tests remain on their original code paths.
- New helpers (all pure rational arithmetic via `fractions.Fraction`):
  - Univariate polynomial GCD via Euclid's algorithm.
  - Petkovšek shift-coprime normalisation of the ratio `a(k+1)/a(k)`.
  - Gosper degree bound for the polynomial `x(k)` in the key
    equation `A(k)·x(k+1) − B(k−1)·x(k) = C(k)`.
  - Gaussian elimination over `Fraction` for the linear coefficient
    system.
- The transcendental factors (`c^k`, `GammaFunc(k+s)`) are reconstructed
  symbolically and the polynomial part of the answer is GCD-cancelled
  against `C(k)` so removable singularities at the boundary (e.g. the
  `k!` cancellation at `k = 0`) don't surface as `0/0`.

### Notes

- Sums that are *not* hypergeometric (e.g. `sin(k)`, `log(k)`) cleanly
  fall through to the unevaluated `Sum` IR — no false positives.
- Coverage of `gosper.py` is 81%; the uncovered lines are
  defensive branches (e.g. negative-exponent guards in the IR-to-Poly
  bridge, the inconsistent-system path in the Gaussian solver).

## 2.26.0 - 2026-06-06

### Added

- Infinite telescope limits now recognise direct decaying exponential terms:
  ``exp(-k)`` and ``b^(-k)`` with ``|b| > 1`` are treated as vanishing at
  infinity. This closes structurally telescoping sums such as
  ``sum(exp(-(k+1)) - exp(-k), k, 1, inf)`` without requiring a rational
  denominator wrapper.
- The recogniser is conservative: growing exponentials, ambiguous exponent
  signs, and unrecognised transcendental factors still fall through to the
  unevaluated ``Sum`` form.

## 2.24.0 — 2026-05-28

**Track A2 cleanup — delete 27 grid helpers superseded by Phase 86 generic.**

Pure deletion: removes the 27 hand-written ``N-Sqrt × M-Log × polynomial``
helpers (Phases 59–85) and their dispatcher calls + tests, now that the
Phase 86 generic recogniser (``_log_sqrt_poly_effective_x2_generic``)
preempts the entire grid in every case the grid covers.  No behavior change.

- ``src/cas_summation/summation.py``: removed all Phase 59–85 dispatcher
  branches inside ``g_vanishes_at_infinity`` and the helper functions
  ``_bounded_sqrt_poly_effective_x2`` through ``_two_sqrt_six_log_poly_effective_x2``.
  The Phase 86 generic branch (which previously preempted them all) is
  unchanged.
- ``tests/test_summation.py``: removed all ``TestEvaluateSumPhase{59..67}``
  and ``TestPhase{68..85}`` classes (117 tests).  Phase 56–58 and Phase 86
  test classes remain.
- Net source diff: ~2,227 lines deleted from ``summation.py``;
  ~2,618 lines deleted from the test module.  pytest count drops from
  ~268 to 151; no test fails or regresses.

## 2.23.0 — 2026-05-28

**Phase 86 — Cleanup: generic log × sqrt × polynomial recogniser.**

Closes a long-running design problem.  Phases 59-85 accumulated a
hand-written grid of ``N-Sqrt × M-Log × polynomial`` helpers — one
function per ``(N, M)`` combination, all with the same body modulo
the hardcoded counts.  This PR replaces the entire grid with a
single generic helper.

### Motivation

The convergence math is identical for every non-negative ``(N, M)``:

- The product of ``N`` ``Log(diverging)`` factors is still
  sub-polynomial — ``log^N(k) = o(k^ε)`` for any ``ε > 0`` — so ``N``
  contributes ``0`` to the effective growth degree.
- Each ``Sqrt(P_i)`` contributes ``deg(P_i)/2`` (recorded ×2 to stay
  in integer arithmetic).
- Each polynomial factor contributes its own degree (also ×2 here).
- Bounded factors (constants in ``k``, ``Sin``, ``Cos``, closures)
  contribute ``0``.

Effective growth ×2: ``Σ sqrt_inner_deg_x2 + 2 · Σ poly_deg``.
Vanishes when ``2·den_deg > effective_x2`` (polynomial denominator)
or non-polynomial diverging denominator.

The grid hardcoded ``(N, M)`` for ``N ∈ {0..5}`` and ``M ∈ {0..6}``
— **42 helpers, all redundant**.  Cases beyond that grid
(``M ≥ 7``, ``N ≥ 6``) silently failed.

### Added

- **``_log_sqrt_poly_effective_x2_generic(node, k) -> int | None``** —
  one function that handles every ``(N, M, K)`` combination.  Returns
  ``effective_x2`` (= ``Σ sqrt_deg_x2 + 2·Σ poly_deg``) when the ``Mul``
  splits cleanly into Log / Sqrt / polynomial / bounded factors;
  ``None`` for unrecognised factors (e.g. ``Exp(k)``,
  ``Sqrt(negative)``).

- **Phase 86 branch in ``_g_vanishes_at_infinity``** — inserted between
  Phase 58 (bounded × Log × polynomial) and Phase 59 (bounded × Sqrt
  × polynomial).  Catches every case the hand-written grid catches,
  *plus* cases beyond the grid that previously failed.

- **6 new tests** in ``TestPhase86GenericLogSqrtPoly`` proving the
  generic handles cases the grid doesn't:

  * 7-Log / k² closes (grid stops at 6 Log).
  * 6-Sqrt / k⁴ closes (grid stops at 5 Sqrt).
  * Mixed 3-Sqrt × 7-Log × poly closes.
  * Regression: ``Exp(k)`` factor refused (would falsely close).
  * Regression: ``Sqrt(-k)`` refused (complex-valued for large k).
  * Regression: pure-bounded numerator falls through to Phase 49.

### Status of the existing grid (Phases 59-85)

The 42 hardcoded helpers remain in place for backward compatibility
and are still wired into the dispatcher.  They are now redundant —
the new Phase 86 branch catches every input they handle.  A
follow-up PR can delete them in a single sweep without behavioral
change.

### Tests

Full suite: **268 passed** (was 262; +6 net new — all from the
generic).

### Why this was needed

74 open PRs (#4471-#4544) had queued up adding more ``(N, M)`` grid
points up to ``N=64`` Log factors and beyond, each bumping the
package version by ``0.006``.  Those PRs are now closed — the
generic supersedes all of them.

### Still deferred (genuine future work)

- Deleting the redundant grid helpers (Phases 59-85) — pure
  refactor, no behavior change.
- Generalising further to ``Exp(non-positive)`` and other vanishing
  transcendentals.
- Two-Log-of-Sqrt patterns and similar nested compositions.

## 2.22.0 — 2026-05-26

### Added

- **Phase 85 — Two-Sqrt × Six-Log × polynomial numerator** (`_two_sqrt_six_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), Log(h6(k)), polynomial..., bounded...)`.
  Exactly 2 Sqrt factors and exactly 6 Log factors required.  `log⁶(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new unit tests in `TestPhase85TwoSqrtSixLogPoly`.

## 2.21.0 — 2026-05-25

### Added

- **Phase 81 — Four-Sqrt × Five-Log × polynomial numerator** (`_four_sqrt_five_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 4 Sqrt factors and exactly 5 Log factors required.  `log⁵(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + sqrt4_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new unit tests in `TestPhase81FourSqrtFiveLogPoly`.

## 2.20.0 — 2026-05-25

### Added

- **Phase 80 — Three-Sqrt × Five-Log × polynomial numerator** (`_three_sqrt_five_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1(k)), ..., Log(h5(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt factors and exactly 5 Log factors required.
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new unit tests in `TestPhase80ThreeSqrtFiveLogPoly`.

## 2.19.0 — 2026-05-25

### Added

- **Phase 84 — One-Sqrt × Six-Log × polynomial numerator** (`_one_sqrt_six_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), Log(h6(k)), polynomial..., bounded...)`.
  Exactly 1 Sqrt factor and exactly 6 Log factors required.  `log⁶(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new unit tests in `TestPhase84OneSqrtSixLogPoly`.

## 2.18.0 — 2026-05-25

### Added

- **Phase 82 — Five-Sqrt × Five-Log × polynomial numerator** (`_five_sqrt_five_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Sqrt(P5), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 5 Sqrt factors and exactly 5 Log factors required.  `log⁵(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + sqrt4_deg_x2 + sqrt5_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new unit tests in `TestPhase82FiveSqrtFiveLogPoly`.
- **Phase 83 — Six-Log × polynomial numerator** (`_six_log_poly_effective_x2`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), Log(h6(k)), polynomial..., bounded...)`.
  Exactly 6 Log factors and zero Sqrt factors required.  `log⁶(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree; `effective_x2 = 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new unit tests in `TestPhase83SixLogPoly`.

## 2.17.0 — 2026-05-25

### Added

- **Phase 79 — Two-Sqrt × Five-Log × polynomial numerator** (`_two_sqrt_five_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 2 Sqrt factors and exactly 5 Log factors required.  `log⁵(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new unit tests in `TestPhase79TwoSqrtFiveLogPoly`.

## 2.16.0 — 2026-05-25

### Added

- **Phase 78 — One-Sqrt × Five-Log × polynomial numerator** (`_one_sqrt_five_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 1 Sqrt factor and exactly 5 Log factors required.  `log⁵(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree; `effective_x2 = sqrt_inner_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new unit tests in `TestPhase78OneSqrtFiveLogPoly`.

## 2.15.0 — 2026-05-25

### Added

- **Phase 77 — Five-Log × polynomial numerator** (`_five_log_poly_effective_x2`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 5 Log factors required; Sqrt factors explicitly refused so this phase does not shadow
  the Sqrt-bearing phases (73–76, 78+).  `log⁵(k)` is sub-polynomial (`o(k^ε)`),
  contributing 0 to effective degree;
  `effective_x2 = 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new unit tests in `TestPhase77FiveLogPoly`.

## 2.14.0 — 2026-05-25

### Added

- **Phase 76 — Three-Sqrt × Four-Log × polynomial numerator** (`_three_sqrt_four_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt factors and exactly 4 Log factors required.  `log⁴(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new unit tests in `TestPhase76ThreeSqrtFourLogPoly`.

## 2.13.0 — 2026-05-25

### Added

- **Phase 75 — Two-Sqrt × Four-Log × polynomial numerator** (`_two_sqrt_four_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), polynomial..., bounded...)`.
  Exactly 2 Sqrt factors and exactly 4 Log factors required.  `log⁴(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 4 new unit tests in `TestPhase75TwoSqrtFourLogPoly`.

## 2.12.0 — 2026-05-25

### Added

- **Phase 74 — One-Sqrt × Four-Log × polynomial numerator** (`_one_sqrt_four_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), polynomial..., bounded...)`.
  Exactly 1 Sqrt factor and exactly 4 Log factors required.  `log⁴(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree; `effective_x2 = sqrt_inner_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 4 new unit tests in `TestPhase74OneSqrtFourLogPoly`.

## 2.11.0 — 2026-05-25

### Added

- **Phase 73 — Four-Log × polynomial numerator** (`_four_log_poly_effective_x2`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), polynomial..., bounded...)`.
  Exactly 4 Log factors required; Sqrt factors are refused.  `log⁴(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree; `effective_x2 = 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 5 new unit tests in `TestPhase73FourLogPoly`.

## 2.10.0 — 2026-05-25

### Added

- **Phase 72 — Three-Sqrt × Three-Log × polynomial numerator** (`_three_sqrt_three_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1(k)), Log(h2(k)), Log(h3(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt factors and exactly 3 Log factors required; log³ sub-polynomial contributes 0
  to effective degree; `effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 5 new unit tests in `TestPhase72ThreeSqrtThreeLogPoly`.

## 2.9.0 — 2026-05-25

### Added

- **Phase 71 — Two-Sqrt × Three-Log × polynomial numerator** (`_two_sqrt_three_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), Log(h3(k)), polynomial..., bounded...)`.
  Exactly 2 Sqrt factors and exactly 3 Log factors required; log³ sub-polynomial contributes 0
  to effective degree; `effective_x2 = deg(P1) + deg(P2) + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 5 new unit tests in `TestPhase71TwoSqrtThreeLogPoly`.

## 2.8.0 — 2026-05-25

### Added

- **Phase 70 — Three-Sqrt × Two-Log × polynomial numerator** (`_three_sqrt_two_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1(k)), Log(h2(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt factors and exactly 2 Log factors required; log² sub-polynomial contributes 0
  to effective degree; `effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 5 new unit tests in `TestPhase70ThreeSqrtTwoLogPoly`.

## 2.7.0 — 2026-05-25

### Added

- **Phase 69 — One-Sqrt × Three-Log × polynomial numerator** (`_one_sqrt_three_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P), Log(h1(k)), Log(h2(k)), Log(h3(k)), polynomial..., bounded...)`.
  Exactly 1 Sqrt factor and exactly 3 Log factors required; log³ sub-polynomial contributes 0
  to effective degree; `effective_x2 = sqrt_inner_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 5 new unit tests in `TestPhase69OneSqrtThreeLogPoly`.

## 2.6.0 — 2026-05-25

### Added

- **Phase 68 — Three-Sqrt × Log × polynomial numerator** (`_three_sqrt_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging), polynomial..., bounded...)`.
  Log is sub-polynomial; `effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 5 new unit tests in `TestPhase68ThreeSqrtLogPoly`.

## 2.5.0 — 2026-05-25

### Added

- **Phase 67 — Three-Log × polynomial numerator** (`_three_log_poly_effective_x2`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Log(h3(k)), polynomial..., bounded...)`.
  Sqrt factors refused; log³ sub-polynomial; `effective_x2 = 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 5 new unit tests in `TestEvaluateSumPhase67ThreeLogPolyNumerator`.

## 2.4.0 — 2026-05-25

### Added

- **Phase 66 — Three-Sqrt × polynomial numerator** (`_three_sqrt_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), polynomial..., bounded...)`.
  Log factors refused (use Phase 63/64/65 for sqrt+log combos);
  `effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 5 new unit tests in `TestEvaluateSumPhase66ThreeSqrtPolyNumerator`.

## 2.3.0 — 2026-05-25

### Added

- **Phase 65 — Two-Sqrt × Two-Log × polynomial numerator** (`_two_sqrt_two_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), polynomial..., bounded...)`.
  log² sub-polynomial; `effective_x2 = deg(P1) + deg(P2) + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 5 new unit tests in `TestEvaluateSumPhase65TwoSqrtTwoLogPolyNumerator`.

## 2.2.0 — 2026-05-25

### Added

- **Phase 64 — Two-Log × Sqrt × polynomial numerator** (`_two_log_sqrt_poly_effective_x2`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Sqrt(P), polynomial..., bounded...)`.
  `log²(k)` is sub-polynomial; `effective_x2 = sqrt_inner_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 5 new unit tests in `TestEvaluateSumPhase64TwoLogSqrtPolyNumerator`.

## 2.1.0 — 2026-05-25

### Added

- **Phase 63 — Two-Sqrt × Log × polynomial numerator** (`_two_sqrt_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h(k)), polynomial..., bounded...)` as a
  numerator that vanishes at infinity.  Log is sub-polynomial; `effective_x2 =
  deg(P1) + deg(P2) + 2·poly_deg`.  Closes when `2·den_deg > effective_x2` or
  non-polynomial diverging denominator.
  - 5 new unit tests in `TestEvaluateSumPhase63TwoSqrtLogPolyNumerator`.

## 2.0.0 — 2026-05-25

### Added

- **Phase 62 — Two-Log × polynomial numerator** (`_two_log_poly_effective_x2`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), polynomial..., bounded...)` as a
  numerator that vanishes at infinity.  `log²(k)` grows sub-polynomially
  (`log²(k) = o(k^ε)` for any ε > 0), so the effective polynomial degree is
  unchanged by the two log factors.  `effective_x2 = 2·poly_deg`; closes when
  `2·den_deg > effective_x2` (polynomial denominator) or the denominator is
  non-polynomial diverging.  Sqrt factors are refused (belong to two-Sqrt /
  log-Sqrt family).
  - 6 new unit tests in `TestEvaluateSumPhase62TwoLogPolyNumerator`.

## 1.9.0 — 2026-05-25

**Phase 61 — Two-Sqrt × polynomial numerator (Python).**

Closes the gap where all existing Sqrt phases (51, 53, 56, 59, 60) require
exactly one Sqrt and hard-reject a second.  Handles numerators of the form
``Mul(Sqrt(P1), Sqrt(P2), polynomial_factors..., bounded_factors...)``.

Effective growth: ``k^{deg(P1)/2 + deg(P2)/2 + m}``.
Using the ×2 integer trick: ``effective_x2 = deg(P1) + deg(P2) + 2·m``.
Vanishes when ``2·den_deg > effective_x2`` (polynomial denominator) or
when the denominator is non-polynomial diverging.

``Log`` factors are refused (belong to future Log×two-Sqrt phases).

### Added

- **``_two_sqrt_poly_effective_x2``** — returns
  ``deg(P1) + deg(P2) + 2·poly_deg`` for
  ``Mul(Sqrt(P1), Sqrt(P2), polynomial_factors..., bounded_factors...)``.
  Refuses three-or-more Sqrt, any Log factor, or unrecognised factors.
- **Phase 61 branch** in ``_g_vanishes_at_infinity`` — checks
  ``2·den_deg > tsp_x2`` for polynomial denominators; falls back to
  ``_h_diverges_at_infinity`` for non-polynomial diverging denominators.
- **6 new tests** in ``TestEvaluateSumPhase61TwoSqrtPolyNumerator``.

### Changed

- Renamed ``test_two_sqrt_factors_refused`` → ``test_two_sqrt_factors_now_closed_by_phase61``
  in Phase 56 tests: Phase 61 now correctly closes what Phase 56 conservatively refused.

## 1.8.0 — 2026-05-24

**Phase 60 — Bounded × Log(diverging) × Sqrt(positive-poly) × polynomial numerator (Python).**

Closes the gap left by Phase 57 (bounded × Log × Sqrt, refuses polynomial
factors).  Allows any number of bounded factors, exactly one
``Log(diverging)`` factor, exactly one ``Sqrt(positive-leading polynomial P)``,
and any polynomial factors (total degree ``m``).

Effective growth: ``k^{1/2·deg(P) + m}`` (log is sub-polynomial).
Using the ×2 integer trick: ``effective_x2 = deg(P) + 2·m``.
Vanishes when ``2·den_deg > effective_x2`` (polynomial denominator) or
when the denominator is non-polynomial diverging.

### Added

- **``_bounded_log_sqrt_poly_effective_x2``** — returns ``deg(P) + 2·poly_deg``
  for ``Mul(bounded..., Log(diverging), Sqrt(positive-poly), polynomial_factors...)``.
  Requires exactly one Log and exactly one Sqrt; refuses two of either.
- **Phase 60 branch** in ``_g_vanishes_at_infinity`` — checks
  ``2·den_deg > blsp_x2`` for polynomial denominators; falls back to
  ``_h_diverges_at_infinity`` for non-polynomial diverging denominators.
- **6 new tests** in ``TestEvaluateSumPhase60BoundedLogSqrtPolyNumerator``.

## 1.7.0 — 2026-05-25

**Phase 59 — Bounded × Sqrt(positive-poly) × polynomial numerator (Python).**

Closes the three-way gap between Phase 53 (Sqrt × polynomial, refuses
bounded factors), Phase 56 (bounded × Sqrt, refuses polynomial factors),
and Phase 57 (bounded × Log × Sqrt, the Log specialisation).

A numerator with one ``Sqrt(positive-leading polynomial P)`` factor, any
polynomial factors (total degree ``m``), and any bounded factors has
effective growth ``k^{deg(P)/2 + m}``.  Using the ×2 integer trick:
``effective_x2 = deg(P) + 2·m``.  Vanishes when ``2·den_deg > effective_x2``
(polynomial denominator) or when the denominator is non-polynomial diverging.

### Added

- **``_bounded_sqrt_poly_effective_x2``** — returns ``deg(P) + 2·poly_deg``
  for ``Mul(bounded..., Sqrt(positive-poly), polynomial_factors...)``.
  Refuses two-Sqrt, any Log (→ Phase 57), or unrecognised factors.
- **Phase 59 branch** in ``_g_vanishes_at_infinity`` — checks
  ``2·den_deg > bsp_x2`` for polynomial denominators; falls back to
  ``_h_diverges_at_infinity`` for non-polynomial diverging denominators.
- **6 new tests** in ``TestEvaluateSumPhase59BoundedSqrtPolyNumerator``.

## 1.6.0 — 2026-05-25

**Phase 58 — Bounded × Log(diverging) × polynomial numerator (Python).**

Closes the three-way gap between Phase 54 (Log × polynomial, refuses
bounded factors), Phase 55 (bounded × Log, refuses polynomial factors),
and Phase 57 (bounded × Log × Sqrt, the Sqrt specialisation).

A numerator with one ``Log(diverging)`` factor, any polynomial factors
(total degree ``m``), and any bounded factors has effective growth
``log(k)·k^m = o(k^{m+ε})`` and vanishes when the denominator grows
strictly faster than ``k^m``.

### Added

- **``_bounded_log_poly_degree``** — returns total polynomial degree for
  ``Mul(bounded..., Log(diverging), polynomial_factors...)``.  Refuses
  two-Log, any Sqrt (→ Phase 57), or unrecognised factors.

### Changed

- ``_g_vanishes_at_infinity`` adds Phase 58 branch after Phase 57:
  ``den_deg > poly_deg`` for polynomial denominators, or
  ``_h_diverges_at_infinity`` for non-polynomial diverging denominators.

### Tests

6 new ``TestEvaluateSumPhase58BoundedLogPolyNumerator`` cases.
Full suite: **145 passed** (was 139; +6).

## 1.5.0 — 2026-05-23

**Phase 57 — Bounded × Log(diverging) × Sqrt(positive-poly) numerator
(Python).**

Closes the mixed sub-polynomial gap deferred in Phase 55 and Phase 56.
Numerator combines Log (sub-polynomial), Sqrt (half-polynomial), and
optional bounded factors.  Effective growth ``log(k)·k^{deg(P)/2}``
strictly dominated by ``k^{deg(P)/2+ε}`` for any ``ε > 0``.

### Added

- **``_bounded_log_sqrt_inner_deg``** — requires exactly one Log AND
  exactly one Sqrt; one-only patterns fall through to Phase 55 / 56.
  Two-of-either refused.

### Tests

7 new ``TestEvaluateSumPhase57BoundedLogSqrtNumerator`` cases.
Full suite: **133 passed** (was 126; +7).

## 1.4.0 — 2026-05-23

**Phase 56 — Bounded × Sqrt(diverging) numerator pattern (Python).**

Extends ``_g_vanishes_at_infinity`` with a new branch for
``Div(Mul(bounded, Sqrt(P(k))), den)`` shapes.  The bounded part is
uniformly bounded by some constant ``C``; the sqrt part grows like
``k^{deg(P)/2}``.  The whole numerator therefore has effective
polynomial degree ``deg(P)/2``, expressed here as ``deg(P)`` (the ×2
trick used elsewhere) so the comparison ``2 × den_deg > deg(P)``
stays exact in integer arithmetic.

This is the bounded-times-sqrt analogue of Phase 55 (bounded × log).
The two phases close the analogous gap between Phase 52 (bounded ×
polynomial, deg-aware) and the sub-polynomial growth families (log
and sqrt).

Bumps 1.3.0 → 1.4.0.

### Added

- **``_bounded_times_sqrt_inner_deg(node, k)``** — Phase 56 helper.
  Returns the ``Sqrt`` inner polynomial degree (×2 to stay exact)
  when ``node`` is a ``Mul`` with exactly one
  ``Sqrt(positive-leading polynomial)`` factor and all remaining
  factors bounded in ``k``.  Returns ``None`` for:

  - non-``Mul`` shapes
  - ``Mul`` with no ``Sqrt`` factor
  - ``Mul`` with two or more ``Sqrt`` factors (conservative — would
    need a combined growth-rate calculation; users can pre-simplify
    ``sqrt(k)·sqrt(k) = k``)
  - ``Mul`` with a non-bounded non-``Sqrt`` factor (e.g. bare ``k``)
  - ``Sqrt`` of a negative-leading polynomial (not real-valued)

- **Phase 56 branch in ``_g_vanishes_at_infinity``** — inserted
  after Phase 55 and before the Phase 42 polynomial widening.  Two
  sub-cases by denominator shape:

  1. Polynomial denominator (``_polynomial_degree_in_k`` returns
     ``int``): require ``2 × den_deg > sqrt_inner_deg``.
  2. Non-polynomial diverging denominator (Exp / Pow / Log×poly):
     dominates any sub-polynomial sqrt growth automatically.

- **6 new tests** in ``TestEvaluateSumPhase56BoundedTimesSqrtNumerator``
  (``test_summation.py``):
  - ``test_sin_times_sqrt_k_over_k_squared_closes`` — 1/2 < 2
  - ``test_cos_times_sqrt_k_cubed_over_k_squared_closes`` — 3/2 < 2 (tight margin)
  - ``test_two_bounded_factors_times_sqrt_closes`` — sin·cos·sqrt(k)/k²
  - ``test_bounded_times_sqrt_over_exponential_closes`` — sin·sqrt(k³)/2^k
  - ``test_sin_times_sqrt_k_cubed_over_k_refused`` — 3/2 > 1 (no vanish)
  - ``test_two_sqrt_factors_refused`` — conservative: two-sqrt patterns refused

Total: **132 tests** (was 126; +6 net new), no regressions.

### Still deferred

- Two-sqrt patterns (``sin(k)·sqrt(k)·sqrt(k+1)``) — would need
  combined growth-rate logic.
- Log + Sqrt combinations (``sin(k)·log(k)·sqrt(k)/k³``) — analogous
  to Phase 55/56 but with both sub-polynomial factors.

## 1.3.0 — 2026-05-23

**Phase 55 — Bounded×Log(diverging) numerator pattern (Python).**

Extends ``_g_vanishes_at_infinity`` with a new branch for
``Div(Mul(bounded, Log(diverging)), h(k))`` shapes.  The product of a
uniformly bounded function (``|f| ≤ C``) and ``log(h(k))`` (sub-polynomial
growth) is dominated by any polynomial or faster-growing denominator.

This is the bounded-times-log complement of Phase 52
(``Mul(bounded, polynomial)``) and Phase 54 (``Mul(Log, polynomial)``).

Bumps 1.2.0 → 1.3.0.

### Added

- **``_is_bounded_times_log_in_k(node, k)``** — Phase 55 helper.
  Returns True when ``node`` is a ``Mul`` with exactly one
  ``Log(diverging)`` factor and all remaining factors bounded in ``k``
  (via ``_is_log_of_diverging_in_k`` and ``_is_bounded_in_k``).  Requires
  exactly one log factor; two or more → False.

- **Phase 55 branch in ``_g_vanishes_at_infinity``** — inserted after
  Phase 54 and before the Phase 42 polynomial widening.  Closes
  ``Div(Mul(bounded, Log(diverging)), den)`` when ``den`` diverges
  (``_h_diverges_at_infinity`` returns True).

- **5 new tests** in ``TestEvaluateSumPhase55BoundedTimesLogNumerator``
  (``test_summation.py``):
  - ``test_sin_k_times_log_k_over_k_squared_closes`` — sin×log / k² → closes
  - ``test_cos_k_times_log_k_over_k_closes`` — cos×log / k → closes
  - ``test_two_bounded_factors_times_log_over_k_cubed_closes`` — sin·cos·log / k³
  - ``test_bounded_times_log_of_k_squared_over_k_cubed_closes`` — sin·log(k²) / k³
  - ``test_bounded_times_log_constant_denominator_refused`` — constant denominator refused

Total: 126 tests (was 121), coverage 88.75%.

## 1.2.0 — 2026-05-23

**Phase 54 — Log×polynomial numerator pattern (Python).**

Extends ``_g_vanishes_at_infinity`` with a new branch for
``Div(Mul(Log(diverging), P(k)), Q(k))`` shapes.  ``log(h(k))`` grows
sub-polynomially — slower than any positive power of ``k`` — so the
effective growth degree is just ``deg(P)``.  The quotient vanishes when
``deg(Q) > deg(P)`` (strictly; equal degrees are refused because
``log(k) × constant`` diverges).

Bumps 1.1.0 → 1.2.0.

### Added

- **``_split_log_polynomial_factor(node, k)``** — Phase 54 helper.
  Splits a ``Mul`` node into exactly one ``Log(diverging)`` factor and
  a polynomial part in ``k``; returns ``(log_factor, poly_deg_sum)`` or
  ``None`` when the shape isn't recognised (no Log factor, more than one
  Log factor, or a non-polynomial non-Log factor).

- **Phase 54 branch in ``_g_vanishes_at_infinity``** — inserted after
  Phase 53 and before the Phase 42 polynomial widening.  Closes
  ``Div(Mul(Log(diverging), P), Q)`` when ``deg(Q) > deg(poly_part)``.

- **5 new tests** in ``TestEvaluateSumPhase54LogTimesPolynomialNumerator``
  (``test_summation.py``):
  - ``test_log_k_times_k_over_k_cubed_closes`` — log×k / k³ → closes
  - ``test_log_k_times_k_squared_over_k_cubed_closes`` — log×k² / k³ → closes
  - ``test_log_k_times_k_over_k_squared_closes`` — log×k / k² → closes
  - ``test_log_k_times_k_squared_over_k_squared_refused`` — equal degrees
    refused (log(k)*k²/k² = log(k) → diverges)
  - ``test_regression_log_k_over_k_cubed_still_phase50`` — plain Log(k)/k³
    still closes via Phase 50 (not Phase 54)

### Tests

121 passed (was 116; +5 net new — Phase 54).

---

## 1.1.0 — 2026-05-23

**Phase 51 + Phase 53 — Sqrt numerator patterns (Python port).**

Ports Rust/TypeScript ``cas-summation`` Phase 51 (0.9.0) and introduces
Phase 53 in all three languages simultaneously.  Extends
``_g_vanishes_at_infinity`` with two new branches for square-root
numerator shapes, both using ×2 integer arithmetic to avoid floating-point
comparisons.

Bumps 1.0.0 → 1.1.0.

### Added

- **``_sqrt_effective_half_degree_x2(node, k)``** — Phase 51 helper.
  Returns ``deg(P)`` (= 2 × effective half-degree) for ``Sqrt(P(k))``
  with a positive-leading-coefficient polynomial inner ``P``.  Returns
  ``None`` for non-Sqrt nodes, non-polynomial inners, and negative-leading-
  coefficient polynomials (those produce complex values for large ``k``).

- **``_sqrt_poly_numerator_effective_degree_x2(node, k)``** — Phase 53
  helper.  For a ``Mul`` node containing exactly one ``Sqrt(P)`` factor
  and any number of polynomial-in-``k`` factors, returns
  ``deg(P) + 2·deg(Q)`` (= 2 × the combined effective growth degree).
  Returns ``None`` for plain ``Sqrt`` nodes (handled by Phase 51),
  non-Mul nodes, or Mul nodes with more than one Sqrt factor, non-
  polynomial non-Sqrt factors, or negative-leading-coefficient inner
  polynomials.

### Changed

- ``_g_vanishes_at_infinity`` adds two new branches after Phase 50 (log
  numerator) and Phase 49/52 (bounded / bounded×poly):

  **Phase 51 branch** — fires when ``num = Sqrt(P)`` with
  ``_sqrt_effective_half_degree_x2(num, k) = d``.  Closes when
  ``2 * deg(den) > d`` (i.e. denominator degree strictly exceeds the
  sqrt half-degree).

  **Phase 53 branch** — fires when ``num = Mul(Sqrt(P), polynomial)``
  with ``_sqrt_poly_numerator_effective_degree_x2(num, k) = e``.
  Closes when ``2 * deg(den) > e``.

  Both branches insert between Phase 52 (bounded × polynomial) and
  Phase 42 (pure rational degree comparison).

### Added — tests

``tests/test_summation.py``

**``TestEvaluateSumPhase51SqrtNumerator``** — 4 new cases:
- ``phase51_sqrt_k_over_k_squared_closes`` — eff deg ½ < 2.
- ``phase51_sqrt_k_squared_over_k_cubed_closes`` — eff deg 1 < 3.
- ``phase51_sqrt_k_over_k_equal_degrees_refused`` — eff deg 1 = 1.
- ``phase51_sqrt_of_negative_polynomial_refused`` — Sqrt(−k) refused.

**``TestEvaluateSumPhase53SqrtTimesPolynomialNumerator``** — 5 new cases:
- ``phase53_sqrt_k_times_k_over_k_cubed_closes`` — eff deg 3/2 < 3.
- ``phase53_sqrt_k_squared_times_k_over_k_cubed_closes`` — eff 2 < 3.
- ``phase53_sqrt_k_times_k_squared_over_k_cubed_closes`` — eff 5/2 < 3.
- ``phase53_sqrt_k_times_k_squared_over_k_squared_stays`` — eff 5/2 > 2.
- ``phase53_regression_sqrt_k_over_k_squared_still_via_phase51`` — plain
  Sqrt bypasses Phase 53 and closes via Phase 51.

Full suite: **116 passed** (was 107; +9 net new — 4 Phase 51 + 5 Phase 53).

## 1.0.0 — 2026-05-22

**Phase 52 — Bounded × polynomial numerator pattern.**

Extends Phase 50 to recognise that ``Mul(bounded, polynomial)``
numerators have effective growth equal to the polynomial part's
degree.  Closes telescopes like ``sin(k)·k/k³``, ``k·cos(k)/k²``,
where the numerator mixes a bounded factor with a non-trivial
polynomial factor.

Major version bump (1.0.0) reflects the maturity of the
vanishing-at-infinity recogniser: Phases 41–52 collectively handle
the realistic cases of rational, transcendental, logarithmic,
and mixed bounded × polynomial summands.

Builds on Phase 50 (0.8.0) which added ``_is_log_of_diverging_in_k``.
Bumps 0.8.0 → 1.0.0.

### Added

- **`_split_bounded_polynomial_factor(node, k)`** — partitions a
  ``Mul`` node's factors into a bounded aggregate and a summed
  polynomial degree; returns ``None`` if any factor is neither
  bounded nor polynomial, or if no non-constant bounded factor
  exists (those go through Phase 42).

### Changed

- ``_g_vanishes_at_infinity`` now has a Phase 52 branch between
  Phase 50 (log numerator) and Phase 42 (degree-aware): when
  the numerator factors as ``bounded × polynomial`` with positive
  polynomial degree, the quotient vanishes iff the denominator's
  polynomial degree strictly exceeds the polynomial part's degree.

### Added — tests

`tests/test_summation.py::TestEvaluateSumPhase52BoundedTimesPolynomial`
— 5 new cases:

- ``sin(k)·k/k³`` closes (bounded × deg 1 / deg 3).
- ``k·cos(k)/k²`` closes (order of factors doesn't matter).
- ``sin(k)·k²/k³`` closes (deg 2 < 3).
- ``sin(k)·k²/k²`` stays unevaluated (degrees tie).
- Regression: ``k/k²`` still closes via Phase 42 (Phase 52 doesn't
  interfere when no bounded factor is present).

Full suite: **103 passed** (was 98; +5 net new).

## 0.8.0 — 2026-05-22

**Phase 50 — Log/polynomial growth-rate recogniser.**

Extends ``_g_vanishes_at_infinity`` to accept
``Div(Log(diverging), diverging)`` shapes — i.e. ``log(h(k))`` over
any positive-degree polynomial / exponential / b^k diverging
denominator.  The squeeze argument: ``log(h) → ∞`` at a logarithmic
rate, while the denominator grows strictly faster, so the quotient
vanishes.  Closes the long-deferred ``log(k)/k²`` case noted in the
Phase 49 CHANGELOG.

> Note: stacks on Phase 49 (PR #3933, in flight) and the Phase 49
> TS/Rust port (PR #3936).  Builds standalone — uses only
> ``_h_diverges_at_infinity`` (Phase 43+44), not Phase 49's
> ``_is_bounded_in_k``.  Independent from a code path perspective.

### Added

- **`_is_log_of_diverging_in_k(node, k)`** — recogniser for
  ``Log(h(k))`` with ``h(k) → +∞``.  Sign-aware: delegates to
  ``_h_diverges_at_infinity`` on the *full* ``Log(...)`` node, which
  routes through Phase 44's Log branch (already refuses
  ``Log(-k)`` / ``Log(Mul(-1, k))``-style negative shapes whose log
  isn't real-valued).

### Changed

- ``_g_vanishes_at_infinity`` now has a Phase 50 branch between
  the Phase 41 constant-numerator fast path and the Phase 42
  degree-aware path: if the numerator is ``Log(diverging)`` AND
  the denominator diverges, the quotient vanishes.

### Added — tests

`tests/test_summation.py::TestEvaluateSumPhase50LogOverPolynomial`
— 5 new cases:

- ``test_log_over_k_squared_closes`` —
  ``∑ [log(k)/k² − log(k+1)/(k+1)²]`` closes.
- ``test_log_over_k_cube_closes`` — higher denominator degree.
- ``test_log_of_polynomial_argument`` — non-trivial inner
  polynomial (``Log(k²+1)/k³``).
- ``test_log_of_constant_numerator_still_refused`` — regression:
  ``Log(5)`` is constant in ``k``, so Phase 41 catches it (not
  Phase 50).  Test pins that the sum still closes via the right
  branch.
- ``test_log_of_negative_argument_refused`` — regression:
  ``Log(Mul(-1, k))`` is complex for odd k; Phase 50 must NOT
  accidentally close the sum.  Delegates to Phase 44's
  sign-aware check.

Full suite: **98 passed** (was 93; +5 net new).

### Still deferred

- ``sqrt(k)/k²`` style growth-rate gaps (sqrt grows faster than
  log but slower than any positive integer power).  Future phase.
- Transcendental limit-finder for general non-polynomial /
  non-Log / non-Sin/Cos shapes.

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
