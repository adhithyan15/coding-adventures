"""Oracle tests for CTE MATERIALIZED / NOT MATERIALIZED hint (SQLite 3.35+).

The optional ``[NOT] MATERIALIZED`` keyword between ``AS`` and the
opening ``(`` of a CTE definition tells SQLite's planner whether to
materialise the CTE result set or inline it.  Mini-sqlite has no
cost-based optimizer, so the hint is parsed and silently ignored —
queries that use it for portability with real SQLite parse and
execute exactly as if the hint weren't there.

Grammar (from ``code/grammars/sql.grammar``)::

    cte_def = NAME [ "(" NAME { "," NAME } ")" ] "AS"
              [ [ "NOT" ] "MATERIALIZED" ]
              "(" query_stmt ")" ;

All assertions compare against real ``sqlite3`` so we know we match
SQLite's exact result rows (not just the parser's acceptance).
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both(sql: str):
    """Return ``(mini_rows, ref_rows)`` for *sql*."""
    m = mini_sqlite.connect(":memory:").execute(sql).fetchall()
    r = sqlite3.connect(":memory:").execute(sql).fetchall()
    return m, r


def _check(sql: str) -> None:
    """Assert mini-sqlite matches real sqlite3 row-for-row."""
    m, r = _both(sql)
    assert m == r, f"SQL: {sql!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Basic acceptance
# ---------------------------------------------------------------------------


class TestMaterializedHintBasic:
    def test_materialized_simple(self) -> None:
        _check("WITH cte AS MATERIALIZED (SELECT 1 AS x) SELECT x FROM cte")

    def test_not_materialized_simple(self) -> None:
        _check("WITH cte AS NOT MATERIALIZED (SELECT 1 AS x) SELECT x FROM cte")

    def test_without_hint_still_works(self) -> None:
        """The hint is optional — plain ``AS (`` must still parse."""
        _check("WITH cte AS (SELECT 1 AS x) SELECT x FROM cte")


# ---------------------------------------------------------------------------
# Hint with column aliases
# ---------------------------------------------------------------------------


class TestMaterializedHintWithColumnAliases:
    def test_with_col_aliases_materialized(self) -> None:
        _check(
            "WITH cte(a, b) AS MATERIALIZED (SELECT 1, 2) "
            "SELECT a + b FROM cte"
        )

    def test_with_col_aliases_not_materialized(self) -> None:
        _check(
            "WITH cte(a, b) AS NOT MATERIALIZED (SELECT 1, 2) "
            "SELECT a + b FROM cte"
        )


# ---------------------------------------------------------------------------
# Hint with recursive CTE
# ---------------------------------------------------------------------------


class TestMaterializedHintWithRecursiveCte:
    def test_recursive_materialized(self) -> None:
        _check(
            "WITH RECURSIVE n(i) AS MATERIALIZED "
            "(SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 5) "
            "SELECT * FROM n"
        )

    def test_recursive_not_materialized(self) -> None:
        _check(
            "WITH RECURSIVE n(i) AS NOT MATERIALIZED "
            "(SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 5) "
            "SELECT * FROM n"
        )


# ---------------------------------------------------------------------------
# Multiple CTEs with mixed hints
# ---------------------------------------------------------------------------


class TestMultipleCtesMixedHints:
    def test_mixed_hints(self) -> None:
        _check(
            "WITH "
            "a AS MATERIALIZED (SELECT 1 AS x), "
            "b AS NOT MATERIALIZED (SELECT 2 AS y), "
            "c AS (SELECT 3 AS z) "
            "SELECT a.x, b.y, c.z FROM a, b, c"
        )


# ---------------------------------------------------------------------------
# Hint with table-backed query
# ---------------------------------------------------------------------------


class TestMaterializedHintWithRealTable:
    def test_materialized_filter_count(self) -> None:
        setup = [
            "CREATE TABLE t (id INTEGER, val INTEGER)",
            "INSERT INTO t VALUES (1, 10), (2, 20), (3, 30), (4, 40)",
        ]
        query = (
            "WITH evens AS MATERIALIZED (SELECT id, val FROM t WHERE val % 20 = 0) "
            "SELECT COUNT(*) FROM evens"
        )
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        for s in setup:
            conn_m.execute(s)
            conn_r.execute(s)
        m = conn_m.execute(query).fetchall()
        r = conn_r.execute(query).fetchall()
        assert m == r == [(2,)]
