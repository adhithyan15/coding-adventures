# Changelog

All notable changes to the C `matrix` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `matrix` crate — an immutable 2D matrix
  of `double` over a single row-major heap block.
- Constructors (`mat_new`, `mat_new_1d`, `mat_new_scalar`, `mat_zeros`,
  `mat_identity`, `mat_from_diagonal`, `mat_clone`); arithmetic (`mat_add`,
  `mat_subtract`, `mat_add_scalar`, `mat_scale`, `mat_transpose`, `mat_dot`);
  element access (`mat_get`, immutable `mat_set`); reductions (`mat_sum`,
  `mat_mean`, `mat_min_val`, `mat_max_val`, `mat_sum_rows`, `mat_sum_cols`,
  `mat_argmin`, `mat_argmax`); element-wise math (`mat_map`, `mat_sqrt`,
  `mat_abs`, `mat_pow`); shape ops (`mat_flatten`, `mat_reshape`, `mat_row`,
  `mat_col`, `mat_slice`); and comparison (`mat_equals`, `mat_close`).
- `MatStatus` status-code API (`MAT_OK`, `MAT_ERR_DIM`, `MAT_ERR_BOUNDS`,
  `MAT_ERR_ALLOC`) in place of the Rust `Result<Matrix, _>`; producers write
  their result through an out-parameter and leave it empty on error.
- `sqrt`, `abs`, and a general `pow` computed without `<math.h>`; the integer
  exponent path is exact, the fractional path uses `exp(y·ln x)`.
- Overflow-guarded allocation: every `rows*cols` element count is checked
  against `SIZE_MAX` before allocating, and `mat_free` is idempotent.
- 122 checks against the Rust crate's own reference values, run under every
  available C compiler via the shared `iso-harness`; the suite also passes
  clean under AddressSanitizer + UndefinedBehaviorSanitizer.
