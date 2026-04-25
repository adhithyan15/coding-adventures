"""First-class symbolic matrices for the symbolic IR.

Quick start::

    from cas_matrix import matrix, determinant, transpose, dot
    from symbolic_ir import IRInteger, IRSymbol

    M = matrix([[IRInteger(1), IRInteger(2)],
                [IRInteger(3), IRInteger(4)]])
    determinant(M)  # un-simplified IR; pass through cas_simplify to reduce
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
from cas_matrix.heads import (
    DETERMINANT,
    DIMENSIONS,
    DOT,
    IDENTITY_MATRIX,
    INVERSE,
    LIST,
    MATRIX,
    TRACE,
    TRANSPOSE,
    ZERO_MATRIX,
)
from cas_matrix.matrix import (
    MatrixError,
    dimensions,
    get_entry,
    is_matrix,
    matrix,
    num_cols,
    num_rows,
)

__all__ = [
    "DETERMINANT",
    "DIMENSIONS",
    "DOT",
    "IDENTITY_MATRIX",
    "INVERSE",
    "LIST",
    "MATRIX",
    "MatrixError",
    "TRACE",
    "TRANSPOSE",
    "ZERO_MATRIX",
    "add_matrices",
    "determinant",
    "dimensions",
    "dot",
    "get_entry",
    "identity_matrix",
    "inverse",
    "is_matrix",
    "matrix",
    "num_cols",
    "num_rows",
    "scalar_multiply",
    "sub_matrices",
    "trace",
    "transpose",
    "zero_matrix",
]
