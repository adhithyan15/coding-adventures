# cas-matrix — Symbolic Matrix Type and Linear Algebra

> **Status**: New spec. Adds a first-class symbolic matrix to the IR
> plus the standard linear-algebra operations.
> Parent: `symbolic-computation.md`. Foundation for any future
> `matlab-runtime` / `octave-runtime` and for the SPICE-on-symbolic-VM
> work mentioned in the project vision.

## Why this package exists

Matrix arithmetic is the central operation of:

- Linear algebra (the obvious case).
- Circuit simulation (Modified Nodal Analysis builds an `Ax = b`
  system per timestep — entries are symbolic in the design phase,
  numeric in the simulation phase).
- Statistics, optimization, and most engineering domains a CAS gets
  used in.

This package introduces matrices as a distinct IR concept (not
just a list-of-lists) and gives them the operations users expect.

## Reuse story

This is the backbone for future Matlab/Octave/SPICE work, exactly as
called out in the project vision. The same matrix type powers:

- MACSYMA's `matrix(...)` and the `.` (matrix-product) operator.
- Mathematica's matrices (which are nested lists, but with operations
  that detect and dispatch on rectangular shape).
- A future `matlab-runtime`'s native matrix.
- A SPICE netlist evaluator's MNA matrix builder.

## Scope

In:

- A new IR concept `Matrix(rows...)` where each `rows` arg is a
  `List` of equal length. This is exposed as the `Matrix` head — no
  new IR primitive needed; the constraint that rows have equal length
  is enforced by handlers.
- Construction: `Matrix([1, 2], [3, 4])`.
- Element access: `Part(M, i, j)` (1-based).
- Shape: `Dimensions(M)` → `[rows, cols]`.
- Element-wise: `Plus`, `Subtract`, scalar-times-matrix `Times`.
- True matrix multiplication: `Dot(A, B)` (also written as `A . B` in
  MACSYMA).
- `Transpose(M)`.
- `Determinant(M)` — symbolic (Bareiss algorithm for exact arithmetic).
- `Inverse(M)` — symbolic (cofactor expansion for small, Bareiss
  elimination + back-substitution for larger).
- `IdentityMatrix(n)`.
- `ZeroMatrix(rows, cols)`.
- `Trace(M)`.
- `Rank(M)`.
- `RowReduce(M)` — reduced row echelon form.

Out (future):

- Eigenvalues / eigenvectors — `cas-eigen` package.
- LU / QR / SVD — separate package.
- Sparse matrix representation — separate package (essential for SPICE
  scale).
- Symbolic Jordan canonical form, exponentials of matrices, etc.

## Public interface

```python
from cas_matrix import register_handlers, dimensions, transpose

# (%i1) M : matrix([1, 2], [3, 4]);
# (%o1)                            [1  2]
#                                   [3  4]
# (%i2) determinant(M);
# (%o2)                            -2
# (%i3) M . M;
# (%o3)                            [7  10]
#                                   [15 22]
```

## Heads added

| Head             | Arity   | Meaning                                  |
|------------------|---------|------------------------------------------|
| `Matrix`         | 1+      | Matrix from row lists.                   |
| `Dimensions`     | 1       | `[rows, cols]`.                          |
| `Dot`            | 2+      | Matrix product.                          |
| `Transpose`      | 1       | Transpose.                               |
| `Determinant`    | 1       | Determinant.                             |
| `Inverse`        | 1       | Matrix inverse.                          |
| `IdentityMatrix` | 1       | Identity of size `n`.                    |
| `ZeroMatrix`     | 1–2     | Zero matrix.                             |
| `Trace`          | 1       | Sum of diagonal.                         |
| `Rank`           | 1       | Rank.                                    |
| `RowReduce`      | 1       | Reduced row-echelon form.                |

## Algorithm notes

- Matrix entries are **arbitrary IR**. Symbolic matrices are first-class.
- Determinant uses **Bareiss's algorithm** (fraction-free Gaussian
  elimination) over the entry ring. With `Fraction` entries this gives
  exact rational determinants without intermediate fraction blowup.
- Inverse uses Bareiss + back-substitution on the augmented matrix
  `[M | I]`.
- Elementwise ops verify shape compatibility; mismatch → error.
- `Dot` verifies `cols(A) == rows(B)`.

## Test strategy

- All operations on `2x2` and `3x3` integer matrices.
- Symbolic matrices: `Matrix([a, b], [c, d])` — determinant
  `a*d - b*c`.
- `Inverse(M) . M = IdentityMatrix(n)` (after simplify).
- Singular matrix: `Inverse([[1, 2], [2, 4]])` raises.
- Coverage: ≥90%.

## Package layout

```
code/packages/python/cas-matrix/
  src/cas_matrix/
    __init__.py
    construction.py
    elementwise.py
    dot.py
    transpose.py
    bareiss.py            # determinant + inverse
    identity_zero.py
    rank_reduce.py
    py.typed
  tests/
    test_construction.py
    test_dot.py
    test_determinant.py
    test_inverse.py
    test_symbolic.py
```

Dependencies: `coding-adventures-symbolic-ir`,
`coding-adventures-cas-list-operations` (for `Part` / `Length`).

## Future extensions

- Sparse representation (essential for SPICE — circuit matrices are
  ~99% zero).
- Eigenvalue algorithms (QR iteration, power method).
- Decompositions (LU, QR, Cholesky, SVD).
- Tensor extension to rank > 2.
- These all stay independent packages so you can pick what you need
  for a given application.
