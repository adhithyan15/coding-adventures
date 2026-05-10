"""Null space, column space, and row space via RREF.

All three subspace operations derive from the reduced row echelon form
(RREF) of the input matrix, computed by
:func:`cas_matrix.rowreduce.row_reduce`.

Null space
----------
The null space of an m×n matrix A is the set of all vectors v such that
Av = 0.  Its basis is found from the RREF of A:

- **Pivot columns** (leading-1 columns): correspond to "basic" variables.
- **Free columns** (non-pivot): correspond to "free" variables.

For each free column j, assign free variable j the value 1 and all other
free variables the value 0.  The basic variables are then determined by
back-substitution from the RREF rows.  This gives one basis vector per
free variable.

Column space
------------
The column space (image) of A is the span of its columns.  Its basis
consists of the **pivot columns of the original A** (not of the RREF) —
because column operations can change the column space, but RREF row
operations preserve column-space membership.

Row space
---------
The row space of A is the span of its rows.  Its basis consists of the
**non-zero rows of the RREF** — because row operations preserve the row
space.

Return format
-------------
All three functions return a ``List(v₁, v₂, …)`` of column-vector (or
row-vector) ``Matrix`` IR nodes.  An empty ``List()`` signals a trivial
subspace.

Literate reading order
-----------------------
1. ``_rref_pivot_cols``  — extract pivot column indices from RREF
2. ``nullspace``         — null-space basis
3. ``columnspace``       — column-space basis
4. ``rowspace``          — row-space basis
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import IRApply, IRNode, IRSymbol

from cas_matrix.eigenvalues import _null_space_fractions
from cas_matrix.matrix import _rows_of, matrix, num_rows
from cas_matrix.rowreduce import _frac_to_ir, _matrix_to_fractions

# ---------------------------------------------------------------------------
# Section 1 — Internal helper: extract pivot columns from RREF
# ---------------------------------------------------------------------------


def _rref_pivot_info(
    frows: list[list[Fraction]],
) -> tuple[list[int], list[list[Fraction]]]:
    """Compute RREF of a Fraction matrix and return pivot column indices + RREF.

    Parameters
    ----------
    frows:
        Row-major list-of-lists of Fraction, shape m×n.

    Returns
    -------
    A tuple ``(pivot_cols, rref_rows)`` where:

    - ``pivot_cols[r]`` is the column index of the pivot in row r.
    - ``rref_rows`` is the matrix in reduced row echelon form.

    Only rows that actually contain a pivot are represented in
    ``pivot_cols``.
    """
    if not frows or not frows[0]:
        return [], frows
    m = len(frows)
    n = len(frows[0])
    rref = [row[:] for row in frows]
    pivot_cols: list[int] = []
    pivot_row = 0
    for col in range(n):
        pivot_pos: int | None = None
        for r in range(pivot_row, m):
            if rref[r][col] != 0:
                pivot_pos = r
                break
        if pivot_pos is None:
            continue
        if pivot_pos != pivot_row:
            rref[pivot_row], rref[pivot_pos] = rref[pivot_pos], rref[pivot_row]
        pv = rref[pivot_row][col]
        rref[pivot_row] = [x / pv for x in rref[pivot_row]]
        for r in range(m):
            if r != pivot_row:
                factor = rref[r][col]
                if factor != 0:
                    rref[r] = [rref[r][c] - factor * rref[pivot_row][c]
                               for c in range(n)]
        pivot_cols.append(col)
        pivot_row += 1
    return pivot_cols, rref


# ---------------------------------------------------------------------------
# Section 2 — Null space
# ---------------------------------------------------------------------------


def nullspace(M: IRNode) -> IRApply:
    """Return a basis for the null space (kernel) of matrix M.

    The null space of an m×n matrix A is:
        ``null(A) = {v ∈ ℝⁿ : Av = 0}``

    Its dimension is n − rank(A) (the "nullity").

    Algorithm
    ---------
    1. Compute RREF(A).
    2. Identify free columns (non-pivot columns).
    3. For each free column j build a basis vector v:
       - ``v[j] = 1``
       - ``v[free_k] = 0`` for all other free columns k
       - ``v[pivot_col] = −RREF[pivot_row, j]``

    Parameters
    ----------
    M:
        An m×n matrix with ``IRInteger`` / ``IRRational`` entries.

    Returns
    -------
    ``List(v₁, v₂, …)`` where each ``vᵢ`` is an n×1 column-vector
    ``Matrix`` IR node.  Returns ``List()`` for a full-column-rank matrix
    (trivial null space).

    Raises
    ------
    MatrixError
        If M is not a valid matrix or has symbolic entries.

    Examples
    --------
    ::

        # [[1,2,3],[4,5,6]] → RREF [[1,0,-1],[0,1,2]],
        # free col 2: v = [1, -2, 1]
        nullspace(matrix([[1,2,3],[4,5,6]]))
        # List(Matrix([[1],[-2],[1]]))
    """
    LIST_HEAD = IRSymbol("List")
    frows = _matrix_to_fractions(M)
    basis = _null_space_fractions(frows)
    vec_nodes = [
        matrix([[_frac_to_ir(v)] for v in vec])
        for vec in basis
    ]
    return IRApply(LIST_HEAD, tuple(vec_nodes))


# ---------------------------------------------------------------------------
# Section 3 — Column space
# ---------------------------------------------------------------------------


def columnspace(M: IRNode) -> IRApply:
    """Return a basis for the column space (image) of matrix M.

    The column space is the span of the columns of A.  The basis consists
    of the pivot columns of the **original** A (not the RREF).

    Why original columns and not RREF pivot columns?  Row operations
    preserve which columns are linearly independent (and thus which columns
    span the column space), but they change the actual column vectors.
    The original pivot columns retain their geometric meaning.

    Parameters
    ----------
    M:
        An m×n matrix with ``IRInteger`` / ``IRRational`` entries.

    Returns
    -------
    ``List(c₁, c₂, …)`` where each ``cᵢ`` is an m×1 column-vector
    ``Matrix`` IR node.

    Raises
    ------
    MatrixError
        If M is not a valid matrix or has symbolic entries.

    Examples
    --------
    ::

        # [[1,2],[2,4]] → RREF [[1,2],[0,0]], pivot col 0
        # column space basis: original column 0 = [1,2]
        columnspace(matrix([[1,2],[2,4]]))
        # List(Matrix([[1],[2]]))
    """
    LIST_HEAD = IRSymbol("List")
    frows = _matrix_to_fractions(M)
    pivot_cols, _ = _rref_pivot_info(frows)

    # Extract the corresponding columns from the *original* IR matrix.
    orig_rows = _rows_of(M)
    m = num_rows(M)

    col_nodes: list[IRNode] = []
    for c in pivot_cols:
        # Build column c as an m×1 matrix of the original entries.
        col = matrix([[orig_rows[r][c]] for r in range(m)])
        col_nodes.append(col)

    return IRApply(LIST_HEAD, tuple(col_nodes))


# ---------------------------------------------------------------------------
# Section 4 — Row space
# ---------------------------------------------------------------------------


def rowspace(M: IRNode) -> IRApply:
    """Return a basis for the row space of matrix M.

    The row space is the span of the rows of A.  Its basis is the set of
    **non-zero rows of the RREF** — row operations preserve the row space,
    so the non-zero RREF rows form an orthonormal-like basis (pivots at 1,
    everything else reduced).

    Parameters
    ----------
    M:
        An m×n matrix with ``IRInteger`` / ``IRRational`` entries.

    Returns
    -------
    ``List(r₁, r₂, …)`` where each ``rᵢ`` is a 1×n row-vector
    ``Matrix`` IR node.

    Raises
    ------
    MatrixError
        If M is not a valid matrix or has symbolic entries.

    Examples
    --------
    ::

        # [[1,2,3],[4,5,6]] → RREF [[1,0,-1],[0,1,2]]
        # both rows are non-zero
        rowspace(matrix([[1,2,3],[4,5,6]]))
        # List(Matrix([[1,0,-1]]), Matrix([[0,1,2]]))
    """
    LIST_HEAD = IRSymbol("List")
    frows = _matrix_to_fractions(M)
    _, rref = _rref_pivot_info(frows)

    row_nodes: list[IRNode] = []
    for row in rref:
        if any(x != 0 for x in row):
            ir_row = [_frac_to_ir(x) for x in row]
            row_nodes.append(matrix([ir_row]))

    return IRApply(LIST_HEAD, tuple(row_nodes))
