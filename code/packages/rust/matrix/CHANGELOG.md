# Changelog

All notable changes to the Rust matrix package will be documented here.

## [0.3.0] - 2026-09-10

### Added
- **Linear solve (VIS01):** `solve(b)`, `invert()`, `determinant()` -- Gaussian
  elimination with partial pivoting for square `f64` systems. Closes the gap
  `code/specs/VIS00-vision-roadmap.md` identified directly: no reusable dense
  numeric solve existed anywhere in the workspace (`blas-library` has no
  exposed solver; `cas-matrix`/`cas-solve` have the right algorithms but
  operate on the symbolic computer-algebra tree type, not plain `f64`).
  Scoped to the small dense systems `VIS03` (geometric transform estimation,
  not yet built) will need -- an 8-unknown homography solve, smaller for
  affine.
- 28 new tests (`tests/matrix_tests.rs`): known-answer round trips, an
  `invert` round trip at the 8x8 size the scoped use case needs, determinant
  cross-checked against `solve`/`invert` agreeing on singularity, singular
  detection (zero row, identical rows, proportional rows), a partial-pivoting
  accuracy case (a tiny-but-nonzero pivot that would catastrophically amplify
  floating-point error without row swapping), and NaN/Inf/non-square/
  dimension-mismatch inputs proven not to panic.
- 3 new doc-tests (`solve`, `invert`, `determinant`).

### Security hardening (pre-merge review)
- Every pivot-selection comparison originally used `partial_cmp(...).unwrap()`,
  which panics if the matrix contains `NaN` (`partial_cmp` returns `None` for
  any comparison involving `NaN`). Fixed to `total_cmp`, which provides a
  total ordering (including `NaN`) and never panics -- proven directly by
  `solve_does_not_panic_on_nan_or_inf`/`determinant_does_not_panic_on_nan_or_inf`,
  not merely assumed from the fix.

## [0.2.0] - 2026-04-04

### Added
- **Element access:** `get(row, col)`, `set(row, col, value)` with Result error handling
- **Reductions:** `sum()`, `sum_rows()`, `sum_cols()`, `mean()`, `min_val()`, `max_val()`, `argmin()`, `argmax()`
- **Element-wise math:** `map(fn)`, `sqrt()`, `abs_val()`, `pow_val(exp)` for applying functions to every element
- **Shape operations:** `flatten()`, `reshape(rows, cols)`, `row(i)`, `col(j)`, `slice(r0, r1, c0, c1)` with Result types
- **Equality:** `equals(other)` for exact comparison, `close(other, tolerance)` for approximate comparison
- **Factory methods:** `Matrix::identity(n)`, `Matrix::from_diagonal(values)`
- 35 unit tests plus 1 doc-test covering all new and existing operations

## [0.1.0] - 2026-04-03

### Added
- Initial implementation with `zeros`, `new_2d`, `new_1d`, `new_scalar`
- Arithmetic: `add`, `add_scalar`, `subtract`, `scale`
- `transpose` and `dot` with Result-based error handling
