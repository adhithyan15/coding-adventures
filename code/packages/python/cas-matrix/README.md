# cas-matrix

First-class symbolic matrices for the symbolic IR. Foundation for the
future Matlab/Octave runtime and for SPICE-on-symbolic-VM (the latter
needs Modified Nodal Analysis matrices that are symbolic at design
time and numeric at simulation time).

## Quick start

```python
from cas_matrix import matrix, determinant, transpose, dot
from symbolic_ir import IRInteger, IRSymbol

# 2x2 numeric matrix
M = matrix([[IRInteger(1), IRInteger(2)],
            [IRInteger(3), IRInteger(4)]])
determinant(M)
# IRApply(SUB, (Mul(1, 4), Mul(2, 3)))  — un-simplified expression
# (call cas_simplify.simplify on it to get IRInteger(-2))

# Symbolic 2x2: matrix([[a, b], [c, d]])
a, b, c, d = (IRSymbol(s) for s in "abcd")
S = matrix([[a, b], [c, d]])
determinant(S)
# IRApply(SUB, (Mul(a, d), Mul(b, c)))   — a*d - b*c
```

## Operations

| Function                     | Behavior                                    |
|------------------------------|---------------------------------------------|
| ``matrix(rows)``             | Build a Matrix from a Python list of rows.  |
| ``dimensions(M)``            | ``IRApply(LIST, (rows, cols))``.            |
| ``transpose(M)``             | Transpose.                                  |
| ``identity_matrix(n)``       | n-by-n identity.                            |
| ``zero_matrix(rows, cols)``  | All-zero matrix.                            |
| ``add_matrices(A, B)``       | Elementwise addition (shape-checked).       |
| ``sub_matrices(A, B)``       | Elementwise subtraction.                    |
| ``scalar_multiply(s, M)``    | Multiply every entry by scalar IR ``s``.    |
| ``dot(A, B)``                | Matrix product (cols(A) == rows(B)).        |
| ``trace(M)``                 | Sum of diagonal.                            |
| ``determinant(M)``           | Cofactor expansion. Returns un-simplified IR. |
| ``inverse(M)``               | Adjugate / determinant. Returns un-simplified IR. |

All entries are arbitrary IR. Users that want a numeric result run
the IR through ``cas_simplify.simplify`` (or any other downstream pass)
afterwards.

## Implementation notes

- **Determinant** uses cofactor (Laplace) expansion on the first row.
  This is O(n!) but produces clean symbolic expressions — the right
  choice for matrices with non-numeric entries. Bareiss is reserved
  for a future numeric-only fast path.
- **Inverse** uses adjugate / determinant. Same algorithm; same
  trade-off.
- Shape mismatches raise :class:`MatrixError` with a clear message.

## Reuse story

Foundation for:

- A future ``matlab-runtime`` / ``octave-runtime`` whose native data
  type is the matrix.
- The SPICE-on-symbolic-VM project, where MNA builds an ``Ax = b``
  system per timestep with mixed numeric and symbolic entries.

## Dependencies

- `coding-adventures-symbolic-ir`
