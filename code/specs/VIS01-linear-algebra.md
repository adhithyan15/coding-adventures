# VIS01 — Dense Linear Solve

## Status

First package in the `VIS00-vision-roadmap.md` L2 build order. Adds
`solve`/`invert`/`determinant` directly to the existing `matrix` crate
rather than introducing a second dense-matrix type — `matrix::Matrix`
is already this workspace's f64 2D matrix (arithmetic, reductions,
shape operations, no external dependencies), and `VIS00` §"L2" already
named the actual gap precisely: not "no matrix type," but "no
`solve`/`invert` of any kind" on the one that exists. This spec closes
exactly that gap, as an amendment to `matrix`, not a new crate.

## 1. Why this exists

A homography — the transform `VIS03` (geometric estimation, not yet
built) will use to straighten a photographed QR code, or any other
rectangle, back into a square — is the solution to a small (8-unknown,
for four point correspondences) linear system `Ax = b`. `VIS00`
verified directly against source that no reusable version of that
solve exists anywhere in this workspace:

- `matrix` has arithmetic, reductions, and shape operations, but no
  `solve`/`invert`/`determinant` of any kind.
- `blas-library` has matrix multiply (SGEMM/SGEMV/SGER, 7 backends)
  but no exposed solver — a comment in its own CPU backend only
  discusses pivoting internally for GEMM numerical stability, not a
  public solve API.
- `cas-matrix`/`cas-solve` *do* have the real algorithms (LU
  decomposition with partial pivoting, exact Gauss–Jordan row
  reduction, a working `solve_linear_system`) — but operate on the
  symbolic computer-algebra `IRNode` tree type (arbitrary-precision
  rationals as tree nodes), not plain `f64`. Correct math,
  architecturally the wrong layer for a small numeric per-call solve:
  every arithmetic operation allocates a tree node and every result
  needs converting back to a float, for a workload that is entirely
  small dense real matrices.

`matrix::Matrix` is the numeric sibling that was always missing its
own `solve`.

## 2. Scope

Small, dense, real (`f64`) systems only — the sizes geometric
estimation actually produces (an 8×8 system for a homography DLT
solve; smaller for affine estimation). Not a general sparse-matrix or
large-scale numerical-linear-algebra library; not iterative methods
(Conjugate Gradient, GMRES); not a full LAPACK-equivalent
decomposition suite (no QR, no SVD, no eigendecomposition — `VIS00`
does not need them, and `cas-matrix` already has an eigenvalues module
for the symbolic case if a future consumer needs the numeric
equivalent, which is its own separate scope, not this one).

## 3. Algorithm

**Gaussian elimination with partial pivoting**, the standard,
numerically-stable classical method for a system this size — a direct
method (exact up to floating-point rounding), not an approximation,
appropriate because these systems are small and dense, not because
speed at scale matters here.

```text
solve(A, b):
  augment [A | b] into one (n × n+1) working matrix
  for each pivot column k in 0..n:
    find the row i >= k with the largest |value| in column k
      (partial pivoting: reduces numerical error from small pivots,
       standard practice, not optional for a general-purpose solver)
    if that largest magnitude is below a near-zero epsilon:
      return Err(Singular) — no partial-pivoting swap can rescue a
      system whose remaining rows are all (numerically) zero in this
      column; the system has no unique solution
    swap row i and row k
    for each row r below k:
      eliminate column k from row r using row k as the pivot row
  back-substitute from the last row to the first to recover x
  return Ok(x)
```

`invert(A)` reuses `solve` — call it once per column of the identity
matrix (`n` solves for an `n×n` inverse), which is simple, correct,
and appropriately cheap at the `VIS00`-scoped sizes (≤ roughly 10×10);
a fused Gauss–Jordan augmented-with-identity pass is the standard
optimization at larger scale and is explicitly not needed here.

`determinant(A)` is a near-free byproduct of the same elimination:
the product of the pivots actually used, times `-1` for each row swap
performed (the sign flip a row swap introduces to the determinant is
standard linear algebra, not specific to this implementation) — this
also means `determinant` can share the same elimination pass `solve`
already performs rather than being a third independent algorithm.

## 4. API

Added to `matrix::Matrix`, matching the crate's existing conventions
exactly: `Result<T, String>` for fallible operations (the style
`get`/`set`/`reshape`/`row`/`col`/`slice` already use, not the older
`Result<T, &'static str>` a few of the original arithmetic methods
use — `String` is necessary here since a useful error names the
specific row/dimension involved, which `&'static str` cannot), and
"returns a new value, never panics on bad input" (the crate's own
stated design principle 3).

```rust
impl Matrix {
    /// Solve `self * x = b` for `x`, where `self` is n×n and `b` is a
    /// length-n column vector (as a 1-column Matrix or a plain `&[f64]`
    /// — see §4.1 on which). Gaussian elimination with partial pivoting.
    pub fn solve(&self, b: &[f64]) -> Result<Vec<f64>, String>;

    /// The inverse of `self`, an n×n matrix. `self.dot(&self.invert()?)`
    /// is the identity up to floating-point rounding.
    pub fn invert(&self) -> Result<Matrix, String>;

    /// The determinant of `self`, an n×n matrix.
    pub fn determinant(&self) -> Result<f64, String>;
}
```

### 4.1 Why `&[f64]` for `b`, not another `Matrix`

`solve`'s right-hand side is conceptually a vector, and every
consumer this spec is written for (`VIS03`'s point-correspondence
systems) already has its coefficients as a plain slice, not a
constructed `Matrix`. Accepting `&[f64]` avoids forcing a caller to
wrap a vector in a 1-column `Matrix` just to hand it to `solve` and
then unwrap a 1-column `Matrix` result back out again — `solve`
returns `Vec<f64>` for the same reason. `invert`/`determinant` operate
on whole matrices and correctly return `Matrix`/`f64` respectively.

### 4.2 Errors

One error surface, `Err(String)`, matching the crate's existing style
(not a new enum type this one crate would be alone in introducing) —
every message names what specifically went wrong:

- **Non-square input** — `solve`/`invert`/`determinant` all require
  `self.rows == self.cols`; a non-square matrix is a caller bug, not
  a runtime numerical condition, and is reported as clearly as
  `dot`'s existing "inner dimensions strictly contradict" message.
- **Dimension mismatch** — `solve`'s `b.len() != self.rows`.
- **Singular (or numerically indistinguishable from singular)** — the
  largest available pivot magnitude at some elimination step is below
  a fixed epsilon. Named explicitly ("matrix is singular" or
  equivalent), not conflated with the dimension-mismatch message —
  the two are different classes of caller-facing problem: one is "you
  called this wrong," the other is "this system has no unique
  solution regardless of how you called it."

No panics on any input shape or content — including intentionally
degenerate matrices (all-zero, non-square, containing `NaN`/`inf`) —
matching test coverage proves this directly (§6), not just by
inspection.

## 5. What this does not decide

- `VIS03`'s own homography/affine estimation design (how point
  correspondences become the `A`/`b` this spec's `solve` consumes) —
  separate future spec, this one only provides the primitive it will
  call.
- Whether a future consumer needs QR/SVD/eigendecomposition on real
  matrices — not needed by anything scoped in `VIS00` today; if it
  ever is, that is new scope for a later amendment or a sibling
  package, not assumed here.

## 6. Test strategy

- **Known-answer round trips**: `solve` against systems with a
  hand-computable exact answer (identity matrix, diagonal matrix,
  a small hand-verified 3×3 system), and the general property
  `A.dot(&Matrix::from_column(A.solve(&b)?)) ≈ b` (within floating-
  point tolerance) for larger random-ish well-conditioned systems.
- **`invert` round trip**: `A.dot(&A.invert()?)` is the identity
  (within tolerance), for several sizes up to the ~8×8/10×10 the
  scoped use case needs.
- **`determinant`**: known values for identity (`1`), a diagonal
  matrix (product of the diagonal), and a matrix with two identical
  rows (`0`, and — since a zero determinant implies singularity —
  cross-checked against `solve`/`invert` on that same matrix both
  returning the singular error, not silently returning a
  wrong-but-plausible answer).
- **Singular detection**: an exactly-singular matrix (a zero row; two
  identical rows; two proportional rows) is rejected by `solve`,
  `invert`, and `determinant` alike, not silently producing `inf`/
  `NaN` or a division-by-near-zero-amplified garbage answer.
- **Partial pivoting actually matters**: a system that would produce a
  numerically poor answer without row swapping (a tiny-but-nonzero
  pivot with a much larger value available lower in the same column) —
  verify the *with-pivoting* answer is close to the known correct
  answer, not simply that the function returns `Ok` at all. This is
  the one test in the suite that would still pass with partial
  pivoting silently disabled, so it must specifically assert accuracy,
  not just success.
- **Non-square / dimension-mismatch rejection** for all three
  functions, with the specific error class distinguished from
  "singular" per §4.2.
- **No panics** on adversarial shapes (`0×0`, non-square, a matrix
  containing `NaN`/`inf` values) — every case above already proves
  `Result` is returned, not a panic; this is the explicit "and I
  checked the degenerate shapes too" pass alongside them.

## 7. Acceptance gates

1. Every existing `matrix` crate test (unit + doc-tests) passes
   unchanged — this is a purely additive amendment.
2. §6's full test matrix is green.
3. No `.unwrap()`/`.expect()`/panicking index reachable from any
   `solve`/`invert`/`determinant` call regardless of input shape or
   content (checked directly, not only inferred from the tests
   passing).
4. README and CHANGELOG updated in the same style as the crate's
   existing `0.2.0` entry.
