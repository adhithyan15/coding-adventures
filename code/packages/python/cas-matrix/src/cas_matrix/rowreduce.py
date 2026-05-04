"""Row reduction and rank for numeric (rational/integer) matrices.

This module provides two operations that work only on matrices whose every
entry is a pure rational number (``IRInteger`` or ``IRRational``).  Symbolic
entries cause an immediate ``MatrixError``; callers should either simplify
first or handle the exception as a fall-through.

Algorithms
----------

Both operations are implemented via Gauss-Jordan elimination over the
rationals.  Since all arithmetic is exact (Python ``fractions.Fraction``),
there is no floating-point cancellation to worry about.

Row-reduce (RREF)
~~~~~~~~~~~~~~~~~
Gauss-Jordan elimination:

1. For each column ``c`` (left to right), find the topmost non-zero entry in
   that column at or below the current pivot row.
2. Swap that row up to the pivot position.
3. Divide the pivot row by the pivot entry so the pivot becomes 1.
4. Eliminate all other entries in column ``c`` (both above and below the
   pivot) using row operations.
5. Advance the pivot row counter.

The output is a matrix in reduced row echelon form:

- Every pivot (leading non-zero) is 1.
- Every other entry in a pivot column is 0.
- Pivot columns are strictly left of any pivot below them (staircase form).
- Zero rows, if any, are at the bottom.

Rank
~~~~
The rank is simply the number of non-zero rows in *row echelon form* (not
RREF — we stop the forward pass early and skip the backward-elimination
step to save work).  Equivalently, it is the number of pivot columns.

Example::

    M = Matrix([1, 2, 3], [4, 5, 6], [7, 8, 9])
    row_reduce(M)
    # → Matrix([1, 0, -1], [0, 1, 2], [0, 0, 0])   (rank 2)

    rank(M)
    # → 2
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import IRApply, IRInteger, IRNode, IRRational

from cas_matrix.matrix import MatrixError, _rows_of, matrix

# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


def _entry_to_fraction(node: IRNode) -> Fraction:
    """Convert an ``IRInteger`` or ``IRRational`` to a ``Fraction``.

    Raises ``MatrixError`` for any other node type (symbolic entries).

    Parameters
    ----------
    node:
        A leaf IR node representing a numeric constant.

    Returns
    -------
    The corresponding ``Fraction`` value.

    Raises
    ------
    MatrixError
        If ``node`` is not ``IRInteger`` or ``IRRational``.
    """
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    raise MatrixError(
        f"row_reduce/rank: symbolic entry not supported: {node!r}"
    )


def _matrix_to_fractions(M: IRNode) -> list[list[Fraction]]:
    """Extract a matrix's entries as a mutable list-of-lists of Fractions.

    Raises ``MatrixError`` if any entry is symbolic.

    Parameters
    ----------
    M:
        An IR node produced by ``matrix()``.  Must satisfy ``is_matrix(M)``.

    Returns
    -------
    ``rows[r][c]`` is the entry at row ``r``, column ``c`` (0-based).
    """
    rows = _rows_of(M)
    return [[_entry_to_fraction(e) for e in row] for row in rows]


def _frac_to_ir(f: Fraction) -> IRNode:
    """Convert a Fraction to the canonical IR literal form.

    Returns ``IRInteger`` for whole numbers and ``IRRational`` for fractions,
    matching the convention used everywhere else in the CAS.
    """
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)


# ---------------------------------------------------------------------------
# Public functions
# ---------------------------------------------------------------------------


def row_reduce(M: IRNode) -> IRApply:
    """Return the reduced row echelon form (RREF) of matrix ``M``.

    Only numeric matrices (integer / rational entries) are supported.
    Symbolic entries cause a ``MatrixError``.

    The algorithm is Gauss-Jordan elimination over the rationals:
    exact arithmetic avoids all floating-point rounding errors.

    Parameters
    ----------
    M:
        A valid ``Matrix`` IR node with ``IRInteger`` / ``IRRational``
        entries.

    Returns
    -------
    A new ``Matrix`` IR node in RREF:

    - Every leading entry is exactly ``1`` (``IRInteger(1)``).
    - Every other entry in a pivot column is exactly ``0``.
    - Zero rows are at the bottom.

    Raises
    ------
    MatrixError
        If ``M`` is not a matrix or has symbolic entries.

    Examples
    --------
    ::

        M = matrix([[IRInteger(1), IRInteger(2), IRInteger(3)],
                    [IRInteger(4), IRInteger(5), IRInteger(6)],
                    [IRInteger(7), IRInteger(8), IRInteger(9)]])
        row_reduce(M)
        # Matrix with rows [1, 0, -1], [0, 1, 2], [0, 0, 0]

        M2 = matrix([[IRInteger(1), IRInteger(0)],
                     [IRInteger(0), IRInteger(1)]])
        row_reduce(M2)
        # Identity — unchanged
    """
    frows = _matrix_to_fractions(M)
    nr = len(frows)
    nc = len(frows[0]) if frows else 0

    pivot_row = 0  # the next row where we'll place a pivot
    for col in range(nc):
        # Step 1: find a non-zero entry in column col at or below pivot_row.
        pivot_pos: int | None = None
        for r in range(pivot_row, nr):
            if frows[r][col] != 0:
                pivot_pos = r
                break
        if pivot_pos is None:
            continue  # entire column below pivot_row is zero — move right

        # Step 2: swap the pivot row up.
        if pivot_pos != pivot_row:
            frows[pivot_row], frows[pivot_pos] = frows[pivot_pos], frows[pivot_row]

        # Step 3: normalise so the pivot entry is 1.
        pv = frows[pivot_row][col]
        frows[pivot_row] = [x / pv for x in frows[pivot_row]]

        # Step 4: eliminate all other entries in this column (above + below).
        for r in range(nr):
            if r != pivot_row:
                factor = frows[r][col]
                if factor != 0:
                    frows[r] = [
                        frows[r][c] - factor * frows[pivot_row][c]
                        for c in range(nc)
                    ]

        pivot_row += 1

    # Convert back to IR.
    ir_rows = [[_frac_to_ir(f) for f in row] for row in frows]
    return matrix(ir_rows)


def rank(M: IRNode) -> IRInteger:
    """Return the rank of matrix ``M`` as an ``IRInteger``.

    The rank is the dimension of the row space — equivalently, the number of
    non-zero rows in any row echelon form.

    Only numeric matrices (integer / rational entries) are supported.
    Symbolic entries cause a ``MatrixError``.

    Parameters
    ----------
    M:
        A valid ``Matrix`` IR node with ``IRInteger`` / ``IRRational``
        entries.

    Returns
    -------
    ``IRInteger(r)`` where ``r`` is the rank.

    Raises
    ------
    MatrixError
        If ``M`` is not a matrix or has symbolic entries.

    Examples
    --------
    ::

        # Full-rank 2×2 identity
        I = identity_matrix(2)
        rank(I)  # IRInteger(2)

        # Rank-2 singular 3×3
        M = matrix([[1,2,3],[4,5,6],[7,8,9]])  (converted to IR)
        rank(M)  # IRInteger(2)

        # Zero matrix
        Z = zero_matrix(3, 3)
        rank(Z)  # IRInteger(0)
    """
    frows = _matrix_to_fractions(M)
    nr = len(frows)
    nc = len(frows[0]) if frows else 0

    # Forward elimination (REF only — no back-substitution needed for rank).
    pivot_row = 0
    for col in range(nc):
        pivot_pos: int | None = None
        for r in range(pivot_row, nr):
            if frows[r][col] != 0:
                pivot_pos = r
                break
        if pivot_pos is None:
            continue

        frows[pivot_row], frows[pivot_pos] = frows[pivot_pos], frows[pivot_row]
        pv = frows[pivot_row][col]
        frows[pivot_row] = [x / pv for x in frows[pivot_row]]

        # Eliminate only BELOW the pivot (REF, not RREF).
        for r in range(pivot_row + 1, nr):
            factor = frows[r][col]
            if factor != 0:
                frows[r] = [
                    frows[r][c] - factor * frows[pivot_row][c]
                    for c in range(nc)
                ]

        pivot_row += 1

    # Count non-zero rows.
    rk = sum(1 for row in frows if any(x != 0 for x in row))
    return IRInteger(rk)
