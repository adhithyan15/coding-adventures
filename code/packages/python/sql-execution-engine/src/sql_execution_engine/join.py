"""Join logic — five SQL join types implemented in pure Python.

SQL Joins Explained
--------------------

A join combines rows from two tables (``left`` and ``right``) based on a
join condition (``ON``).  The five join types differ in how they handle
rows that have no matching partner.

.. list-table::
   :widths: 20 80
   :header-rows: 1

   * - Join type
     - Behaviour
   * - ``INNER``
     - Only rows that match the ON condition on both sides.
   * - ``LEFT (OUTER)``
     - All left rows. Non-matching left rows get NULLs for right columns.
   * - ``RIGHT (OUTER)``
     - All right rows. Non-matching right rows get NULLs for left columns.
   * - ``FULL (OUTER)``
     - All rows from both sides. Unmatched sides padded with NULLs.
   * - ``CROSS``
     - Cartesian product — every combination of left × right rows.
       The ON condition is ignored for CROSS JOIN.

Implementation Note
--------------------

This is a **nested-loop join** — the simplest possible join algorithm.
For each left row we scan every right row and test the ON condition.
The time complexity is O(|left| × |right|), which is fine for an
educational engine but not for production.

A production engine would use:
- **Hash join** for equality conditions (O(n+m))
- **Merge join** for sorted inputs (O(n log n + m log m))
- **Index nested-loop join** for indexed columns

Row Representation
-------------------

During a join, each row is stored as a flat dict.  To avoid key collisions
when both tables have a column named ``"id"``, we prefix qualified names:

.. code-block:: python

    # employees has "id", departments has "id" too
    merged_row = {
        "employees.id": 1,
        "employees.name": "Alice",
        "departments.id": 1,
        "departments.name": "Engineering",
    }

The executor also adds bare column names (the last-seen value wins) so
that simple references like ``WHERE name = 'Alice'`` still work.
"""

from __future__ import annotations

from typing import Any

from sql_execution_engine.expression import eval_expr
from lang_parser import ASTNode
from lexer import Token


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def perform_join(
    left_rows: list[dict[str, Any]],
    left_alias: str,
    right_rows: list[dict[str, Any]],
    right_alias: str,
    join_type: str,
    on_condition: ASTNode | Token | None,
) -> list[dict[str, Any]]:
    """Perform a SQL join between two sets of rows.

    Args:
        left_rows: Rows from the left table (already may be a join result).
        left_alias: Table alias for the left side (used for qualified keys).
        right_rows: Rows from the right table.
        right_alias: Table alias for the right side.
        join_type: One of ``"INNER"``, ``"LEFT"``, ``"RIGHT"``,
                   ``"FULL"``, or ``"CROSS"``.
        on_condition: The ON expression AST node, or ``None`` for CROSS JOIN.

    Returns:
        The joined rows as a list of merged dicts.
    """
    # CROSS JOIN — no condition needed, just the Cartesian product.
    if join_type == "CROSS" or on_condition is None:
        return _cross_join(left_rows, right_alias, right_rows)

    null_left = _null_row_for(left_rows, left_alias)
    null_right = _null_row_for(right_rows, right_alias)

    if join_type == "INNER":
        return _inner_join(left_rows, right_rows, right_alias, on_condition)
    if join_type in ("LEFT", "LEFT OUTER"):
        return _left_join(left_rows, right_rows, right_alias, null_right, on_condition)
    if join_type in ("RIGHT", "RIGHT OUTER"):
        return _right_join(left_rows, left_alias, right_rows, right_alias, null_left, on_condition)
    if join_type in ("FULL", "FULL OUTER"):
        return _full_join(left_rows, left_alias, right_rows, right_alias, null_left, null_right, on_condition)

    # Default: INNER
    return _inner_join(left_rows, right_rows, right_alias, on_condition)


# ---------------------------------------------------------------------------
# Individual join implementations
# ---------------------------------------------------------------------------


def _inner_join(
    left_rows: list[dict[str, Any]],
    right_rows: list[dict[str, Any]],
    right_alias: str,
    condition: ASTNode | Token,
) -> list[dict[str, Any]]:
    """INNER JOIN — emit only rows where the condition is true.

    Nested-loop: for each left row, scan all right rows.
    Time: O(|left| × |right|).
    """
    result: list[dict[str, Any]] = []
    for lrow in left_rows:
        for rrow in right_rows:
            merged = _merge(lrow, right_alias, rrow)
            if _test_condition(condition, merged):
                result.append(merged)
    return result


def _left_join(
    left_rows: list[dict[str, Any]],
    right_rows: list[dict[str, Any]],
    right_alias: str,
    null_right: dict[str, Any],
    condition: ASTNode | Token,
) -> list[dict[str, Any]]:
    """LEFT (OUTER) JOIN — all left rows, NULL-padded right columns when no match."""
    result: list[dict[str, Any]] = []
    for lrow in left_rows:
        matched = False
        for rrow in right_rows:
            merged = _merge(lrow, right_alias, rrow)
            if _test_condition(condition, merged):
                result.append(merged)
                matched = True
        if not matched:
            result.append(_merge(lrow, right_alias, null_right))
    return result


def _right_join(
    left_rows: list[dict[str, Any]],
    left_alias: str,
    right_rows: list[dict[str, Any]],
    right_alias: str,
    null_left: dict[str, Any],
    condition: ASTNode | Token,
) -> list[dict[str, Any]]:
    """RIGHT (OUTER) JOIN — all right rows, NULL-padded left columns when no match."""
    result: list[dict[str, Any]] = []
    for rrow in right_rows:
        matched = False
        for lrow in left_rows:
            merged = _merge(lrow, right_alias, rrow)
            if _test_condition(condition, merged):
                result.append(merged)
                matched = True
        if not matched:
            result.append(_merge(null_left, right_alias, rrow))
    return result


def _full_join(
    left_rows: list[dict[str, Any]],
    left_alias: str,
    right_rows: list[dict[str, Any]],
    right_alias: str,
    null_left: dict[str, Any],
    null_right: dict[str, Any],
    condition: ASTNode | Token,
) -> list[dict[str, Any]]:
    """FULL OUTER JOIN — all rows from both sides, NULL-padded where unmatched."""
    result: list[dict[str, Any]] = []
    matched_right: set[int] = set()

    for lrow in left_rows:
        left_matched = False
        for i, rrow in enumerate(right_rows):
            merged = _merge(lrow, right_alias, rrow)
            if _test_condition(condition, merged):
                result.append(merged)
                left_matched = True
                matched_right.add(i)
        if not left_matched:
            result.append(_merge(lrow, right_alias, null_right))

    # Add unmatched right rows.
    for i, rrow in enumerate(right_rows):
        if i not in matched_right:
            result.append(_merge(null_left, right_alias, rrow))

    return result


def _cross_join(
    left_rows: list[dict[str, Any]],
    right_alias: str,
    right_rows: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    """CROSS JOIN — Cartesian product of left × right."""
    return [
        _merge(lrow, right_alias, rrow)
        for lrow in left_rows
        for rrow in right_rows
    ]


# ---------------------------------------------------------------------------
# Row merging helpers
# ---------------------------------------------------------------------------


def _merge(
    left: dict[str, Any],
    right_alias: str,
    right: dict[str, Any],
) -> dict[str, Any]:
    """Merge a left row and a right row into a single flat dict.

    The merged row contains:
    - All keys from the left row (unchanged — they are already qualified).
    - Right row keys prefixed with ``right_alias.`` (if not already qualified).
    - Bare (unqualified) keys from both sides for convenience.

    When key collision occurs (both sides have the same bare column name),
    the right side's value is used for the bare key. Qualified keys
    ``"alias.col"`` always uniquely identify the origin.
    """
    merged: dict[str, Any] = dict(left)

    for key, val in right.items():
        if "." in key:
            # Already qualified — use as-is.
            merged[key] = val
        else:
            # Add qualified version.
            merged[f"{right_alias}.{key}"] = val
            # Add bare version (right wins on collision).
            merged[key] = val

    return merged


def _null_row_for(
    rows: list[dict[str, Any]],
    alias: str,
) -> dict[str, Any]:
    """Build a NULL-value row with the same schema as *rows*.

    Used when an outer join has no matching partner row — all values
    should be NULL (Python ``None``).
    """
    if not rows:
        return {}
    # Collect all unique keys from the first available row.
    null_row: dict[str, Any] = {}
    for key in rows[0]:
        null_row[key] = None
    return null_row


def _test_condition(
    condition: ASTNode | Token,
    merged: dict[str, Any],
) -> bool:
    """Evaluate the ON condition against a merged row."""
    val = eval_expr(condition, merged)
    return bool(val) if val is not None else False
