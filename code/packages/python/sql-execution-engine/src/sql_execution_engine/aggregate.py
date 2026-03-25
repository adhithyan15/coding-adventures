"""Aggregate functions — COUNT, SUM, AVG, MIN, MAX.

Aggregate functions operate on a *group* of rows (all rows sharing the
same GROUP BY key values) and return a single summary value.

SQL Aggregation Model
---------------------

The aggregation pipeline works in three phases:

1. **Group** — partition all rows into groups by their GROUP BY columns.
   If there is no GROUP BY clause, all rows form a single group.

2. **Aggregate** — for each group, compute the aggregate function value
   across all rows in that group.

3. **Filter** — apply the HAVING clause to each group's aggregated result.

Aggregate Functions Supported
------------------------------

.. list-table::
   :widths: 20 80
   :header-rows: 1

   * - Function
     - Description
   * - ``COUNT(*)``
     - Number of rows in the group.
   * - ``COUNT(col)``
     - Number of non-NULL values in *col*.
   * - ``SUM(expr)``
     - Sum of non-NULL values. Returns NULL if all are NULL.
   * - ``AVG(expr)``
     - Arithmetic mean of non-NULL values. NULL if all are NULL.
   * - ``MIN(expr)``
     - Smallest non-NULL value. NULL if all are NULL.
   * - ``MAX(expr)``
     - Largest non-NULL value. NULL if all are NULL.

NULL Handling
-------------

SQL aggregate functions (except COUNT(*)) ignore NULL values. This is
called "NULL-skipping" semantics. If *all* input values are NULL, SUM,
AVG, MIN, and MAX return NULL (not 0), while COUNT returns 0.

Usage Pattern (used by executor.py)
-------------------------------------

.. code-block:: python

    from sql_execution_engine.aggregate import compute_aggregates

    groups = {("Engineering",): [row1, row2], ("Marketing",): [row3]}
    agg_specs = [("COUNT", "*"), ("SUM", "salary")]

    for key, rows in groups.items():
        result = compute_aggregates(rows, agg_specs)
        # result == {"_agg_COUNT(*)": 2, "_agg_SUM(salary)": 165000}
"""

from __future__ import annotations

from typing import Any


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def compute_aggregates(
    rows: list[dict[str, Any]],
    agg_specs: list[tuple[str, str]],
) -> dict[str, Any]:
    """Compute aggregate values for a group of rows.

    Args:
        rows: All rows belonging to the current group.
        agg_specs: A list of ``(function_name, argument)`` pairs.  The
                   argument is ``"*"`` for COUNT(*) or a column name for
                   all other functions.

    Returns:
        A dict mapping aggregate keys (``"_agg_COUNT(*)"`` etc.) to their
        computed values.  These keys are stored back into the representative
        row so that the SELECT projection and HAVING clause can reference them.
    """
    result: dict[str, Any] = {}
    for func_name, arg in agg_specs:
        key = f"_agg_{func_name.upper()}({arg})"
        result[key] = _compute_one(rows, func_name.upper(), arg)
    return result


# ---------------------------------------------------------------------------
# Internal implementation
# ---------------------------------------------------------------------------


def _compute_one(rows: list[dict[str, Any]], func: str, arg: str) -> Any:
    """Compute one aggregate function over a list of rows.

    Args:
        rows: The rows in the current group.
        func: Uppercase function name: ``"COUNT"``, ``"SUM"``, ``"AVG"``,
              ``"MIN"``, or ``"MAX"``.
        arg: ``"*"`` for COUNT(*), or a column name for all others.

    Returns:
        The aggregate value, or ``None`` if no non-NULL values exist.
    """
    if func == "COUNT":
        if arg == "*":
            return len(rows)
        # COUNT(col) — count non-NULL values
        return sum(1 for row in rows if _get_val(row, arg) is not None)

    # All other aggregates ignore NULL values.
    values = [_get_val(row, arg) for row in rows if _get_val(row, arg) is not None]

    if not values:
        return None  # all NULLs → NULL result

    if func == "SUM":
        return sum(values)

    if func == "AVG":
        return sum(values) / len(values)

    if func == "MIN":
        return min(values)  # type: ignore[type-var]

    if func == "MAX":
        return max(values)  # type: ignore[type-var]

    return None


def _get_val(row: dict[str, Any], col: str) -> Any:
    """Get a column value from a row, trying bare and qualified names."""
    if col in row:
        return row[col]
    # Try case-insensitive match
    col_lower = col.lower()
    for key, val in row.items():
        if key.lower() == col_lower:
            return val
    return None
