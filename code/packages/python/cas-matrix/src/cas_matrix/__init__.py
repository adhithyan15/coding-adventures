"""First-class symbolic matrices for the symbolic IR.

Quick start::

    from cas_matrix import matrix, determinant, transpose, dot, rank, row_reduce
    from symbolic_ir import IRInteger, IRSymbol

    M = matrix([[IRInteger(1), IRInteger(2)],
                [IRInteger(3), IRInteger(4)]])
    determinant(M)  # un-simplified IR; pass through cas_simplify to reduce
    rank(M)         # IRInteger(2)
    row_reduce(M)   # identity matrix (RREF)

Phase 19 additions::

    from cas_matrix import (
        eigenvalues, eigenvectors, charpoly,
        lu_decompose, nullspace, columnspace, rowspace, norm,
    )

    A = matrix([[IRInteger(1), IRInteger(2)],
                [IRInteger(2), IRInteger(1)]])
    eigenvalues(A)   # List(List(-1, 1), List(3, 1))
    eigenvectors(A)  # List(List(-1, 1, List(v1)), List(3, 1, List(v2)))
    lu_decompose(A)  # List(L, U, P) — Doolittle with partial pivoting
    nullspace(A)     # List() — A has full rank
"""

from cas_matrix.arithmetic import (
    add_matrices,
    dot,
    identity_matrix,
    scalar_multiply,
    sub_matrices,
    trace,
    transpose,
    zero_matrix,
)
from cas_matrix.determinant import determinant, inverse
from cas_matrix.eigenvalues import char_poly_coeffs, charpoly, eigenvalues, eigenvectors
from cas_matrix.heads import (
    CHARPOLY,
    COLUMNSPACE,
    DETERMINANT,
    DIMENSIONS,
    DOT,
    EIGENVALUES,
    EIGENVECTORS,
    IDENTITY_MATRIX,
    INVERSE,
    LIST,
    LU,
    MATRIX,
    NORM,
    NULLSPACE,
    RANK,
    ROW_REDUCE,
    ROWSPACE,
    TRACE,
    TRANSPOSE,
    ZERO_MATRIX,
)
from cas_matrix.lu import lu_decompose
from cas_matrix.matrix import (
    MatrixError,
    dimensions,
    get_entry,
    is_matrix,
    matrix,
    num_cols,
    num_rows,
)
from cas_matrix.norms import norm
from cas_matrix.rowreduce import rank, row_reduce
from cas_matrix.subspaces import columnspace, nullspace, rowspace

__all__ = [
    # Heads
    "CHARPOLY",
    "COLUMNSPACE",
    "DETERMINANT",
    "DIMENSIONS",
    "DOT",
    "EIGENVALUES",
    "EIGENVECTORS",
    "IDENTITY_MATRIX",
    "INVERSE",
    "LIST",
    "LU",
    "MATRIX",
    "NORM",
    "NULLSPACE",
    "RANK",
    "ROW_REDUCE",
    "ROWSPACE",
    "TRACE",
    "TRANSPOSE",
    "ZERO_MATRIX",
    # Errors
    "MatrixError",
    # Construction / query
    "dimensions",
    "get_entry",
    "identity_matrix",
    "is_matrix",
    "matrix",
    "num_cols",
    "num_rows",
    "zero_matrix",
    # Arithmetic
    "add_matrices",
    "dot",
    "scalar_multiply",
    "sub_matrices",
    "trace",
    "transpose",
    # Determinant / inverse
    "determinant",
    "inverse",
    # Row operations
    "rank",
    "row_reduce",
    # Phase 19 — eigenvalues / eigenvectors / charpoly
    "char_poly_coeffs",
    "charpoly",
    "eigenvalues",
    "eigenvectors",
    # Phase 19 — LU
    "lu_decompose",
    # Phase 19 — subspaces
    "columnspace",
    "nullspace",
    "rowspace",
    # Phase 19 — norm
    "norm",
]
