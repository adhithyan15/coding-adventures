"""BLAS Data Types — Matrix, Vector, and enumeration types.

=== What Lives Here ===

This module defines the core data types used throughout the BLAS library:

    1. StorageOrder  — how matrix elements are laid out in memory
    2. Transpose     — whether to logically transpose a matrix
    3. Side          — which side the special matrix is on (for SYMM)
    4. Vector        — a 1-D array of floats
    5. Matrix        — a 2-D array of floats stored as a flat list

=== Why Flat Storage? ===

GPUs need contiguous memory. A Python ``list[list[float]]`` (the 2D nested
list used by the existing ``matrix`` package) has each row allocated
separately in memory. A flat ``list[float]`` is one contiguous block — when
we upload it to GPU memory, it's a single memcpy.

    Nested (existing matrix package):
        data = [[1, 2, 3],
                [4, 5, 6]]
        # Each inner list is a separate Python object

    Flat (BLAS library):
        data = [1, 2, 3, 4, 5, 6]
        # One contiguous list. A[i][j] = data[i * cols + j]

=== Conversion Utilities ===

The ``from_matrix_pkg()`` and ``to_matrix_pkg()`` functions convert between
the two representations, so existing ML code (loss functions, gradient
descent) can work with BLAS results.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

# =========================================================================
# Enumerations — small types that control BLAS operation behavior
# =========================================================================


class StorageOrder(Enum):
    """How matrix elements are laid out in memory.

    ================================================================
    HOW MATRICES ARE STORED IN MEMORY
    ================================================================

    A 2x3 matrix:
        [ 1  2  3 ]
        [ 4  5  6 ]

    Row-major (C convention):    [1, 2, 3, 4, 5, 6]
        A[i][j] = data[i * cols + j]

    Column-major (Fortran/BLAS): [1, 4, 2, 5, 3, 6]
        A[i][j] = data[j * rows + i]

    We default to row-major because Python, C, and most ML frameworks
    use row-major. Traditional BLAS uses column-major (Fortran heritage).
    ================================================================
    """

    ROW_MAJOR = "row_major"
    COLUMN_MAJOR = "column_major"


class Transpose(Enum):
    """Transpose flags for GEMM and GEMV.

    ================================================================
    TRANSPOSE FLAGS FOR GEMM AND GEMV
    ================================================================

    When computing C = alpha * A * B + beta * C, you often want to use A^T or B^T
    without physically transposing the matrix. The Transpose flag
    tells the backend to "pretend" the matrix is transposed.

    This is a classic BLAS optimization: instead of allocating a new
    matrix and copying transposed data, you just change the access
    pattern. For a row-major matrix with shape (M, N):
      - NO_TRANS: access as (M, N), stride = N
      - TRANS:    access as (N, M), stride = M
    ================================================================
    """

    NO_TRANS = "no_trans"
    TRANS = "trans"


class Side(Enum):
    """Which side the special matrix is on (for SYMM, TRMM).

    ================================================================
    WHICH SIDE THE SPECIAL MATRIX IS ON (FOR SYMM, TRMM)
    ================================================================

    SYMM computes C = alpha * A * B + beta * C where A is symmetric.
    If Side.LEFT:  A is on the left  -> C = alpha * (A) * B + beta * C
    If Side.RIGHT: A is on the right -> C = alpha * B * (A) + beta * C
    ================================================================
    """

    LEFT = "left"
    RIGHT = "right"


# =========================================================================
# Vector — a 1-D array of single-precision floats
# =========================================================================


@dataclass
class Vector:
    """A 1-D array of single-precision floats.

    ================================================================
    A 1-D ARRAY OF SINGLE-PRECISION FLOATS
    ================================================================

    This is the simplest possible vector type. It holds:
    - data: a flat list of f32 values
    - size: how many elements

    It is NOT a tensor. It is NOT a GPU buffer. It lives on the host
    (CPU). Each backend copies it to the device when needed and copies
    results back. This keeps the interface dead simple.

    Example:
        v = Vector(data=[1.0, 2.0, 3.0], size=3)
        v.data[0]  # 1.0
        v.size     # 3
    ================================================================
    """

    data: list[float]
    size: int

    def __post_init__(self) -> None:
        """Validate that data length matches declared size.

        This catches bugs early — if you accidentally pass the wrong
        size, you get a clear error instead of a silent mismatch that
        causes wrong results deep in a BLAS operation.
        """
        if len(self.data) != self.size:
            raise ValueError(
                f"Vector data has {len(self.data)} elements but size={self.size}"
            )


# =========================================================================
# Matrix — a 2-D array of single-precision floats (flat storage)
# =========================================================================


@dataclass
class Matrix:
    """A 2-D array of single-precision floats stored as a flat list.

    ================================================================
    A 2-D ARRAY OF SINGLE-PRECISION FLOATS
    ================================================================

    Stored as a flat list in row-major order by default:

        Matrix(data=[1,2,3,4,5,6], rows=2, cols=3)

        represents:  [ 1  2  3 ]
                     [ 4  5  6 ]

        data[i * cols + j] = element at row i, column j

    The Matrix type is deliberately simple -- it's a container for
    moving data between the caller and the BLAS backend. The backend
    handles device memory management internally.
    ================================================================
    """

    data: list[float]
    rows: int
    cols: int
    order: StorageOrder = StorageOrder.ROW_MAJOR

    def __post_init__(self) -> None:
        """Validate that data length matches rows * cols.

        A 2x3 matrix must have exactly 6 elements. No more, no less.
        This validation catches shape mismatches before they cause
        cryptic errors in BLAS operations.
        """
        if len(self.data) != self.rows * self.cols:
            raise ValueError(
                f"Matrix data has {len(self.data)} elements "
                f"but shape is {self.rows}x{self.cols} = {self.rows * self.cols}"
            )


# =========================================================================
# Conversion utilities — bridge to the existing matrix package
# =========================================================================


def from_matrix_pkg(m: object) -> Matrix:
    """Convert an existing Matrix (2D nested list) to BLAS Matrix (flat).

    The existing ``matrix`` package stores data as ``list[list[float]]``.
    This function flattens it into the BLAS library's ``list[float]``
    format, row by row:

        Existing:  [[1, 2, 3], [4, 5, 6]]
        BLAS flat: [1, 2, 3, 4, 5, 6]

    Args:
        m: A matrix object with ``.data`` (list of lists), ``.rows``, and ``.cols``.

    Returns:
        A BLAS Matrix with the same data in flat row-major order.
    """
    flat = [m.data[i][j] for i in range(m.rows) for j in range(m.cols)]  # type: ignore[attr-defined]
    return Matrix(data=flat, rows=m.rows, cols=m.cols)  # type: ignore[attr-defined]


def to_matrix_pkg(m: Matrix) -> object:
    """Convert a BLAS Matrix (flat) to the existing Matrix (2D nested list).

    The reverse of ``from_matrix_pkg()``. Reshapes the flat data back
    into nested lists:

        BLAS flat: [1, 2, 3, 4, 5, 6]  (rows=2, cols=3)
        Existing:  [[1, 2, 3], [4, 5, 6]]

    Args:
        m: A BLAS Matrix.

    Returns:
        An existing-style Matrix with 2D nested list data.
    """
    from matrix import Matrix as MatrixPkg

    data_2d = [m.data[i * m.cols : (i + 1) * m.cols] for i in range(m.rows)]
    return MatrixPkg(data_2d)
