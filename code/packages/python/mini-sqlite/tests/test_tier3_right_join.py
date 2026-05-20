"""Oracle tests for RIGHT [OUTER] JOIN column ordering with SELECT *.

mini-sqlite implements RIGHT JOIN by swapping the left and right sides
and emitting a LEFT JOIN.  That works correctly for explicit column
projections (where the Project node above the join controls output
order by name), but ``SELECT *`` iterates ``ctx.alias_to_cursor`` in
insertion order — which the swap reverses, putting RIGHT columns
before LEFT columns and diverging from SQLite.

The fix in ``sql_codegen/compiler.py``'s RIGHT JOIN branch wraps the
inner ``body`` closure so that ``alias_to_cursor`` is reordered
(original-left first, original-right second) for the duration of each
body invocation, restoring left→right column order in SELECT * output.

Coverage:
- Matched and unmatched-right rows are both correct.
- LEFT JOIN regression guard — must still work.
- FULL OUTER JOIN regression guard — must still work.
- RIGHT JOIN with explicit projection — was already correct, included as
  a sanity test.
- RIGHT JOIN with derived right side and CTE left side.

Note: SQLite normalizes column-qualified ORDER BY against ``SELECT *``
in a way mini-sqlite doesn't fully replicate (latent bug, separate
from this fix).  Tests therefore either skip ORDER BY or sort the
result rows in Python before comparing.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both(setup: list[str], query: str):
    """Apply *setup* on both engines then run *query* and return rows."""
    conn_m = mini_sqlite.connect(":memory:")
    conn_r = sqlite3.connect(":memory:")
    for s in setup:
        conn_m.execute(s)
        conn_r.execute(s)
    m = conn_m.execute(query).fetchall()
    r = conn_r.execute(query).fetchall()
    return m, r


def _check_unordered(setup: list[str], query: str) -> None:
    """Compare row multisets — independent of order."""
    m, r = _both(setup, query)
    # Use repr-based sort so heterogeneous types and None work.
    assert sorted(m, key=repr) == sorted(r, key=repr), (
        f"SQL: {query!r}\n  mini (sorted): {sorted(m, key=repr)}\n"
        f"  ref  (sorted): {sorted(r, key=repr)}"
    )


def _check_ordered(setup: list[str], query: str) -> None:
    """Compare row sequences exactly (relies on the query being deterministic)."""
    m, r = _both(setup, query)
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref: {r}"


# ---------------------------------------------------------------------------
# RIGHT JOIN — SELECT * column ordering
# ---------------------------------------------------------------------------


class TestRightJoinSelectStarColumnOrder:
    def test_partial_match_right_side(self) -> None:
        # Right has a row that left doesn't match — must NULL-pad the LEFT
        # columns and emit them BEFORE the right columns (FROM-order).
        _check_unordered(
            [
                "CREATE TABLE a (id INTEGER, x TEXT)",
                "CREATE TABLE b (id INTEGER, y TEXT)",
                "INSERT INTO a VALUES (1, 'a1'), (2, 'a2')",
                "INSERT INTO b VALUES (2, 'b2'), (3, 'b3')",
            ],
            "SELECT * FROM a RIGHT JOIN b ON a.id = b.id",
        )

    def test_all_right_matched(self) -> None:
        _check_unordered(
            [
                "CREATE TABLE a (id INTEGER, x TEXT)",
                "CREATE TABLE b (id INTEGER, y TEXT)",
                "INSERT INTO a VALUES (1, 'a1'), (2, 'a2')",
                "INSERT INTO b VALUES (1, 'b1'), (2, 'b2')",
            ],
            "SELECT * FROM a RIGHT JOIN b ON a.id = b.id",
        )

    def test_no_right_match(self) -> None:
        # b has rows that none of a matches — every result row has left = NULL.
        _check_unordered(
            [
                "CREATE TABLE a (id INTEGER, x TEXT)",
                "CREATE TABLE b (id INTEGER, y TEXT)",
                "INSERT INTO a VALUES (10, 'a10')",
                "INSERT INTO b VALUES (1, 'b1'), (2, 'b2')",
            ],
            "SELECT * FROM a RIGHT JOIN b ON a.id = b.id",
        )

    def test_empty_left(self) -> None:
        # a is empty — every b row is null-padded on the left side.
        _check_unordered(
            [
                "CREATE TABLE a (id INTEGER, x TEXT)",
                "CREATE TABLE b (id INTEGER, y TEXT)",
                "INSERT INTO b VALUES (1, 'b1'), (2, 'b2')",
            ],
            "SELECT * FROM a RIGHT JOIN b ON a.id = b.id",
        )


# ---------------------------------------------------------------------------
# RIGHT JOIN — explicit column projection (sanity)
# ---------------------------------------------------------------------------


class TestRightJoinExplicitProjection:
    def test_named_columns_preserve_order(self) -> None:
        _check_unordered(
            [
                "CREATE TABLE a (id INTEGER, x TEXT)",
                "CREATE TABLE b (id INTEGER, y TEXT)",
                "INSERT INTO a VALUES (1, 'a1')",
                "INSERT INTO b VALUES (1, 'b1'), (2, 'b2')",
            ],
            "SELECT a.id, b.id, a.x, b.y FROM a RIGHT JOIN b ON a.id = b.id",
        )

    def test_only_left_columns(self) -> None:
        _check_unordered(
            [
                "CREATE TABLE a (id INTEGER, x TEXT)",
                "CREATE TABLE b (id INTEGER, y TEXT)",
                "INSERT INTO a VALUES (1, 'a1')",
                "INSERT INTO b VALUES (1, 'b1'), (2, 'b2')",
            ],
            "SELECT a.x FROM a RIGHT JOIN b ON a.id = b.id",
        )

    def test_only_right_columns(self) -> None:
        _check_unordered(
            [
                "CREATE TABLE a (id INTEGER, x TEXT)",
                "CREATE TABLE b (id INTEGER, y TEXT)",
                "INSERT INTO a VALUES (1, 'a1')",
                "INSERT INTO b VALUES (1, 'b1'), (2, 'b2')",
            ],
            "SELECT b.y FROM a RIGHT JOIN b ON a.id = b.id",
        )


# ---------------------------------------------------------------------------
# Regression — LEFT JOIN + FULL OUTER JOIN must still work
# ---------------------------------------------------------------------------


class TestLeftAndFullJoinNoRegression:
    def test_left_join_select_star(self) -> None:
        _check_ordered(
            [
                "CREATE TABLE a (id INTEGER, x TEXT)",
                "CREATE TABLE b (id INTEGER, y TEXT)",
                "INSERT INTO a VALUES (1, 'a1'), (2, 'a2')",
                "INSERT INTO b VALUES (2, 'b2')",
            ],
            "SELECT * FROM a LEFT JOIN b ON a.id = b.id",
        )

    def test_full_outer_join_select_star(self) -> None:
        _check_unordered(
            [
                "CREATE TABLE a (id INTEGER, x TEXT)",
                "CREATE TABLE b (id INTEGER, y TEXT)",
                "INSERT INTO a VALUES (1, 'a1'), (2, 'a2')",
                "INSERT INTO b VALUES (2, 'b2'), (3, 'b3')",
            ],
            "SELECT * FROM a FULL OUTER JOIN b ON a.id = b.id",
        )

    def test_inner_join_select_star(self) -> None:
        _check_unordered(
            [
                "CREATE TABLE a (id INTEGER, x TEXT)",
                "CREATE TABLE b (id INTEGER, y TEXT)",
                "INSERT INTO a VALUES (1, 'a1'), (2, 'a2')",
                "INSERT INTO b VALUES (2, 'b2')",
            ],
            "SELECT * FROM a JOIN b ON a.id = b.id",
        )


# ---------------------------------------------------------------------------
# RIGHT JOIN with derived tables and CTEs
# ---------------------------------------------------------------------------


class TestRightJoinDerivedAndCte:
    def test_derived_left_table_right_join(self) -> None:
        _check_unordered(
            [
                "CREATE TABLE b (id INTEGER, y TEXT)",
                "INSERT INTO b VALUES (1, 'b1'), (2, 'b2')",
            ],
            "SELECT * FROM (SELECT 1 AS id, 'a1' AS x) a "
            "RIGHT JOIN b ON a.id = b.id",
        )

    def test_cte_left_table_right_join(self) -> None:
        _check_unordered(
            [
                "CREATE TABLE b (id INTEGER, y TEXT)",
                "INSERT INTO b VALUES (1, 'b1'), (2, 'b2')",
            ],
            "WITH a(id, x) AS (SELECT 1, 'a1') "
            "SELECT * FROM a RIGHT JOIN b ON a.id = b.id",
        )
