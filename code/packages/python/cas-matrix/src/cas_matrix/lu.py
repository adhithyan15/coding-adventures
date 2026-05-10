"""LU decomposition with partial pivoting (Doolittle algorithm).

Given an n×n matrix A, LU decomposition finds:

    P · A = L · U

where:

- **P** is a permutation matrix (rows of the identity, reordered).
- **L** is lower-triangular with 1s on the main diagonal.
- **U** is upper-triangular.

Any non-singular square matrix has an LU decomposition (possibly requiring
row exchanges, hence P).

Why partial pivoting?
---------------------
Without pivoting, the algorithm fails on matrices with a zero in the
pivot position even though the matrix itself is non-singular (e.g.
[[0, 1], [1, 0]]).  Partial pivoting selects the row with the largest
absolute value in the current column as the pivot, which both avoids
division-by-zero and improves numerical stability.

Since we work with exact Fraction arithmetic, "stability" is not strictly
required — but pivoting avoids degenerate cases where an intermediate U[k,k]
is 0 while the overall matrix is invertible.

Doolittle algorithm
-------------------
Initialise:

    P = identity, L = identity, U = copy of A (as Fractions)

For each column k (k = 0 … n-1):

  1. **Pivot search**: find row p ≥ k with the largest |U[p, k]|.
  2. **Row swap**: swap rows k and p in U and P; also swap the already-
     filled part of L (columns 0 … k-1) in rows k and p.
  3. **Doolittle step**: for each row i > k:
        factor = U[i,k] / U[k,k]
        L[i,k] = factor
        U[i,:] -= factor * U[k,:]

After the loop, U has been reduced to upper-triangular form, L contains
the multipliers, and P records all row swaps.

Correctness check: P @ A == L @ U (entry-by-entry equality over Q).

Singular matrices
-----------------
If U[k,k] == 0 even after pivoting, the matrix is singular.  A
``MatrixError`` is raised.

Literate reading order
-----------------------
1. ``_identity_fractions``  — build an n×n identity as Fractions
2. ``_fracs_to_matrix``     — convert Fraction matrix to IR Matrix
3. ``lu_decompose``         — the main LU algorithm
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import IRApply, IRNode, IRSymbol

from cas_matrix.matrix import MatrixError, matrix, num_cols, num_rows
from cas_matrix.rowreduce import _frac_to_ir, _matrix_to_fractions

# ---------------------------------------------------------------------------
# Section 1 — Internal helpers
# ---------------------------------------------------------------------------


def _identity_fractions(n: int) -> list[list[Fraction]]:
    """Return an n×n identity matrix as a list-of-lists of Fraction.

    Parameters
    ----------
    n:
        Matrix dimension.

    Returns
    -------
    ``I[i][j] = Fraction(1)`` if i==j else ``Fraction(0)``.
    """
    return [
        [Fraction(1) if i == j else Fraction(0) for j in range(n)]
        for i in range(n)
    ]


def _fracs_to_matrix(frows: list[list[Fraction]]) -> IRApply:
    """Convert a list-of-lists of Fraction to a Matrix IR node.

    Parameters
    ----------
    frows:
        Row-major matrix; each entry is a ``Fraction``.

    Returns
    -------
    ``Matrix`` IR node with ``IRInteger`` / ``IRRational`` entries.
    """
    return matrix([[_frac_to_ir(f) for f in row] for row in frows])


# ---------------------------------------------------------------------------
# Section 2 — LU decomposition
# ---------------------------------------------------------------------------


def lu_decompose(M: IRNode) -> IRApply:
    """Compute the LU decomposition of a square rational matrix.

    Returns a ``List(L, U, P)`` IR node where:

    - ``L`` is lower-triangular with 1s on the diagonal.
    - ``U`` is upper-triangular.
    - ``P`` is a permutation matrix.
    - ``P · A = L · U`` (exact over Q).

    Parameters
    ----------
    M:
        Square matrix with ``IRInteger`` / ``IRRational`` entries.

    Returns
    -------
    ``IRApply(List, (L, U, P))`` — three ``Matrix`` IR nodes.

    Raises
    ------
    MatrixError
        If M is not square, has symbolic entries, or is singular
        (zero pivot after partial pivoting).

    Examples
    --------
    ::

        A = matrix([[IRInteger(2), IRInteger(1)],
                    [IRInteger(1), IRInteger(3)]])
        L, U, P = lu_decompose(A).args
        # L = [[1, 0], [1/2, 1]]
        # U = [[2, 1], [0, 5/2]]
        # P = [[1, 0], [0, 1]]  (no row swap needed)

        A2 = matrix([[IRInteger(0), IRInteger(1)],
                     [IRInteger(1), IRInteger(0)]])
        # Requires pivoting: P swaps rows
        L2, U2, P2 = lu_decompose(A2).args
        # P2 = [[0,1],[1,0]]
        # L2 = [[1,0],[0,1]]
        # U2 = [[1,0],[0,1]]
    """
    n = num_rows(M)
    if n != num_cols(M):
        raise MatrixError(f"lu_decompose: matrix must be square, got {n}×{num_cols(M)}")

    U = _matrix_to_fractions(M)   # will become upper-triangular
    L = _identity_fractions(n)    # will accumulate multipliers
    P = _identity_fractions(n)    # permutation matrix

    for k in range(n):
        # Step 1: partial pivoting — find row p >= k with max |U[p, k]|
        best_row = k
        best_val = abs(U[k][k])
        for r in range(k + 1, n):
            if abs(U[r][k]) > best_val:
                best_val = abs(U[r][k])
                best_row = r

        if best_row != k:
            # Swap rows k and best_row in U and P.
            U[k], U[best_row] = U[best_row], U[k]
            P[k], P[best_row] = P[best_row], P[k]
            # Swap the already-filled part of L (columns 0 … k-1).
            for c in range(k):
                L[k][c], L[best_row][c] = L[best_row][c], L[k][c]

        pivot = U[k][k]
        if pivot == 0:
            raise MatrixError(
                f"lu_decompose: singular matrix (zero pivot at column {k})"
            )

        # Step 2: eliminate entries below the pivot (Doolittle step).
        for i in range(k + 1, n):
            factor = U[i][k] / pivot
            L[i][k] = factor
            for c in range(k, n):
                U[i][c] -= factor * U[k][c]

    LIST_HEAD = IRSymbol("List")
    return IRApply(
        LIST_HEAD,
        (_fracs_to_matrix(L), _fracs_to_matrix(U), _fracs_to_matrix(P)),
    )
