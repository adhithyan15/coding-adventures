"""Matrix arithmetic — elementwise plus dot product and transpose."""

from __future__ import annotations

from symbolic_ir import ADD, MUL, SUB, IRApply, IRInteger, IRNode

from cas_matrix.matrix import (
    MatrixError,
    _rows_of,
    matrix,
    num_cols,
    num_rows,
)

# ---------------------------------------------------------------------------
# Constructors
# ---------------------------------------------------------------------------


def identity_matrix(n: int) -> IRApply:
    """``n``-by-``n`` identity matrix with integer 0/1 entries."""
    if n < 0:
        raise MatrixError("identity_matrix: n must be non-negative")
    rows: list[list[IRNode]] = []
    for i in range(n):
        row: list[IRNode] = []
        for j in range(n):
            row.append(IRInteger(1) if i == j else IRInteger(0))
        rows.append(row)
    return matrix(rows)


def zero_matrix(rows: int, cols: int) -> IRApply:
    """Rows-by-cols matrix of integer zeros."""
    if rows < 0 or cols < 0:
        raise MatrixError("zero_matrix: dims must be non-negative")
    return matrix([[IRInteger(0) for _ in range(cols)] for _ in range(rows)])


# ---------------------------------------------------------------------------
# Elementwise operations
# ---------------------------------------------------------------------------


def transpose(M: IRNode) -> IRApply:
    """Transpose."""
    rows = _rows_of(M)
    if not rows:
        return matrix([])
    nrows = len(rows)
    ncols = len(rows[0])
    new_rows: list[list[IRNode]] = []
    for j in range(ncols):
        row = [rows[i][j] for i in range(nrows)]
        new_rows.append(row)
    return matrix(new_rows)


def add_matrices(A: IRNode, B: IRNode) -> IRApply:
    """A + B elementwise (shapes must match)."""
    a_rows = _rows_of(A)
    b_rows = _rows_of(B)
    _check_same_shape(a_rows, b_rows, op="add")
    new_rows: list[list[IRNode]] = []
    for ra, rb in zip(a_rows, b_rows, strict=True):
        new_rows.append([IRApply(ADD, (x, y)) for x, y in zip(ra, rb, strict=True)])
    return matrix(new_rows)


def sub_matrices(A: IRNode, B: IRNode) -> IRApply:
    """A - B elementwise."""
    a_rows = _rows_of(A)
    b_rows = _rows_of(B)
    _check_same_shape(a_rows, b_rows, op="sub")
    new_rows: list[list[IRNode]] = []
    for ra, rb in zip(a_rows, b_rows, strict=True):
        new_rows.append([IRApply(SUB, (x, y)) for x, y in zip(ra, rb, strict=True)])
    return matrix(new_rows)


def scalar_multiply(scalar: IRNode, M: IRNode) -> IRApply:
    """Multiply every entry by ``scalar``."""
    rows = _rows_of(M)
    new_rows: list[list[IRNode]] = []
    for row in rows:
        new_rows.append([IRApply(MUL, (scalar, x)) for x in row])
    return matrix(new_rows)


def trace(M: IRNode) -> IRNode:
    """Sum of the main diagonal. Square matrices only."""
    rows = _rows_of(M)
    if num_rows(M) != num_cols(M):
        cols = len(rows[0]) if rows else 0
        raise MatrixError(
            f"trace: matrix must be square, got {len(rows)}x{cols}"
        )
    diag = tuple(rows[i][i] for i in range(len(rows)))
    if not diag:
        return IRInteger(0)
    if len(diag) == 1:
        return diag[0]
    return IRApply(ADD, diag)


# ---------------------------------------------------------------------------
# Dot product
# ---------------------------------------------------------------------------


def dot(A: IRNode, B: IRNode) -> IRApply:
    """Matrix product. ``cols(A)`` must equal ``rows(B)``."""
    a_rows = _rows_of(A)
    b_rows = _rows_of(B)
    if not a_rows or not b_rows:
        raise MatrixError("dot: both operands must have at least one row")
    a_cols = len(a_rows[0])
    if a_cols != len(b_rows):
        raise MatrixError(
            f"dot: cols(A)={a_cols} != rows(B)={len(b_rows)}"
        )
    b_cols = len(b_rows[0])
    new_rows: list[list[IRNode]] = []
    for i in range(len(a_rows)):
        row: list[IRNode] = []
        for j in range(b_cols):
            terms = tuple(
                IRApply(MUL, (a_rows[i][k], b_rows[k][j])) for k in range(a_cols)
            )
            if len(terms) == 1:
                row.append(terms[0])
            else:
                row.append(IRApply(ADD, terms))
        new_rows.append(row)
    return matrix(new_rows)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _check_same_shape(
    a: tuple[tuple[IRNode, ...], ...],
    b: tuple[tuple[IRNode, ...], ...],
    *,
    op: str,
) -> None:
    if len(a) != len(b) or (a and len(a[0]) != len(b[0])):
        raise MatrixError(
            f"{op}: shape mismatch "
            f"({len(a)}x{len(a[0]) if a else 0} vs {len(b)}x{len(b[0]) if b else 0})"
        )
