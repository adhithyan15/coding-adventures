"""Matrix and vector norms.

This module provides two norm computations:

1. **Euclidean (L²) norm** for column vectors: ``‖v‖ = √(Σ vᵢ²)``
2. **Frobenius norm** for matrices: ``‖A‖_F = √(Σᵢⱼ aᵢⱼ²)``

Both are computed exactly over the rationals.  If the sum of squares is
a perfect square (numerator and denominator are both perfect squares of
integers), the result is returned as ``IRInteger`` or ``IRRational``.
Otherwise the result is ``IRApply(SQRT, (sum_of_squares,))``.

Vector vs matrix
----------------
A "column vector" is defined as an m×1 matrix.  A row vector (1×n) is
also accepted.  For any other matrix shape, a second argument ``"frobenius"``
must be provided.

API
---
``norm(M)``               — Euclidean norm of a column/row vector.
``norm(M, "frobenius")``  — Frobenius norm of any matrix.

Both fall through (return the unevaluated ``Norm(…)`` node) when:

- Entries are symbolic (not ``IRInteger``/``IRRational``).
- A matrix is passed without the ``"frobenius"`` flag.
- The input is not a valid matrix.

Literate reading order
-----------------------
1. ``_sum_of_squares``  — exact Fraction sum
2. ``_sqrt_fraction``   — exact square root or SQRT IR node
3. ``norm``             — main public function
"""

from __future__ import annotations

from fractions import Fraction
from math import isqrt

from symbolic_ir import SQRT, IRApply, IRNode

from cas_matrix.matrix import MatrixError, _rows_of, num_cols, num_rows
from cas_matrix.rowreduce import _entry_to_fraction, _frac_to_ir

# ---------------------------------------------------------------------------
# Section 1 — Sum of squares
# ---------------------------------------------------------------------------


def _sum_of_squares(entries: list[Fraction]) -> Fraction:
    """Return the exact sum of squares of a list of Fraction values.

    Parameters
    ----------
    entries:
        Flat list of matrix entries already converted to Fraction.

    Returns
    -------
    ``Σ eᵢ²`` as a ``Fraction``.
    """
    return sum(e * e for e in entries)


# ---------------------------------------------------------------------------
# Section 2 — Exact square root or SQRT IR node
# ---------------------------------------------------------------------------


def _isqrt_fraction(f: Fraction) -> Fraction | None:
    """Return the exact square root of f if f is a perfect square, else None.

    A fraction p/q is a perfect square when both p and q are perfect squares
    of integers.

    Parameters
    ----------
    f:
        A non-negative Fraction.

    Returns
    -------
    ``Fraction(√p, √q)`` if both are perfect squares, else ``None``.
    """
    p, q = f.numerator, f.denominator
    if p < 0 or q < 0:
        return None
    sp = isqrt(p)
    sq = isqrt(q)
    if sp * sp == p and sq * sq == q:
        return Fraction(sp, sq)
    return None


def _sqrt_fraction(total: Fraction) -> IRNode:
    """Return the exact IR representation of √total.

    Parameters
    ----------
    total:
        A non-negative Fraction (the sum of squares).

    Returns
    -------
    ``IRInteger`` or ``IRRational`` if total is a perfect square; otherwise
    ``IRApply(SQRT, (total_ir,))``.
    """
    exact = _isqrt_fraction(total)
    if exact is not None:
        return _frac_to_ir(exact)
    return IRApply(SQRT, (_frac_to_ir(total),))


# ---------------------------------------------------------------------------
# Section 3 — Public norm function
# ---------------------------------------------------------------------------


def norm(M: IRNode, kind: str | None = None) -> IRNode:
    """Compute the Euclidean or Frobenius norm of a matrix.

    Parameters
    ----------
    M:
        A ``Matrix`` IR node with ``IRInteger`` / ``IRRational`` entries.
    kind:
        ``None`` (default) — Euclidean norm of a column or row vector.
        ``"frobenius"`` — Frobenius norm of any matrix.

    Returns
    -------
    An IR node representing the norm:

    - ``IRInteger`` or ``IRRational`` if the sum of squares is a perfect
      square (exact integer/rational result).
    - ``IRApply(SQRT, (sum_of_squares_ir,))`` otherwise.

    Raises
    ------
    MatrixError
        If entries are symbolic, or if kind is None and M is not a vector.
        If kind is unknown.

    Examples
    --------
    ::

        # Euclidean norm of [3, 4]:  √(9+16) = √25 = 5
        v = matrix([[IRInteger(3)], [IRInteger(4)]])
        norm(v)  # IRInteger(5)

        # Frobenius norm of [[1,1],[1,1]]:  √(1+1+1+1) = √4 = 2
        A = matrix([[IRInteger(1), IRInteger(1)],
                    [IRInteger(1), IRInteger(1)]])
        norm(A, "frobenius")  # IRInteger(2)

        # Non-perfect square: √2
        v2 = matrix([[IRInteger(1)], [IRInteger(1)]])
        norm(v2)
        # IRApply(SQRT, (IRInteger(2),))
    """
    if kind is not None and kind != "frobenius":
        raise MatrixError(f"norm: unknown norm kind {kind!r}; use 'frobenius' or None")

    rows = _rows_of(M)
    nr = num_rows(M)
    nc = num_cols(M)

    if kind is None and nc != 1 and nr != 1:
        # Euclidean vector norm: M must be m×1 or 1×n.
        raise MatrixError(
            "norm: Euclidean norm requires a column or row vector "
            f"(got {nr}×{nc}); use norm(M, 'frobenius') for matrices"
        )

    # Flatten all entries and convert to Fraction.
    flat: list[Fraction] = []
    for row in rows:
        for entry in row:
            flat.append(_entry_to_fraction(entry))

    total = _sum_of_squares(flat)
    return _sqrt_fraction(total)
