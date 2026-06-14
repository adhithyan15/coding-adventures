# Changelog

## 2.28.0 — 2026-05-29

### Added — Track I2 (Closed-form transcendental infinite sums port)

Ports the Python ``cas_summation.series_closed_forms`` module (Track I1,
PR #5382) to TypeScript ``cas-summation``.  Pattern-matches the
canonical convergent infinite series and emits their closed forms when
``hi = %inf``:

- ``sum(1/k^(2m), k, 1, %inf) → ζ(2m) · π^(2m)`` for ``m = 1..6``
  (Basel through ``ζ(12) = 691·π¹²/638512875``).
- ``sum((-1)^(k-1)/k, k, 1, %inf) → log(2)`` (Mercator).
- ``sum((-1)^(k-1)/k^(2m), k, 1, %inf) → η(2m) · π^(2m)`` for
  ``m = 1..3`` (Dirichlet eta).
- ``sum(1/k!, k, 0, %inf) → %e``.
- ``sum(x^k/k!, k, 0, %inf) → exp(x)`` (symbolic ``x ≠ k``).
- ``sum((-1)^k · x^(2k)/(2k)!, k, 0, %inf) → cos(x)``.
- ``sum((-1)^k · x^(2k+1)/(2k+1)!, k, 0, %inf) → sin(x)``.
- ``sum(x^(2k)/(2k)!, k, 0, %inf) → cosh(x)``.
- ``sum(x^(2k+1)/(2k+1)!, k, 0, %inf) → sinh(x)``.

The new ``tryClosedFormSeries`` handler is wired into ``evaluateSum``
between the existing ``trySpecialInfinite`` (legacy Basel + Leibniz
table) and the small-range numeric path; pre-existing tests stay on
their original routes because the legacy table fires first for the
overlapping ``ζ(2)`` / ``ζ(4)`` / ``π/4`` patterns.

One generic ``bernoulliRational`` helper computes ``B_n`` via the
textbook recurrence ``B_0 = 1; Σ_{j=0}^{n} C(n+1, j) · B_j = 0``.  Six
even-zeta exponents and three even-eta exponents share the same code —
no per-degree tables.  The recurrence depth is bounded by ``n ≤ 12``,
so the helper is provably terminating, and results are cached in a
module-level array.

All numeric work is exact (BigInt-backed ``Frac``); the closed forms
emerge as ``π^(2m) / denom`` IR shapes that match the parser-emitted
forms verified by the test suite.

### Notes

Falls through (returns ``undefined``) for: odd zeta ``ζ(2m+1)``,
indices past ``m > 6``, wrong lower bound (zeta requires ``lo=1``,
Taylor requires ``lo=0``), finite upper bound, and any non-table
summand (``sin(k)``, ``log(k)``, etc.).

## 2.27.0 — 2026-05-29

### Added — Track H2 (Gosper hypergeometric summation port)

Ports the Python ``cas_summation.gosper`` module (Track H1, PR #5366) to
TypeScript ``cas-summation``.  When the summand ``a(k)`` is a
hypergeometric term — a product of a polynomial in ``k`` with constant-
base exponentials ``c^(αk+β)`` and ``GammaFunc(k+s)`` factors — and the
upper bound is finite, ``evaluateSum`` now attempts Gosper's algorithm
to find an antidifference ``T(k)`` satisfying ``T(k+1) − T(k) = a(k)``
and returns the closed form ``T(hi+1) − T(lo)``.

This unlocks closed forms for the classical hypergeometric shapes the
existing narrow recognisers miss, e.g.:

- ``∑_{k=1}^{N} k·2^k = (N−1)·2^(N+1) + 2``
- ``∑_{k=0}^{N} k·k! = (N+1)! − 1``

### Changes

- ``src/gosper.ts``: new module — full Gosper pipeline (structural
  decomposition → ratio computation → Petkovšek shift-coprime
  normalisation → Gosper degree bound → linear system solve via
  Gaussian elimination over exact ``bigint`` rationals).  Mirrors the
  Python module 1:1 including the boundary-singularity cancellation
  step that handles removable factorial denominators at ``k = lo``.

- ``src/gosper.ts``: defensive ``MAX_POLY_DEGREE = 64`` cap on
  polynomial exponents during IR-to-poly conversion.  Without this,
  an adversarial summand like ``Pow(k, 10**9)`` would balloon the
  internal polynomial representation into a memory-bomb.  Gosper-
  summable expressions in practice have very small polynomial degree
  (typically ≤ 5).

- ``src/index.ts``: wire ``tryGosperSum`` into the dispatch chain at the
  same insertion point as Python (step 5b in ``summation.py``) — after
  all narrow recognisers (constant, geometric, Faulhaber, telescoping,
  classic infinite series, small-range numeric) and before the
  Apart-retry telescope chain and the unevaluated fallthrough.  Guarded
  by ``if (!infUpper)`` to mirror Python: Gosper returns
  ``T(hi+1) − T(lo)`` which is only meaningful for finite ``hi``;
  infinite upper bounds belong to the limit-aware paths above.

- ``tests/gosper.test.ts``: 15 tests — 4 acceptance cases (``k·2^k``,
  ``k·k!``, ``2^k`` regression, mixed-handler dispatcher), 2 fall-
  through safety cases (``sin(k)``, ``log(k)``), 2 regression cases
  (Faulhaber and constant handlers still take priority), 4 internal
  helper tests, 2 structural pieces (``decompose`` + ``hypRatio``),
  and 1 DoS-cap test verifying ``Pow(k, 10**9)`` is refused promptly.

- ``package.json``: minor bump to 2.27.0.

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

## 2.25.0 — 2026-05-28

### Added — Track B2 (Apart-retry telescope chain port: Phase 40 + Phase 46)

Ports the Python ``sum_handler`` Apart-retry composition (``symbolic-vm``
Phase 40 + Phase 46) to TypeScript ``cas-summation``.  After the existing
direct telescoping / vanishing-at-infinity / classic-series pipeline falls
through on a rational summand ``Div(P(k), Q(k))``, ``evaluateSum`` now:

1. Dispatches ``Apart(f, k)`` through the user-provided ``evalFn`` —
   typically a ``symbolic-vm`` VM with the Apart handler installed
   (Track B1, ``symbolic-vm`` 0.13.0).
2. If Apart actually decomposes ``f`` (returned shape structurally differs
   from the input), normalises the ``Add(a, Div(-c, d))`` /
   ``Add(Neg(b), a)`` shapes to ``Sub`` via the existing
   ``normaliseAddNegToSub`` helper.
3. Retries the full pipeline on the normalised result with a one-shot
   ``apartRetried`` guard so we never recurse a second time.

This is the long-promised TypeScript closure of the classic
``∑_{k=1}^∞ 1/(k(k+1)) = 1`` telescope: Apart decomposes the summand to
``Add(Div(1, k), Div(-1, k+1))`` which the Phase 40+46 normaliser
rewrites to ``Sub(1/k, 1/(k+1))``; the structural telescope detector
fires and Phase 41 emits ``1`` (since ``1/(k+1) → 0`` at infinity).

- ``src/index.ts``: refactor ``evaluateSum`` to delegate to a private
  ``evaluateSumInner`` carrying an ``apartRetried`` flag; add the
  Apart-retry block just above the unevaluated fallback.
- ``package.json``: bump to 2.25.0; add ``@coding-adventures/symbolic-vm``
  as a ``devDependency`` so the tests can construct a real VM with the
  Apart handler installed.
- ``BUILD``: chain-install ``cas-factor`` and ``symbolic-vm`` ahead of the
  package install so CI has the transitive ``file:`` deps available.
- ``tests/cas-summation.test.ts``: new ``"summation: Track B2 Apart-retry
  telescope chain"`` describe block — 6 cases (acceptance, three-term,
  Phase 46 constant numerator, irreducible-denominator fallthrough,
  polynomial-summand fallthrough, non-telescoping-after-Apart fallthrough).

When the user's ``evalFn`` does not dispatch ``Apart`` (e.g. a bare
arithmetic walker without the symbolic-vm handler), the Apart attempt
returns ``Apply(Apart, [f, k])`` which structurally differs from ``f``,
but the recursive retry on that shape also returns unevaluated — so the
original unevaluated ``Sum`` is preserved.  No spurious "closure" can
leak out.

## 2.24.0 — 2026-05-28

### Removed — Track A2 cleanup (delete 27 grid helpers superseded by Phase 86)

Pure deletion: removes the 27 hand-written ``N-Sqrt × M-Log × polynomial``
helpers (Phases 59–85), their dispatcher branches, and their tests, now
that ``logSqrtPolyEffectiveDegGeneric`` preempts the entire grid.  No
behavior change.

- ``src/index.ts``: removed dispatcher branches for Phases 59–85 inside
  ``gVanishesAtInfinity`` and the helper functions
  ``boundedSqrtPolyEffectiveDeg`` through ``twoSqrtSixLogPolyEffectiveDeg``
  (~1,390 lines).  ``logSqrtPolyEffectiveDegGeneric`` and all earlier
  helpers (``splitBoundedPolynomialFactor``, ``boundedTimesSqrtHalfDegree``,
  …) remain untouched.
- ``tests/cas-summation.test.ts``: removed every ``describe("summation:
  Phase 5{9}…")`` through ``Phase 85`` block (~2,503 lines, 96 tests).
  Phase 56–58 and Phase 86 describes remain.
- Vitest count drops from 179 → 83; all surviving tests pass.

## 2.23.0 — 2026-05-28

### Added — Phase 86 cleanup (generic log × sqrt × polynomial recogniser)

Mirrors the Python `2.23.0` cleanup: a single generic helper supersedes the
hand-written grid of `N-Sqrt × M-Log × polynomial` recognisers (Phases 59-85).
The convergence math is identical for every non-negative `(N, M, K)`:

- The product of `N` `Log(diverging)` factors is sub-polynomial
  (`log^N(k) = o(k^ε)`), so `N` contributes 0 to the effective growth degree.
- Each `Sqrt(P_i)` contributes `deg(P_i)/2`.
- Each polynomial factor `Q_j` contributes its own `deg(Q_j)`.
- Bounded factors contribute 0.

`logSqrtPolyEffectiveDegGeneric(node, k)` returns `Σ sqrtHalfDeg + Σ polyDeg`
when the numerator matches; the dispatcher in `gVanishesAtInfinity` inserts
this branch between Phase 58 and Phase 59 so it preempts the entire grid for
every shape the grid was meant to cover (and many it wasn't — e.g. seven
`Log` factors, six `Sqrt` factors, arbitrary mixes).

The hand-written grid helpers (`twoSqrtPolyEffectiveDeg`, `fiveLogPolyEffectiveDeg`,
…) remain in place for now but are now dead code; a follow-up cleanup PR will
delete them.

6 new tests in the "Phase 86 generic" describe block:

- `seven logs over k²` (grid stops at 6 — generic handles it).
- `six sqrts of k over k⁴` (grid stops at 5 — generic handles it).
- `three sqrts × seven logs × k over k⁵` (mixed; outside the grid).
- `refuses unrecognised factor (Exp)` (must not silently close a divergent sum).
- `refuses Sqrt of negative polynomial` (complex-valued — refuse).
- `pure bounded falls through to Phase 49` (regression — generic returns
  undefined so Phase 49 takes over).

## 2.22.0 — 2026-05-26

### Added

- **Phase 85 — Two-Sqrt × Six-Log × polynomial numerator** (`twoSqrtSixLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), Log(h6(k)), polynomial..., bounded...)`.
  Exactly 2 Sqrt and exactly 6 Log factors; `log⁶(k)` sub-polynomial — contributes 0 to effective
  degree; effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  Closes when `denDeg > twoSqrtSixLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 3 new tests in the "Phase 85" describe block.

## 2.21.0 — 2026-05-25

### Added

- **Phase 81 — Four-Sqrt × Five-Log × polynomial numerator** (`fourSqrtFiveLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 4 Sqrt and exactly 5 Log factors; `log⁵(k)` sub-polynomial — contributes 0 to effective
  degree; effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + sqrtHalfDeg4 + polyDeg.
  Closes when `denDeg > fourSqrtFiveLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 3 new tests in the "Phase 81" describe block.

## 2.20.0 — 2026-05-25

### Added

- **Phase 80 — Three-Sqrt × Five-Log × polynomial numerator** (`threeSqrtFiveLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1(k)), ..., Log(h5(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt and exactly 5 Log factors; `log⁵(k)` sub-polynomial — contributes 0 to effective
  degree; effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
  Closes when `denDeg > threeSqrtFiveLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 3 new tests in the "Phase 80" describe block.

## 2.19.0 — 2026-05-25

### Added

- **Phase 84 — One-Sqrt × Six-Log × polynomial numerator** (`oneSqrtSixLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), Log(h6(k)), polynomial..., bounded...)`.
  Exactly 1 Sqrt and exactly 6 Log factors; `log⁶(k)` sub-polynomial — contributes 0 to effective
  degree; effective degree = sqrtHalfDeg + polyDeg.
  Closes when `denDeg > oneSqrtSixLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 3 new tests in the "Phase 84" describe block.

## 2.18.0 — 2026-05-25

### Added

- **Phase 82 — Five-Sqrt × Five-Log × polynomial numerator** (`fiveSqrtFiveLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Sqrt(P5), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 5 Sqrt and exactly 5 Log factors; `log⁵(k)` sub-polynomial — contributes 0 to effective
  degree; effective degree = sqrtHalfDeg1+sqrtHalfDeg2+sqrtHalfDeg3+sqrtHalfDeg4+sqrtHalfDeg5 + polyDeg.
  Closes when `denDeg > fiveSqrtFiveLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 3 new tests in the "Phase 82" describe block.
- **Phase 83 — Six-Log × polynomial numerator** (`sixLogPolyEffectiveDeg`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), Log(h6(k)), polynomial..., bounded...)`.
  Exactly 6 Log factors and zero Sqrt factors; `log⁶(k)` sub-polynomial — contributes 0 to
  effective degree; effective degree = polyDeg.
  Closes when `denDeg > sixLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 3 new tests in the "Phase 83" describe block.

## 2.17.0 — 2026-05-25

### Added

- **Phase 79 — Two-Sqrt × Five-Log × polynomial numerator** (`twoSqrtFiveLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 2 Sqrt and exactly 5 Log factors; `log⁵(k)` sub-polynomial — contributes 0 to effective
  degree; effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  Closes when `denDeg > twoSqrtFiveLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 3 new tests in the "Phase 79" describe block.

## 2.16.0 — 2026-05-25

### Added

- **Phase 78 — One-Sqrt × Five-Log × polynomial numerator** (`oneSqrtFiveLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 1 Sqrt factor and exactly 5 Log factors; `log⁵(k)` sub-polynomial — contributes 0 to
  effective degree; effective degree = sqrtHalfDeg + polyDeg.
  Closes when `denDeg > oneSqrtFiveLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 3 new tests in the "Phase 78" describe block.

## 2.15.0 — 2026-05-25

### Added

- **Phase 77 — Five-Log × polynomial numerator** (`fiveLogPolyEffectiveDeg`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 5 Log factors; Sqrt factors explicitly refused so Sqrt-bearing phases (73–76, 78+) are
  not shadowed.  `log⁵(k)` sub-polynomial — contributes 0 to effective degree;
  effective degree = polyDeg.
  Closes when `denDeg > fiveLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 3 new tests in the "Phase 77" describe block.

## 2.14.0 — 2026-05-25

### Added

- **Phase 76 — Three-Sqrt × Four-Log × polynomial numerator** (`threeSqrtFourLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt and exactly 4 Log factors; `log⁴(k)` sub-polynomial — contributes 0 to effective
  degree; effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
  Closes when `denDeg > threeSqrtFourLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 3 new tests in the "Phase 76" describe block.

## 2.13.0 — 2026-05-25

### Added

- **Phase 75 — Two-Sqrt × Four-Log × polynomial numerator** (`twoSqrtFourLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), polynomial..., bounded...)`.
  Exactly 2 Sqrt and exactly 4 Log factors; `log⁴(k)` sub-polynomial — contributes 0 to effective
  degree; effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  Closes when `denDeg > twoSqrtFourLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 3 new tests in the "Phase 75" describe block.

## 2.12.0 — 2026-05-25

### Added

- **Phase 74 — One-Sqrt × Four-Log × polynomial numerator** (`oneSqrtFourLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), polynomial..., bounded...)`.
  Exactly 1 Sqrt and exactly 4 Log factors; `log⁴(k)` sub-polynomial — contributes 0 to effective
  degree; effective degree = sqrtHalfDeg + polyDeg.
  Closes when `denDeg > oneSqrtFourLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 3 new tests in the "Phase 74" describe block.

## 2.11.0 — 2026-05-25

### Added

- **Phase 73 — Four-Log × polynomial numerator** (`fourLogPolyEffectiveDeg`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), polynomial..., bounded...)`.
  Exactly 4 Log factors; Sqrt factors refused.  `log⁴(k)` sub-polynomial — contributes 0 to
  effective degree; effective degree = polyDeg.
  Closes when `denDeg > fourLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 4 new tests in the "Phase 73" describe block.

## 2.10.0 — 2026-05-25

### Added

- **Phase 72 — Three-Sqrt × Three-Log × polynomial numerator** (`threeSqrtThreeLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1(k)), Log(h2(k)), Log(h3(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt and exactly 3 Log factors; log³ sub-polynomial contributes 0 to effective
  degree. effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
  Closes when `denDeg > threeSqrtThreeLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 4 new tests in the "Phase 72" describe block.

## 2.9.0 — 2026-05-25

### Added

- **Phase 71 — Two-Sqrt × Three-Log × polynomial numerator** (`twoSqrtThreeLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), Log(h3(k)), polynomial..., bounded...)`.
  Exactly 2 Sqrt and exactly 3 Log factors; log³ sub-polynomial contributes 0 to effective
  degree. effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  Closes when `denDeg > twoSqrtThreeLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 4 new tests in the "Phase 71" describe block.

## 2.8.0 — 2026-05-25

### Added

- **Phase 70 — Three-Sqrt × Two-Log × polynomial numerator** (`threeSqrtTwoLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1(k)), Log(h2(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt and exactly 2 Log factors; log² sub-polynomial contributes 0 to effective
  degree. effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
  Closes when `denDeg > threeSqrtTwoLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 4 new tests in the "Phase 70" describe block.

## 2.7.0 — 2026-05-25

### Added

- **Phase 69 — One-Sqrt × Three-Log × polynomial numerator** (`oneSqrtThreeLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P), Log(h1(k)), Log(h2(k)), Log(h3(k)), polynomial..., bounded...)`.
  Exactly 1 Sqrt and exactly 3 Log factors; log³ sub-polynomial contributes 0 to effective
  degree. effective degree = sqrtHalfDeg + polyDeg.
  Closes when `denDeg > oneSqrtThreeLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 4 new tests in the "Phase 69" describe block.

## 2.6.0 — 2026-05-25

### Added

- **Phase 68 — Three-Sqrt × Log × polynomial numerator** (`threeSqrtLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging), polynomial..., bounded...)`.
  Log sub-polynomial; effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
  Closes when `denDeg > threeSqrtLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 4 new tests in the "Phase 68" describe block.

## 2.5.0 — 2026-05-25

### Added

- **Phase 67 — Three-Log × polynomial numerator** (`threeLogPolyEffectiveDeg`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Log(h3(k)), polynomial..., bounded...)`.
  Sqrt factors refused; log³ sub-polynomial; effective degree = polyDeg.
  Closes when `denDeg > threeLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 4 new tests in the "Phase 67" describe block.

## 2.4.0 — 2026-05-25

### Added

- **Phase 66 — Three-Sqrt × polynomial numerator** (`threeSqrtPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), polynomial..., bounded...)`.
  Log factors refused (use Phase 63/64/65 for sqrt+log combos);
  effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
  Closes when `denDeg > threeSqrtPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 4 new tests in the "Phase 66" describe block.

## 2.3.0 — 2026-05-25

### Added

- **Phase 65 — Two-Sqrt × Two-Log × polynomial numerator** (`twoSqrtTwoLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), polynomial..., bounded...)`.
  log² sub-polynomial; effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  Closes when `denDeg > twoSqrtTwoLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 4 new tests in the "Phase 65" describe block.

## 2.2.0 — 2026-05-25

### Added

- **Phase 64 — Two-Log × Sqrt × polynomial numerator** (`twoLogSqrtPolyEffectiveDeg`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Sqrt(P), polynomial..., bounded...)`.
  log² sub-polynomial; effective degree = sqrtHalfDeg + polyDeg.
  Closes when `denDeg > twoLogSqrtPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 4 new tests in the "Phase 64" describe block.

## 2.1.0 — 2026-05-25

### Added

- **Phase 63 — Two-Sqrt × Log × polynomial numerator** (`twoSqrtLogPolyEffectiveDeg`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h(k)), polynomial..., bounded...)`.
  Log is sub-polynomial; effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  Closes when `denDeg > twoSqrtLogPolyEffectiveDeg` or non-polynomial diverging denominator.
  - 4 new tests in the "Phase 63" describe block.

## 2.0.0 — 2026-05-25

### Added

- **Phase 62 — Two-Log × polynomial numerator** (`twoLogPolyEffectiveDeg`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), polynomial..., bounded...)` as a
  numerator that vanishes at infinity.  Effective degree = `poly_deg` (log²
  contributes nothing).  Closes when `denDeg > twoLogPolyEffectiveDeg` or
  denominator is non-polynomial diverging.  Sqrt factors refused.
  - 4 new tests in the "Phase 62" describe block.

## 1.9.0 — 2026-05-25

**Phase 61 — Two-Sqrt × polynomial numerator (TypeScript port).**

Ports Python ``cas-summation`` 1.9.0.  Closes the gap where all prior Sqrt
phases (51, 53, 56, 59, 60) hard-reject a second Sqrt operand.

Effective growth: ``k^{sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg}``.
TypeScript convention: compare ``denDeg > tspDeg`` (actual half-degrees, no ×2).

### Added

- **`twoSqrtPolyEffectiveDeg(node, k)`** — returns
  ``sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg`` for
  ``Mul(Sqrt(P1), Sqrt(P2), poly_factors..., bounded_factors...)``.
  Refuses three-or-more Sqrt, any Log factor, or unrecognised factors.
- **Phase 61 branch** in ``gVanishesAtInfinity`` — checks
  ``denDeg > tspDeg`` for polynomial denominators; falls back to
  ``hDivergesAtInfinity`` for non-polynomial diverging denominators.
- **4 new tests** in the Phase 61 ``describe`` block.

## 1.8.0 — 2026-05-24

**Phase 60 — Bounded × Log(diverging) × Sqrt(positive-poly) × polynomial
numerator (TypeScript port).**

Ports Python ``cas-summation`` 1.8.0.  Closes the gap left by Phase 57
(bounded × Log × Sqrt, refuses polynomial factors).

Effective growth: ``log(k)·k^{sqrtHalfDeg + polyDeg} = o(k^{sqrtHalfDeg+polyDeg+ε})``.
TypeScript convention: compare ``denDeg > sqrtHalfDeg + polyDeg`` (actual
half-degree, no ×2).

### Added

- **`boundedLogSqrtPolyEffectiveDeg(node, k)`** — returns
  ``sqrtHalfDeg + polyDeg`` for
  ``Mul(bounded..., Log(diverging), Sqrt(positive-poly), polynomial_factors...)``.
  Requires exactly one Log and exactly one Sqrt; refuses two of either.
- **Phase 60 branch** in ``gVanishesAtInfinity`` — checks
  ``denDeg > blspDeg`` for polynomial denominators; falls back to
  ``hDivergesAtInfinity`` for non-polynomial diverging denominators.
- **4 new tests** in the Phase 60 ``describe`` block.

## 1.7.0 — 2026-05-25

**Phase 59 — Bounded × Sqrt(positive-poly) × polynomial numerator
(TypeScript port).**

Ports Python ``cas-summation`` 1.7.0.  Fills the gap between Phase 53
(Sqrt × polynomial, refuses bounded factors) and Phase 56 (bounded × Sqrt,
refuses polynomial factors).

Effective growth: ``C·k^{deg(P)/2 + polyDeg}``.  ×2 trick:
``effective_x2 = deg(P) + 2·polyDeg``.  Vanishes when
``2·denDeg > effective_x2`` or non-polynomial diverging denominator.

### Added

- **`boundedSqrtPolyEffectiveX2(node, k)`** — returns
  ``sqrtInnerDegX2 + 2·polyDeg`` for
  ``Mul(bounded..., Sqrt(positive-poly), polynomial_factors...)``.
  Refuses two-Sqrt, any Log (→ Phase 57), or unrecognised factors.
- **Phase 59 branch** in ``gVanishesAtInfinity`` — checks
  ``2·denDeg > bspX2`` for polynomial denominators; falls back to
  ``hDivergesAtInfinity`` for non-polynomial diverging denominators.
- **4 new tests** in the Phase 59 ``describe`` block.

## 1.6.0 — 2026-05-25

**Phase 58 — Bounded × Log(diverging) × polynomial numerator
(TypeScript port).**

Ports Python ``cas-summation`` 1.6.0.  Fills the gap between Phase 54
(Log × polynomial, refuses bounded factors) and Phase 55 (bounded × Log,
refuses polynomial factors).

### Added

- **`boundedLogPolyDegree(node, k)`** — returns total polynomial degree
  for ``Mul(bounded..., Log(diverging), polynomial_factors...)``.  Refuses
  two-Log, Sqrt (→ Phase 57), or unrecognised factors.

### Changed

- ``gVanishesAtInfinity`` adds Phase 58 branch after Phase 57:
  ``denDeg > polyDeg`` for polynomial denominators, or
  ``hDivergesAtInfinity`` for non-polynomial diverging denominators.

### Tests

4 new ``summation: Phase 58 bounded × log × polynomial numerator`` cases.
Full suite: **77 passed** (was 73; +4).

## 1.5.0 — 2026-05-24

**Phase 57 — Bounded × Log(diverging) × Sqrt(positive-poly) numerator
(TypeScript port).**

Ports Python ``cas-summation`` 1.5.0 (PR #4215).  Closes the mixed
sub-polynomial gap left by Phase 55 (bounded × Log) and Phase 56
(bounded × Sqrt).  The Log and Sqrt factors must both be present;
one-only patterns continue to fall through to Phase 55 / 56.

### Added

- **`boundedLogSqrtHalfDegree(node, k)`** — returns the Sqrt half-degree
  for ``Mul`` with exactly one ``Log(diverging)`` AND one ``Sqrt(positive-
  poly)`` factor (plus optional bounded factors).  Returns ``undefined``
  for zero/two-Log, zero/two-Sqrt, or unrecognised factors.

### Changed

- ``gVanishesAtInfinity`` adds Phase 57 branch after Phase 56, comparing
  ``denDeg > halfDeg`` (polynomial) or short-circuiting on non-polynomial
  diverging denominator.

### Tests

4 new ``summation: Phase 57 bounded × log × sqrt numerator`` cases.
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
