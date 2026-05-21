"""Compound queries (UNION / INTERSECT / EXCEPT) as derived tables.

SQLite allows the inner query of a derived table to be any compound
query, not just a plain SELECT.  Before this PR mini-sqlite raised
``derived table must be a plain SELECT, not a set operation`` for
queries like ``SELECT * FROM (SELECT 1 UNION SELECT 2) AS u``.

The fix spans three layers:

1. **AST** (``sql_planner.ast.DerivedTableRef.select``) — widened from
   ``SelectStmt`` to ``SelectStmt | UnionStmt | IntersectStmt | ExceptStmt``.

2. **Adapter** — the rejection check now allows any of the four typed
   query-producing statement forms.

3. **Planner** — a new ``_plan_derived_inner`` helper dispatches by
   statement type so the inner of a derived table can be planned the
   same way as a top-level query.  ``_output_columns`` and
   ``_source_columns`` learn to descend through ``Union`` / ``Intersect``
   / ``Except`` nodes (inheriting column names from the left side, per
   SQLite's documented rule).

The alias is still mandatory — the optional-alias relaxation is a
separate change.

These tests pair every interesting case against reference ``sqlite3``
byte-for-byte.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# UNION as the derived table inner query
# ---------------------------------------------------------------------------


class TestUnionDerivedTable:
    def test_two_value_union(self) -> None:
        _check("SELECT * FROM (SELECT 1 AS x UNION SELECT 2) AS t ORDER BY x")

    def test_union_all_preserves_duplicates(self) -> None:
        _check("SELECT * FROM (SELECT 1 AS x UNION ALL SELECT 1) AS u ORDER BY 1")

    def test_three_way_union(self) -> None:
        _check(
            "SELECT count(*) FROM (SELECT 1 UNION SELECT 2 UNION SELECT 3) AS s"
        )

    def test_union_with_outer_where(self) -> None:
        _check(
            "SELECT x FROM (SELECT 1 AS x UNION SELECT 2 UNION SELECT 3) AS t "
            "WHERE x > 1 ORDER BY x"
        )

    def test_union_with_outer_aggregate(self) -> None:
        _check(
            "SELECT sum(x) FROM (SELECT 10 AS x UNION SELECT 20 UNION SELECT 30) AS t"
        )


# ---------------------------------------------------------------------------
# INTERSECT as the derived table inner query
# ---------------------------------------------------------------------------


class TestIntersectDerivedTable:
    def test_simple_intersect(self) -> None:
        _check(
            "SELECT * FROM (SELECT 1 AS x INTERSECT SELECT 1) AS i ORDER BY x"
        )

    def test_intersect_disjoint(self) -> None:
        # No overlap → empty result.
        _check("SELECT count(*) FROM (SELECT 1 INTERSECT SELECT 2) AS i")

    def test_mixed_intersect_then_union(self) -> None:
        # Set-op chaining inside the derived table.
        _check(
            "SELECT x FROM (SELECT 1 AS x INTERSECT SELECT 1 UNION SELECT 2) "
            "AS s ORDER BY x"
        )


# ---------------------------------------------------------------------------
# EXCEPT as the derived table inner query
# ---------------------------------------------------------------------------


class TestExceptDerivedTable:
    def test_simple_except(self) -> None:
        _check("SELECT * FROM (SELECT 1 AS x EXCEPT SELECT 2) AS e")

    def test_except_removes_overlap(self) -> None:
        _check(
            "SELECT * FROM (SELECT 1 AS x UNION SELECT 2 UNION SELECT 3 EXCEPT SELECT 2) "
            "AS e ORDER BY x"
        )


# ---------------------------------------------------------------------------
# Compound queries in JOIN positions
# ---------------------------------------------------------------------------


class TestCompoundJoin:
    def test_join_left_plain_right_union(self) -> None:
        _check(
            "SELECT a.x, b.y FROM (SELECT 1 AS x) AS a "
            "JOIN (SELECT 1 AS y UNION SELECT 2) AS b ON 1=1 ORDER BY b.y"
        )

    def test_join_both_union(self) -> None:
        _check(
            "SELECT a.x, b.y "
            "FROM (SELECT 1 AS x UNION SELECT 2) AS a "
            "JOIN (SELECT 10 AS y UNION SELECT 20) AS b ON 1=1 "
            "ORDER BY a.x, b.y"
        )


# ---------------------------------------------------------------------------
# Tables provide rows for the inner queries — exercises full pipeline
# ---------------------------------------------------------------------------


class TestUnionDerivedWithRealTables:
    SETUP = [
        "CREATE TABLE odds (n INTEGER)",
        "INSERT INTO odds VALUES (1), (3), (5)",
        "CREATE TABLE evens (n INTEGER)",
        "INSERT INTO evens VALUES (2), (4), (6)",
    ]

    def test_union_of_two_tables_in_derived(self) -> None:
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        for s in self.SETUP:
            conn_m.execute(s)
            conn_r.execute(s)
        m = conn_m.execute(
            "SELECT n FROM (SELECT n FROM odds UNION SELECT n FROM evens) "
            "AS combined ORDER BY n"
        ).fetchall()
        r = conn_r.execute(
            "SELECT n FROM (SELECT n FROM odds UNION SELECT n FROM evens) "
            "AS combined ORDER BY n"
        ).fetchall()
        assert m == r

    def test_count_distinct_via_union(self) -> None:
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        for s in self.SETUP:
            conn_m.execute(s)
            conn_r.execute(s)
        # UNION dedups; UNION ALL keeps duplicates.
        m = conn_m.execute(
            "SELECT count(*) FROM (SELECT n FROM odds UNION ALL SELECT n FROM evens) AS u"
        ).fetchall()
        r = conn_r.execute(
            "SELECT count(*) FROM (SELECT n FROM odds UNION ALL SELECT n FROM evens) AS u"
        ).fetchall()
        assert m == r


# ---------------------------------------------------------------------------
# Regression — plain SELECT in derived table still works
# ---------------------------------------------------------------------------


class TestPlainSelectStillWorks:
    def test_plain_select(self) -> None:
        _check("SELECT * FROM (SELECT 1 AS x) AS t")

    def test_plain_select_with_filter(self) -> None:
        _check("SELECT x FROM (SELECT 1 AS x WHERE 1=1) AS t")
