"""
Matrix — a pure-Python 2D matrix for linear algebra.

A matrix is a rectangular grid of numbers arranged in rows and columns.
Think of it like a spreadsheet: each cell sits at the intersection of a
row number and a column number.  Matrices are the fundamental building
block for linear algebra, which powers everything from graphics to
machine learning.

Design principles
-----------------
1. **Immutable by default.**  Every method returns a *new* Matrix; the
   original is never mutated.  This makes reasoning about data flow easy
   and keeps the door open for future parallelism.
2. **No external dependencies.**  Only the Python standard library's
   ``math`` module is used, so the package installs everywhere.
3. **Consistent error handling.**  Out-of-bounds indices, shape
   mismatches, and invalid reshape dimensions all raise descriptive
   ``ValueError`` or ``IndexError`` exceptions.
"""

from __future__ import annotations

import math
from typing import Callable, Tuple


class Matrix:
    """A 2D matrix of floating-point numbers.

    Internal storage
    ~~~~~~~~~~~~~~~~
    ``self.data`` is a list of lists — each inner list is one row.
    ``self.rows`` and ``self.cols`` cache the dimensions so we never
    have to recompute them.

    Construction
    ~~~~~~~~~~~~
    You can build a Matrix from:

    * A single number   -> 1x1 matrix
    * A flat list        -> 1-row matrix (row vector)
    * A list of lists    -> full 2D matrix

    >>> Matrix(5)
    Matrix([[5.0]])
    >>> Matrix([1, 2, 3])
    Matrix([[1.0, 2.0, 3.0]])
    >>> Matrix([[1, 2], [3, 4]])
    Matrix([[1.0, 2.0], [3.0, 4.0]])
    """

    # ------------------------------------------------------------------
    # Construction
    # ------------------------------------------------------------------

    def __init__(self, data):
        if isinstance(data, (int, float)):
            self.data = [[float(data)]]
            self.rows, self.cols = 1, 1
        elif isinstance(data, list) and len(data) > 0 and isinstance(data[0], (int, float)):
            self.data = [[float(x) for x in data]]
            self.rows, self.cols = 1, len(data)
        elif isinstance(data, list) and len(data) > 0 and isinstance(data[0], list):
            self.data = data
            self.rows = len(data)
            self.cols = len(data[0]) if self.rows > 0 else 0
        else:
            self.data = []
            self.rows, self.cols = 0, 0

    @classmethod
    def zeros(cls, rows: int, cols: int) -> "Matrix":
        """Create an ``rows x cols`` matrix filled with zeros.

        >>> Matrix.zeros(2, 3).data
        [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]
        """
        return cls([[0.0 for _ in range(cols)] for _ in range(rows)])

    # ------------------------------------------------------------------
    # Factory methods
    # ------------------------------------------------------------------

    @classmethod
    def identity(cls, n: int) -> "Matrix":
        """Create an ``n x n`` identity matrix.

        The identity matrix is the matrix equivalent of the number 1:
        multiplying any matrix by the identity leaves it unchanged.
        It has 1s on the main diagonal and 0s everywhere else.

        >>> Matrix.identity(3).data
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        """
        data = [[1.0 if i == j else 0.0 for j in range(n)] for i in range(n)]
        return cls(data)

    @classmethod
    def from_diagonal(cls, values: list) -> "Matrix":
        """Create a square diagonal matrix from a list of values.

        A diagonal matrix has non-zero entries only on the main diagonal.
        This is useful for creating scaling transforms — multiplying by a
        diagonal matrix scales each dimension independently.

        >>> Matrix.from_diagonal([2, 3]).data
        [[2.0, 0.0], [0.0, 3.0]]
        """
        n = len(values)
        data = [[float(values[i]) if i == j else 0.0 for j in range(n)] for i in range(n)]
        return cls(data)

    # ------------------------------------------------------------------
    # Arithmetic operators (existing)
    # ------------------------------------------------------------------

    def __add__(self, other):
        if isinstance(other, (int, float)):
            return Matrix([[self.data[i][j] + other for j in range(self.cols)] for i in range(self.rows)])
        if self.rows != other.rows or self.cols != other.cols:
            raise ValueError(f"Addition dimension mismatch: {self.rows}x{self.cols} vs {other.rows}x{other.cols}")
        return Matrix([[self.data[i][j] + other.data[i][j] for j in range(self.cols)] for i in range(self.rows)])

    def __sub__(self, other):
        if isinstance(other, (int, float)):
            return Matrix([[self.data[i][j] - other for j in range(self.cols)] for i in range(self.rows)])
        if self.rows != other.rows or self.cols != other.cols:
            raise ValueError("Subtraction dimension mismatch.")
        return Matrix([[self.data[i][j] - other.data[i][j] for j in range(self.cols)] for i in range(self.rows)])

    def __mul__(self, scalar: float):
        """Element-wise scalar multiplication mapped to the * operator"""
        return Matrix([[self.data[i][j] * scalar for j in range(self.cols)] for i in range(self.rows)])

    def dot(self, other: "Matrix") -> "Matrix":
        """Matrix dot product execution"""
        if self.cols != other.rows:
            raise ValueError(f"Dot product dimension mismatch: {self.cols} cols vs {other.rows} rows.")
        C = Matrix.zeros(self.rows, other.cols)
        for i in range(self.rows):
            for j in range(other.cols):
                for k in range(self.cols):
                    C.data[i][j] += self.data[i][k] * other.data[k][j]
        return C

    def transpose(self) -> "Matrix":
        return Matrix([[self.data[j][i] for j in range(self.rows)] for i in range(self.cols)])

    def __eq__(self, other):
        return isinstance(other, Matrix) and self.data == other.data

    # ------------------------------------------------------------------
    # Element access
    # ------------------------------------------------------------------
    # These two methods give you fine-grained control over individual
    # cells.  ``get`` reads a value; ``set`` returns a *new* matrix with
    # that one cell changed (remember: we never mutate).

    def get(self, row: int, col: int) -> float:
        """Return the element at ``(row, col)``.

        Raises ``IndexError`` if the indices are out of bounds, just
        like indexing a Python list.

        >>> Matrix([[1, 2], [3, 4]]).get(0, 1)
        2.0
        """
        if row < 0 or row >= self.rows or col < 0 or col >= self.cols:
            raise IndexError(f"Index ({row}, {col}) out of bounds for {self.rows}x{self.cols} matrix")
        return float(self.data[row][col])

    def set(self, row: int, col: int, value: float) -> "Matrix":
        """Return a new matrix with the element at ``(row, col)`` replaced.

        The original matrix is unchanged — this follows the immutable-
        by-default principle.

        >>> Matrix([[1, 2], [3, 4]]).set(0, 0, 99).data
        [[99.0, 2.0], [3.0, 4.0]]
        """
        if row < 0 or row >= self.rows or col < 0 or col >= self.cols:
            raise IndexError(f"Index ({row}, {col}) out of bounds for {self.rows}x{self.cols} matrix")
        new_data = [r[:] for r in self.data]
        new_data[row][col] = float(value)
        return Matrix(new_data)

    # ------------------------------------------------------------------
    # Reductions
    # ------------------------------------------------------------------
    # Reductions collapse a matrix (or parts of it) down to a single
    # number or a smaller matrix.  They answer questions like "what is
    # the total?" or "which row has the most?"

    def sum(self) -> float:
        """Sum of every element in the matrix.

        Walk through every cell and accumulate.  A 2x2 matrix
        ``[[1,2],[3,4]]`` sums to 10.

        >>> Matrix([[1, 2], [3, 4]]).sum()
        10.0
        """
        total = 0.0
        for row in self.data:
            for val in row:
                total += val
        return total

    def sum_rows(self) -> "Matrix":
        """Sum each row, returning an ``(rows x 1)`` column vector.

        Imagine collapsing every row into a single number by adding all
        its elements together.  The result is a tall, thin matrix with
        one column.

        >>> Matrix([[1, 2], [3, 4]]).sum_rows().data
        [[3.0], [7.0]]
        """
        return Matrix([[float(builtins_sum(row))] for row in self.data])

    def sum_cols(self) -> "Matrix":
        """Sum each column, returning a ``(1 x cols)`` row vector.

        Imagine collapsing every column downward.  The result is a wide,
        flat matrix with one row.

        >>> Matrix([[1, 2], [3, 4]]).sum_cols().data
        [[4.0, 6.0]]
        """
        sums = [0.0] * self.cols
        for row in self.data:
            for j, val in enumerate(row):
                sums[j] += val
        return Matrix([sums])

    def mean(self) -> float:
        """Arithmetic mean of every element.

        The mean is the sum divided by the count.  For ``[[1,2],[3,4]]``
        the sum is 10 and there are 4 elements, so the mean is 2.5.

        >>> Matrix([[1, 2], [3, 4]]).mean()
        2.5
        """
        n = self.rows * self.cols
        if n == 0:
            raise ValueError("Cannot compute mean of an empty matrix")
        return self.sum() / n

    def min(self) -> float:
        """Smallest element in the matrix.

        >>> Matrix([[3, 1], [4, 2]]).min()
        1.0
        """
        if self.rows == 0 or self.cols == 0:
            raise ValueError("Cannot compute min of an empty matrix")
        result = self.data[0][0]
        for row in self.data:
            for val in row:
                if val < result:
                    result = val
        return float(result)

    def max(self) -> float:
        """Largest element in the matrix.

        >>> Matrix([[3, 1], [4, 2]]).max()
        4.0
        """
        if self.rows == 0 or self.cols == 0:
            raise ValueError("Cannot compute max of an empty matrix")
        result = self.data[0][0]
        for row in self.data:
            for val in row:
                if val > result:
                    result = val
        return float(result)

    def argmin(self) -> Tuple[int, int]:
        """Position ``(row, col)`` of the smallest element.

        If the minimum value appears more than once, the *first*
        occurrence (scanning left-to-right, top-to-bottom) is returned.

        >>> Matrix([[3, 1], [4, 2]]).argmin()
        (0, 1)
        """
        if self.rows == 0 or self.cols == 0:
            raise ValueError("Cannot compute argmin of an empty matrix")
        best_val = self.data[0][0]
        best_r, best_c = 0, 0
        for i, row in enumerate(self.data):
            for j, val in enumerate(row):
                if val < best_val:
                    best_val = val
                    best_r, best_c = i, j
        return (best_r, best_c)

    def argmax(self) -> Tuple[int, int]:
        """Position ``(row, col)`` of the largest element.

        First occurrence wins on ties.

        >>> Matrix([[1, 2], [3, 4]]).argmax()
        (1, 1)
        """
        if self.rows == 0 or self.cols == 0:
            raise ValueError("Cannot compute argmax of an empty matrix")
        best_val = self.data[0][0]
        best_r, best_c = 0, 0
        for i, row in enumerate(self.data):
            for j, val in enumerate(row):
                if val > best_val:
                    best_val = val
                    best_r, best_c = i, j
        return (best_r, best_c)

    # ------------------------------------------------------------------
    # Element-wise math
    # ------------------------------------------------------------------
    # These methods apply a function to every element independently.
    # The shape stays the same; only the values change.

    def map(self, fn: Callable[[float], float]) -> "Matrix":
        """Apply ``fn`` to every element, returning a new matrix.

        This is the most general element-wise operation.  All of
        ``sqrt``, ``abs``, and ``pow`` are implemented on top of it.

        >>> Matrix([[1, 4], [9, 16]]).map(math.sqrt).data
        [[1.0, 2.0], [3.0, 4.0]]
        """
        return Matrix([[fn(self.data[i][j]) for j in range(self.cols)] for i in range(self.rows)])

    def sqrt(self) -> "Matrix":
        """Element-wise square root.

        Each cell ``x`` becomes ``sqrt(x)``.  Negative values will
        raise a ``ValueError`` from ``math.sqrt``.

        >>> Matrix([[1, 4], [9, 16]]).sqrt().data
        [[1.0, 2.0], [3.0, 4.0]]
        """
        return self.map(math.sqrt)

    def abs(self) -> "Matrix":
        """Element-wise absolute value.

        Flips negative numbers to positive; leaves positive numbers
        and zero unchanged.

        >>> Matrix([[-1, 2], [-3, 4]]).abs().data
        [[1.0, 2.0], [3.0, 4.0]]
        """
        return self.map(builtins_abs)

    def pow(self, exp: float) -> "Matrix":
        """Raise every element to the power ``exp``.

        ``M.pow(2)`` squares each element.  ``M.pow(0.5)`` is the same
        as ``M.sqrt()`` for non-negative values.

        >>> Matrix([[1, 2], [3, 4]]).pow(2).data
        [[1.0, 4.0], [9.0, 16.0]]
        """
        return self.map(lambda x: math.pow(x, exp))

    # ------------------------------------------------------------------
    # Shape operations
    # ------------------------------------------------------------------
    # Shape operations change the arrangement of elements without
    # altering their values.  Think of them as rearranging tiles on a
    # grid.

    def flatten(self) -> "Matrix":
        """Flatten to a ``1 x n`` row vector.

        All elements are placed into a single row, reading left-to-right
        and top-to-bottom (row-major order).

        >>> Matrix([[1, 2], [3, 4]]).flatten().data
        [[1.0, 2.0, 3.0, 4.0]]
        """
        flat = []
        for row in self.data:
            flat.extend(float(v) for v in row)
        return Matrix([flat])

    def reshape(self, rows: int, cols: int) -> "Matrix":
        """Reshape to ``rows x cols``.

        The total number of elements must stay the same — you cannot
        create or destroy values by reshaping.  Elements are filled in
        row-major order (left-to-right, top-to-bottom).

        >>> Matrix([[1, 2, 3, 4, 5, 6]]).reshape(2, 3).data
        [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]
        """
        total = self.rows * self.cols
        if rows * cols != total:
            raise ValueError(
                f"Cannot reshape {self.rows}x{self.cols} ({total} elements) "
                f"into {rows}x{cols} ({rows * cols} elements)"
            )
        flat = self.flatten().data[0]
        new_data = []
        for i in range(rows):
            new_data.append(flat[i * cols : (i + 1) * cols])
        return Matrix(new_data)

    def row(self, i: int) -> "Matrix":
        """Extract row ``i`` as a ``1 x cols`` matrix.

        >>> Matrix([[1, 2], [3, 4]]).row(0).data
        [[1.0, 2.0]]
        """
        if i < 0 or i >= self.rows:
            raise IndexError(f"Row {i} out of bounds for {self.rows}-row matrix")
        return Matrix([list(self.data[i])])

    def col(self, j: int) -> "Matrix":
        """Extract column ``j`` as a ``rows x 1`` matrix.

        >>> Matrix([[1, 2], [3, 4]]).col(0).data
        [[1.0], [3.0]]
        """
        if j < 0 or j >= self.cols:
            raise IndexError(f"Column {j} out of bounds for {self.cols}-column matrix")
        return Matrix([[self.data[i][j]] for i in range(self.rows)])

    def slice(self, r0: int, r1: int, c0: int, c1: int) -> "Matrix":
        """Extract a sub-matrix for rows ``[r0, r1)`` and cols ``[c0, c1)``.

        Uses half-open intervals, just like Python slicing — ``r0`` is
        included, ``r1`` is excluded.

        >>> Matrix([[1, 2, 3], [4, 5, 6], [7, 8, 9]]).slice(0, 2, 1, 3).data
        [[2.0, 3.0], [5.0, 6.0]]
        """
        if r0 < 0 or r1 > self.rows or c0 < 0 or c1 > self.cols:
            raise IndexError(
                f"Slice [{r0}:{r1}, {c0}:{c1}] out of bounds for "
                f"{self.rows}x{self.cols} matrix"
            )
        if r0 >= r1 or c0 >= c1:
            raise ValueError("Slice dimensions must be positive (r0 < r1, c0 < c1)")
        return Matrix([
            [float(self.data[i][j]) for j in range(c0, c1)]
            for i in range(r0, r1)
        ])

    # ------------------------------------------------------------------
    # Equality and comparison
    # ------------------------------------------------------------------

    def equals(self, other: "Matrix") -> bool:
        """Exact element-wise equality.

        Two matrices are equal if they have the same shape and every
        corresponding element is identical.

        >>> Matrix([[1, 2]]).equals(Matrix([[1, 2]]))
        True
        """
        if self.rows != other.rows or self.cols != other.cols:
            return False
        for i in range(self.rows):
            for j in range(self.cols):
                if self.data[i][j] != other.data[i][j]:
                    return False
        return True

    def close(self, other: "Matrix", tolerance: float = 1e-9) -> bool:
        """Check whether two matrices are element-wise close.

        Useful for comparing results of floating-point arithmetic, where
        tiny rounding errors make exact equality unreliable.  The default
        tolerance of ``1e-9`` is tight enough for double-precision math.

        >>> a = Matrix([[1.0000000001]])
        >>> b = Matrix([[1.0]])
        >>> a.close(b)
        True
        """
        if self.rows != other.rows or self.cols != other.cols:
            return False
        for i in range(self.rows):
            for j in range(self.cols):
                if builtins_abs(self.data[i][j] - other.data[i][j]) > tolerance:
                    return False
        return True


# ======================================================================
# Module-level helpers
# ======================================================================
# We shadow the built-in names ``sum`` and ``abs`` with our Matrix
# methods, so we stash references to the originals here.  This is a
# common Python idiom when method names coincide with builtins.

builtins_sum = sum
builtins_abs = abs
