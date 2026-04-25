"""Determinant and inverse via cofactor expansion.

The implementation produces *symbolic* output: every arithmetic step
is wrapped as ``IRApply(ADD/SUB/MUL/NEG/...)`` rather than collapsed
to a number. Pass the result through ``cas_simplify.simplify`` (or any
downstream pass) to reduce.

Cofactor expansion is O(n!) so it's only practical up to roughly
n=6. For numeric-only matrices, a future fast path can detect that
and switch to Bareiss elimination.
"""

from __future__ import annotations

from symbolic_ir import ADD, MUL, NEG, SUB, IRApply, IRInteger, IRNode

from cas_matrix.matrix import MatrixError, _rows_of, matrix, num_cols, num_rows


def determinant(M: IRNode) -> IRNode:
    """Determinant of a square matrix. Returns un-simplified IR."""
    rows = _rows_of(M)
    n = num_rows(M)
    if n != num_cols(M):
        raise MatrixError(f"determinant: matrix must be square, got {n}x{num_cols(M)}")
    return _det(rows)


def _det(rows: tuple[tuple[IRNode, ...], ...]) -> IRNode:
    n = len(rows)
    if n == 0:
        return IRInteger(1)  # det of 0x0 is 1 by convention
    if n == 1:
        return rows[0][0]
    if n == 2:
        # ad - bc
        a, b = rows[0]
        c, d = rows[1]
        return IRApply(SUB, (IRApply(MUL, (a, d)), IRApply(MUL, (b, c))))
    # Expand along the first row.
    terms: list[IRNode] = []
    for j, entry in enumerate(rows[0]):
        minor = _minor(rows, 0, j)
        sub_det = _det(minor)
        product: IRNode = IRApply(MUL, (entry, sub_det))
        if j % 2 == 1:
            product = IRApply(NEG, (product,))
        terms.append(product)
    if len(terms) == 1:
        return terms[0]
    return IRApply(ADD, tuple(terms))


def _minor(
    rows: tuple[tuple[IRNode, ...], ...],
    skip_row: int,
    skip_col: int,
) -> tuple[tuple[IRNode, ...], ...]:
    """Return the minor obtained by deleting ``skip_row`` and ``skip_col``."""
    return tuple(
        tuple(cell for cj, cell in enumerate(row) if cj != skip_col)
        for ri, row in enumerate(rows)
        if ri != skip_row
    )


def inverse(M: IRNode) -> IRApply:
    """Symbolic matrix inverse via adjugate / determinant.

    Returns the inverse as a Matrix whose entries are symbolic
    expressions. Pass through ``cas_simplify.simplify`` to reduce.
    """
    rows = _rows_of(M)
    n = num_rows(M)
    if n != num_cols(M):
        raise MatrixError(f"inverse: matrix must be square, got {n}x{num_cols(M)}")
    if n == 0:
        return matrix([])
    det = _det(rows)
    # Build the matrix of cofactors, then transpose to get the adjugate.
    cof_rows: list[list[IRNode]] = []
    for i in range(n):
        cof_row: list[IRNode] = []
        for j in range(n):
            sub_det = _det(_minor(rows, i, j))
            entry: IRNode = sub_det
            if (i + j) % 2 == 1:
                entry = IRApply(NEG, (entry,))
            cof_row.append(entry)
        cof_rows.append(cof_row)
    # Transpose cofactor matrix → adjugate, then divide every entry by det.
    adj_rows = [
        [cof_rows[r][c] for r in range(n)] for c in range(n)
    ]
    from symbolic_ir import DIV

    inv_rows: list[list[IRNode]] = []
    for row in adj_rows:
        inv_rows.append([IRApply(DIV, (cell, det)) for cell in row])
    return matrix(inv_rows)
