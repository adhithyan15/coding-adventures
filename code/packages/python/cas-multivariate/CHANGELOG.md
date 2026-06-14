# Changelog

## [0.1.0] - 2026-04-27

### Added

- `MPoly` class: sparse multivariate polynomial over Q (Fraction coefficients)
- Monomial orderings: `grlex`, `lex`, `grevlex` with `monomial_key()`
- `s_poly(f, g)`: S-polynomial computation for Buchberger
- `reduce_poly(f, G)`: multivariate polynomial reduction (normal form)
- `buchberger(F)`: Buchberger's algorithm with inter-reduction
- `GrobnerError`: raised when safety limits (degree > 8, basis > 50) exceeded
- `ideal_solve(polys)`: lex Gröbner basis + back-substitution solver
- `groebner_handler`, `poly_reduce_handler`, `ideal_solve_handler`: VM handlers
- `build_multivariate_handler_table()`: returns handler dict for VM integration
- IR heads `GROEBNER`, `POLY_REDUCE`, `IDEAL_SOLVE` (local to this package)
- Full IR ↔ MPoly conversion (`_ir_to_mpoly`, `_mpoly_to_ir`)
