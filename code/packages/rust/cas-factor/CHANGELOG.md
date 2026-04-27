# Changelog — cas-factor (Rust)

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
