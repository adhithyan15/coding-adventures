"""Oracle tests for positional ``ORDER BY N`` with ``SELECT *``.

Before the fix in ``sql-planner``'s ``_resolve_order_key``, the planner
rejected positional ORDER BY indices greater than the number of
*unexpanded* SELECT items.  For ``SELECT *`` the only SELECT item is a
Wildcard, so ``ORDER BY 2`` (or any N > 1) would fall through to the
column-name resolution path, set ``column=""``, and the VM's
``columns.index("")`` would raise ``ValueError: tuple.index(x): x not in
tuple``::

    SELECT * FROM t ORDER BY 2
    -- before: InternalError: ValueError: tuple.index(x): x not in tuple
    -- now:    matches SQLite

The planner now accepts ``ORDER BY N`` for any N ≥ 1 when at least one
SELECT item is a Wildcard, setting ``positional_index = N-1`` directly
so the VM uses position-based ``row[N-1]`` lookup.  Out-of-range
indices error out at runtime (matching SQLite for this corner case).

All assertions compare against real ``sqlite3``.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(setup: list[str], query: str) -> None:
    conn_m = mini_sqlite.connect(":memory:")
    conn_r = sqlite3.connect(":memory:")
    for s in setup:
        conn_m.execute(s)
        conn_r.execute(s)
    m = conn_m.execute(query).fetchall()
    r = conn_r.execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref: {r}"


# ---------------------------------------------------------------------------
# SELECT * ORDER BY N — the headline fix
# ---------------------------------------------------------------------------


class TestSelectStarOrderByPositional:
    SETUP = [
        "CREATE TABLE t (a INTEGER, b INTEGER, c INTEGER)",
        "INSERT INTO t VALUES (3, 10, 100), (1, 20, 200), (2, 30, 300)",
    ]

    def test_order_by_1(self) -> None:
        _check(self.SETUP, "SELECT * FROM t ORDER BY 1")

    def test_order_by_2(self) -> None:
        _check(self.SETUP, "SELECT * FROM t ORDER BY 2")

    def test_order_by_3(self) -> None:
        _check(self.SETUP, "SELECT * FROM t ORDER BY 3")

    def test_order_by_desc(self) -> None:
        _check(self.SETUP, "SELECT * FROM t ORDER BY 2 DESC")

    def test_order_by_with_nulls(self) -> None:
        # SQLite default NULL ordering: ASC → NULLs first, DESC → NULLs last.
        setup = [
            "CREATE TABLE u (a INTEGER, b INTEGER)",
            "INSERT INTO u VALUES (1, 10), (2, NULL), (3, 20)",
        ]
        _check(setup, "SELECT * FROM u ORDER BY 2")

    def test_multiple_keys(self) -> None:
        setup = [
            "CREATE TABLE m (a INTEGER, b INTEGER, c INTEGER)",
            "INSERT INTO m VALUES (1, 2, 30), (1, 1, 40), (2, 1, 10), (2, 2, 20)",
        ]
        _check(setup, "SELECT * FROM m ORDER BY 1, 2")

    def test_multiple_keys_mixed_direction(self) -> None:
        setup = [
            "CREATE TABLE m (a INTEGER, b INTEGER, c INTEGER)",
            "INSERT INTO m VALUES (1, 2, 30), (1, 1, 40), (2, 1, 10), (2, 2, 20)",
        ]
        _check(setup, "SELECT * FROM m ORDER BY 1 ASC, 2 DESC")


# ---------------------------------------------------------------------------
# SELECT * + ORDER BY positional across joins
# ---------------------------------------------------------------------------


class TestSelectStarOrderByPositionalJoin:
    def test_cross_join_order_by_first(self) -> None:
        setup = [
            "CREATE TABLE a (x INTEGER)",
            "CREATE TABLE b (y INTEGER)",
            "INSERT INTO a VALUES (3), (1), (2)",
            "INSERT INTO b VALUES (100)",
        ]
        _check(setup, "SELECT * FROM a, b ORDER BY 1")

    def test_inner_join_order_by_second_table_column(self) -> None:
        setup = [
            "CREATE TABLE a (id INTEGER, name TEXT)",
            "CREATE TABLE b (id INTEGER, val INTEGER)",
            "INSERT INTO a VALUES (1, 'x'), (2, 'y')",
            "INSERT INTO b VALUES (1, 30), (2, 10)",
        ]
        _check(setup, "SELECT * FROM a JOIN b ON a.id = b.id ORDER BY 4")


# ---------------------------------------------------------------------------
# Regression — non-wildcard SELECT with positional ORDER BY still works
# ---------------------------------------------------------------------------


class TestExplicitProjectionPositionalNoRegression:
    SETUP = [
        "CREATE TABLE t (a INTEGER, b INTEGER)",
        "INSERT INTO t VALUES (3, 10), (1, 20), (2, 30)",
    ]

    def test_explicit_order_by_1(self) -> None:
        _check(self.SETUP, "SELECT a, b FROM t ORDER BY 1")

    def test_explicit_order_by_2(self) -> None:
        _check(self.SETUP, "SELECT a, b FROM t ORDER BY 2")

    def test_swapped_order_by_2(self) -> None:
        _check(self.SETUP, "SELECT b, a FROM t ORDER BY 2")
