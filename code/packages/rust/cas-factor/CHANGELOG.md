# Changelog — cas-factor (Rust)

## [0.3.0] — 2026-05-29

**Track K2 — n-variate Hensel lifting (Rust port).**

Ports the Python `cas_factor.hensel.try_n_variate_hensel` algorithm
(Track K1, PR #5590) to Rust.  Extends the bivariate Hensel lift to
n ≥ 3 variables via iterated bivariate lifting:

1. Pick a main variable `v_0`; substitute auxiliary variables with
   small integer values to reduce f to a univariate polynomial.
2. Factor the univariate image via the existing `factor_uni_q` chain.
3. Lift the univariate factors back one auxiliary variable at a time
   via Hensel-style expansion in powers of `(v_k − a_k)`.  Each lift
   step solves a coefficient-ring diophantine equation recursively;
   base case hits `u_diophantine` directly.
4. Verify the final product equals the input; return `None` on any
   mismatch so the caller falls through to other handlers.

Reuses the existing bivariate Hensel machinery (`Rat`, univariate
diophantine, `factor_uni_q`) as building blocks.  Bounded
specialisation search (≤ 10 tuples); recursion depth bounded by `n`.

### Added

- `try_n_variate_hensel(f: &NPoly, num_vars: usize) -> Option<Vec<NPoly>>`
  — top-level entry point for n-variate (n ≥ 2) factoring via
  iterated Hensel lifting.
- `NPoly` type — sparse `BTreeMap<Vec<usize>, Rat>` mapping exponent
  tuples to rational coefficients.
- `n_mul` re-exported from the crate root for tests.
- `tests/n_variate_hensel.rs` — 13 acceptance cases mirroring the
  Python `test_n_variate_hensel.py` suite: trivariate quadratic, two
  trivariate cubics (sum-of-cubes companion, asymmetric coefficients),
  quadrivariate iterated lift, six fall-through cases, two bivariate
  regressions via the n-variate front door, and a bounded-resource
  smoke test.

## [0.2.0] — 2026-05-28

**Track D2 — bivariate Hensel lifting (Rust port).**

Ports the Python `cas_factor.hensel` algorithm (Track D1, PR #4563) to
Rust.  `try_bivariate_hensel(f: &BiPoly) -> Option<Vec<BiPoly>>` factors
a bivariate polynomial in ℚ[x, y] by lucky-substitution, univariate
image factoring, and Hensel lift of each factor through the y-layers.
Multi-factor inputs are handled by iterated two-factor lift.

### Added

- New `hensel` module:
  - `BiPoly` — sparse `BTreeMap<(usize, usize), Rat>` mapping
    `(x_exp, y_exp)` to a rational coefficient.
  - `Rat` — exact rational with `i128` numerator/denominator for
    coefficient-growth headroom; `add`/`sub`/`mul`/`div`/`pow`/`neg`/
    `is_zero`.
  - `try_bivariate_hensel(f)` — top-level entry point.
  - Helpers re-exported from the crate root: `bi_mul`, `bi_degree_x`,
    `bi_degree_y`, `BiPoly`, `Rat`.
- `tests/hensel.rs` — 6 acceptance cases (5 Hensel cases + 1 univariate
  fall-through regression) mirroring the Python `test_hensel.py` suite.

### Unreleased (carried)

- `kronecker` module with public `kronecker_factor(p)` for primitive integer
  polynomial residual splitting.
- Recursive residual factoring in `factor_integer_polynomial`, covering
  Sophie Germain quartics, `x^4 + x^2 + 1`, and repeated quadratic residuals.
- Focused Kronecker and integration tests for residual factoring parity with
  the Python/TypeScript `cas-factor` implementations.

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `cas-factor` package.
- `polynomial` module:
  - `type Poly = Vec<i64>` — coefficient list, constant term first.
  - `normalize(p)` — strip trailing zeros.
  - `degree(p)` — polynomial degree (-1 for zero polynomial).
  - `content(p)` — GCD of all coefficients.
  - `primitive_part(p)` — divide by content.
  - `evaluate(p, x)` — Horner-rule evaluation at an integer.
  - `divide_linear(p, root)` — synthetic division by `(x - root)`.
  - `divisors(n)` — all positive integer divisors of `|n|`.
- `rational_roots` module:
  - `find_integer_roots(p)` — enumerate integer roots via the Rational Root Theorem.
  - `extract_linear_factors(p)` — fixed-point extraction of all linear factors with multiplicities.
- `factor` module:
  - `type FactorList = Vec<(Vec<i64>, usize)>`.
  - `factor_integer_polynomial(p)` — full factoring orchestrator: extracts content, finds primitive part, pulls linear factors, appends irreducible residual.
- `FACTOR` and `IRREDUCIBLE` head name constants for symbolic IR integration.
- No external dependencies (pure math, no symbolic-ir).
- 30 integration tests + 4 doc-tests; all passing.
