# Changelog

## 0.5.0 — 2026-05-29

**Track K1 — n-variate Hensel lifting (n ≥ 3) via iterated bivariate.**

Closes the multivariate-factoring gap in the MACSYMA-truly-finish plan
(`code/specs/macsyma-truly-finish-plan.md`, Track K1).  Extends the
bivariate Hensel lift of Track D1 to an arbitrary number of variables
using **one generic iterated-lift algorithm** — no per-variable-count
helpers.

Adds two new public exports from `cas_factor.hensel`:

* `NPoly = dict[tuple[int, ...], Fraction]` — sparse-dict
  representation generalising `BiPoly`.  Tuple length equals the
  variable count; entry `i` is the exponent of variable `v_i`.
* `try_n_variate_hensel(f: NPoly, num_vars: int) -> list[NPoly] | None` —
  attempts a non-trivial factoring of `f`.  Returns `None` (clean
  fall-through) when `num_vars < 2`, the polynomial is irreducible,
  or no lucky specialisation tuple produces a usable univariate image
  within the bounded search.

Algorithm
~~~~~~~~~

1. Pick the main variable ``v_0`` (lex-first).
2. Specialise ``v_1..v_{n−1}`` with a bounded list of small integer
   tuples (at most 10 tuples).  Try the all-ones tuple first, then
   each primitive value ``{1, 2, −1, 3, −2}`` constant across all
   slots, then single-coordinate perturbations.
3. Verify the univariate image at the specialisation is squarefree and
   has full ``v_0``-degree (Track D1's "lucky-substitution" check
   generalised to n variables).
4. Factor the univariate image via the existing rational-root +
   Kronecker + BZH chain.
5. **Lift one variable at a time** from ``Q[v_0]`` up through
   ``Q[v_0, v_1]``, ``Q[v_0, v_1, v_2]``, …, ``Q[v_0, …, v_{n−1}]``
   by Hensel-style expansion in powers of ``(v_k − a_k)``.
6. Each lift step solves a **coefficient-ring diophantine** in
   ``Q[v_0, …, v_{k−1}]``.  Implemented recursively: the
   ``len(active_vars) == 1`` base case calls the existing
   ``_u_diophantine`` over ``Q[v_0]``; the recursive case eliminates
   the trailing variable by specialisation + lift, again recursively.
7. Verify the final product equals the input; otherwise fall through.

New cases correctly handled (previously returned unevaluated):

* `factor(x² − y² − z² − 2yz) → (x + y + z)(x − y − z)`
* `factor((x + y + z)(x + 2y + 3z))` round-trips
* `factor(x³ + y³ + z³ − 3xyz) → (x + y + z)(x² + y² + z² − xy − yz − xz)`
* `factor((x + y)(x + z + w))` — four variables, two lift iterations
* `factor(x² + y² + z² + 1)` → confirmed irreducible (returns `None`)

Bounded-resource guarantees
~~~~~~~~~~~~~~~~~~~~~~~~~~~

* Specialisation search: at most ``_MAX_N_SPECIALISATION = 10`` tuples
  per recursive level — adversarial input does not loop indefinitely.
* Recursion depth: bounded by ``num_vars`` (each level eliminates one
  variable).
* Lift loop: ``deg_{v_k}(f) + 1`` iterations per level (bounded by the
  input's polynomial structure — no memory bombs on
  ``Pow(x, 10⁹)``-shaped adversarial input because exponents are
  already explicit in the sparse representation).

Limitations (explicitly documented)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

* **Squarefree image required** at the lucky specialisation point.
  Polynomials whose univariate image at every small integer tuple has
  a repeated root return `None` from this path.
* **Rational coefficients only** — works in ℚ throughout via the
  exact-arithmetic ``Fraction``-based diophantine solver.

New tests: `tests/test_n_variate_hensel.py` — 13 cases covering the
trivariate acceptance criterion, a 4-variate iterated lift, the cubic
sum-of-cubes identity, irreducibility detection, single-variable
fall-through, ``num_vars < 2`` fall-through, constant input, the empty
polynomial, lone linear inputs, two bivariate-via-n-variate
regressions, and a bounded-resource check on a degree-4 irreducible.

Total test count: 154 (141 existing + 13 new).  Coverage: 89%
(86% for the updated `hensel.py` module).

This is the **Python** side of Track K1.  The TS+Rust port is Track K2
(separate PR).

## 0.4.0 — 2026-05-28

**Track D1 — Bivariate Hensel lifting in Q[x, y].**

Closes the bivariate-factoring gap in the MACSYMA finish plan
(`code/specs/macsyma-finish-plan.md`).  The acceptance criterion was
`factor(x² + xy − 2y²) → (x + 2y)(x − y)` and similar bivariate
polynomials with rational coefficients.

Adds `cas_factor/hensel.py` implementing the textbook bivariate Hensel
lift over ℚ:

1. **Lucky-substitution search** — try small integer values `y₀ ∈
   {0, ±1, ±2, …}` until the univariate image `f(x, y₀)` is squarefree
   and has full x-degree.
2. **Univariate factoring of the image** — clear denominators and reuse
   the existing `factor_integer_polynomial` pipeline (rational roots +
   Kronecker + BZH).
3. **Hensel lift in powers of `(y − y₀)`** — at each step solve the
   univariate Diophantine equation `u·g₀ + v·h₀ = e_k` in ℚ[x] via
   extended GCD, then add the corrections at the next `y`-layer.
4. **Multi-factor handling** — iteratively peel off one univariate
   factor against the product of the rest, routing every factor count
   through the same two-factor lift (per the spec's
   "no helper per polynomial shape" guardrail).
5. **Final verification** — multiply lifted factors and confirm the
   product equals the input before returning.

New cases correctly handled (previously returned unevaluated):

- `factor(x² + xy − 2y²) → (x + 2y)(x − y)`
- `factor(2x² + 3xy − 2y²)` → linear × linear over ℚ
- `factor(x³ − y³) → (x − y)(x² + xy + y²)`
- `factor(x² + y² + 1)` → confirmed irreducible (returns `None`)
- Univariate inputs (`x² − 1`) fall through cleanly to the existing
  univariate path.

Updates `symbolic_vm/cas_handlers.py`:

- New helpers `_find_two_variables`, `_ir_to_bipoly`, `_bipoly_to_ir`,
  `_try_bivariate_hensel_ir` that bridge the IR layer to the new
  bivariate algorithm.
- `factor_handler` dispatcher gains the bivariate Hensel branch after
  the existing multivariate pattern recognisers (perfect-square,
  difference-of-squares, cubic-identity, perfect-cube, grouping,
  common-factor extraction) and before returning unevaluated.

Limitations (explicitly documented):

- **Bivariate only** — trivariate+ falls through.  The spec calls
  three-variable factoring a non-goal.
- **Rational coefficients only** — works directly in ℚ via the
  diophantine-solver approach rather than mod-p + CRT, simpler and
  sufficient for graduate-engineering inputs.
- **Squarefree image required** — polynomials whose univariate image
  at every small `y₀` has a repeated root (e.g. `(x − y)²(x + y)`)
  return `None` from the bivariate path.  Squarefree decomposition is
  out of scope for this PR.

New tests: `tests/test_hensel.py` — 6 cases covering the acceptance
criterion, non-monic leading coefficient, multi-factor lift,
irreducibility detection, linear input (fall-through), and the
univariate fall-through.

Total test count: 141 (135 existing + 6 new).  Coverage: 90%
(88% for the new `hensel.py` module).

This is the **Python** side of Track D1.  The TS+Rust port is Track D2
(separate PR).

## 0.3.0 — 2026-04-28

**Phase 3 — Berlekamp-Zassenhaus-Hensel (BZH) for arbitrary-degree factoring.**

Adds `cas_factor/bzh.py` implementing the full BZH pipeline for monic primitive
polynomials over Z:

1. **Prime selection** — find the smallest prime p < 200 such that `f mod p` is
   squarefree (via `gcd(f mod p, f' mod p) = 1` in GF(p)).
2. **Berlekamp mod p** — build the Frobenius Q-matrix, compute the null space of
   `(Q − I)` over GF(p) via Gaussian elimination, then split using `gcd(f, v − s)`.
3. **Hensel lifting** — linear Newton lift from mod p to mod p^k where
   `p^k > 2 * Mignotte_bound(f)`. Multi-factor splitting uses divide-and-conquer.
4. **Zassenhaus recombination** — try all subsets of lifted factors and test exact
   divisibility in Z[x].

Updates `factor.py`:
- `_factor_residual` now calls `bzh_factor` as a fallback when Kronecker returns
  `None` and the residual is monic of degree ≥ 4. Both algorithms are tried before
  declaring a polynomial irreducible.

New cases correctly handled (previously returned unevaluated):
- `x^5 − 1 = (x−1)(x^4+x^3+x^2+x+1)` — cyclotomic Φ_5
- `x^8 − 1 = (x−1)(x+1)(x^2+1)(x^4+1)` — full factorization
- `x^6 − 1` and `x^9 − 1` — iterated cyclotomic
- `x^4 + 1` → confirmed irreducible over Q

Limitations (explicitly documented):
- Restricted to **monic** polynomials; non-monic falls through to Kronecker.
- Degree cap MAX_DEGREE = 20.

New tests: `tests/test_bzh.py` — 74 new tests covering GF(p) arithmetic,
Berlekamp, Hensel lifting, factor combination, public API, edge cases, and
integrated pipeline tests.

Total test count: 135 (61 existing + 74 new). Coverage: 90%.

## 0.2.0 — 2026-04-27

**Phase 2 — Kronecker's algorithm for non-linear irreducible factors.**

Adds `cas_factor/kronecker.py` with:
- `_eval_points(n)` — generates the sequence 0, 1, −1, 2, −2, … .
- `_signed_divisors(n)` — all ±d for each positive divisor of |n|.
- `_lagrange_interpolate(xs, ys)` — exact Lagrange interpolation returning
  `list[Fraction]`; used to reconstruct candidate factor polynomials.
- `_poly_divmod_frac` / `_divides_exactly` — polynomial long division over
  Q; tests whether a candidate factor divides the target exactly with
  integer-coefficient quotient.
- `kronecker_factor(p)` — finds a non-trivial factor of degree
  1 ≤ k ≤ ⌊deg(p)/2⌋ by enumerating divisor combinations and
  Lagrange-interpolating.  Returns `(factor, cofactor)` or `None`.
  Safety cap: at most `_MAX_COMBOS = 10_000` combinations per degree trial.

Updates `factor.py`:
- `_factor_residual(residual)` — recursive work-queue that calls
  `kronecker_factor` and accumulates `(poly, multiplicity)` pairs.
- `factor_integer_polynomial` now calls `_factor_residual` instead of
  appending the residual as a single opaque factor.

Now handles cases not covered by Phase 1:
- **Sophie Germain identity**: `x^4 + 4 = (x^2+2x+2)(x^2−2x+2)`.
- **Cyclotomic identity**: `x^4+x^2+1 = (x^2+x+1)(x^2−x+1)`.
- **Repeated irreducibles**: `x^4+2x^2+1 = (x^2+1)^2`.
- **Mixed**: `(x^2+1)(x−2)` splits correctly.

Exports `kronecker_factor` from the package `__init__`.

New tests:
- `tests/test_kronecker.py` — 22 unit tests covering helpers and the
  full algorithm.
- `tests/test_factor.py` — 7 new integration tests (Sophie Germain,
  cyclotomic, repeated quadratic, mixed, `x^6−1`, irreducible regression).

## 0.1.0 — 2026-04-25

Initial release — Phase 1.

- ``content`` extraction (integer GCD of coefficients).
- Rational-root test for linear factors over Q.
- ``factor_integer_polynomial(coeffs)`` — orchestrates content +
  rational-root iteration; returns
  ``(content, [(factor_coeffs, multiplicity), ...])``.
- ``FACTOR``, ``IRREDUCIBLE`` head sentinels.

Deferred to Phase 2: Berlekamp factorization mod p, Hensel lifting,
Zassenhaus recombination.
