"""Matrix construction, shape, and trivial accessors."""

from __future__ import annotations

from collections.abc import Sequence

from symbolic_ir import IRApply, IRInteger, IRNode, IRSymbol

from cas_matrix.heads import LIST, MATRIX


class MatrixError(ValueError):
    """Raised on shape mismatch, malformed matrix, etc."""


# ---------------------------------------------------------------------------
# Construction and shape
# ---------------------------------------------------------------------------


def matrix(rows: Sequence[Sequence[IRNode]]) -> IRApply:
    """Build a ``Matrix`` from a Python iterable of row iterables.

    Every row must have the same length. The matrix is stored as
    ``IRApply(MATRIX, (row1, row2, ...))`` where each row is itself
    ``IRApply(LIST, (cell, cell, ...))``.
    """
    if not rows:
        raise MatrixError("matrix() requires at least one row")
    width = len(rows[0])
    for i, row in enumerate(rows):
        if len(row) != width:
            raise MatrixError(
                f"matrix row {i} has {len(row)} entries, expected {width}"
            )
    irs_rows = tuple(IRApply(LIST, tuple(row)) for row in rows)
    return IRApply(MATRIX, irs_rows)


def is_matrix(node: IRNode) -> bool:
    return (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head.name == "Matrix"
    )


def _rows_of(M: IRNode) -> tuple[tuple[IRNode, ...], ...]:
    if not is_matrix(M):
        raise MatrixError(f"expected a Matrix, got {M!r}")
    assert isinstance(M, IRApply)  # for type-narrowing
    return tuple(_row_args(r) for r in M.args)


def _row_args(row: IRNode) -> tuple[IRNode, ...]:
    if (
        isinstance(row, IRApply)
        and isinstance(row.head, IRSymbol)
        and row.head.name == "List"
    ):
        return row.args
    raise MatrixError(f"matrix row must be a List, got {row!r}")


def dimensions(M: IRNode) -> IRApply:
    """Return ``IRApply(LIST, (IRInteger(rows), IRInteger(cols)))``."""
    rows = _rows_of(M)
    nrows = len(rows)
    ncols = len(rows[0]) if rows else 0
    return IRApply(LIST, (IRInteger(nrows), IRInteger(ncols)))


def num_rows(M: IRNode) -> int:
    return len(_rows_of(M))


def num_cols(M: IRNode) -> int:
    rows = _rows_of(M)
    return len(rows[0]) if rows else 0


def get_entry(M: IRNode, row: int, col: int) -> IRNode:
    """1-based access to the entry at (row, col)."""
    rows = _rows_of(M)
    if not (1 <= row <= len(rows) and 1 <= col <= len(rows[0])):
        raise MatrixError(f"index ({row}, {col}) out of range")
    return rows[row - 1][col - 1]
