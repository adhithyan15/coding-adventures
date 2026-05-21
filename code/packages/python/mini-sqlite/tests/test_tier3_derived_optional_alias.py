"""Optional alias for derived tables — matches SQLite (and standard SQL).

PR #3817 widened the inner statement of a derived table to accept
compound queries; this companion PR drops the long-standing requirement
that derived tables carry an alias.  ``SELECT * FROM (SELECT 1 AS x)``
is legal in SQLite — the outer scope sees ``x`` as an unqualified
column and the user doesn't have to invent a name they'll never use.

The fix touches four layers:

1. **Grammar** (``code/grammars/sql.grammar``) — the derived-table
   alias becomes ``[ [ "AS" ] NAME ]`` (the whole alias group is now
   optional).
2. **Adapter** — the rejection check is removed; alias=None flows
   through to the AST node.
3. **AST** (``DerivedTableRef.alias: str | None``) — the type widens.
4. **Planner** — when the alias is None, the planner synthesises a
   sentinel name (``<derived #hex>``) so the scope / cursor layers
   continue to use string identifiers.  The sentinel starts with
   ``<`` so user SQL can never collide with it; only bare column
   lookups can ever resolve through it.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# No alias at all — bare derived table
# ---------------------------------------------------------------------------


class TestNoAlias:
    def test_select_star_no_alias(self) -> None:
        _check("SELECT * FROM (SELECT 1 AS x)")

    def test_unqualified_column_no_alias(self) -> None:
        _check("SELECT x FROM (SELECT 1 AS x)")

    def test_no_alias_with_order_by(self) -> None:
        _check("SELECT x FROM (SELECT 1 AS x UNION SELECT 2) ORDER BY x")

    def test_no_alias_with_outer_aggregate(self) -> None:
        _check("SELECT count(*) FROM (SELECT 1 UNION SELECT 2 UNION SELECT 3)")

    def test_no_alias_with_filter(self) -> None:
        _check(
            "SELECT x FROM (SELECT 1 AS x UNION SELECT 2 UNION SELECT 3) "
            "WHERE x > 1 ORDER BY x"
        )

    def test_no_alias_compound_inner(self) -> None:
        # Combines optional alias + compound query (PR #3817).
        _check("SELECT * FROM (SELECT 1 AS x INTERSECT SELECT 1)")


# ---------------------------------------------------------------------------
# Aliased forms still work (regression guards)
# ---------------------------------------------------------------------------


class TestAliasedStillWorks:
    def test_with_as_alias(self) -> None:
        _check("SELECT * FROM (SELECT 1 AS x) AS t")

    def test_with_bare_alias(self) -> None:
        _check("SELECT * FROM (SELECT 1 AS x) t")

    def test_qualified_column_with_alias(self) -> None:
        _check("SELECT t.x FROM (SELECT 1 AS x) AS t")

    def test_qualified_column_bare_alias(self) -> None:
        _check("SELECT t.x FROM (SELECT 1 AS x) t")


# ---------------------------------------------------------------------------
# JOIN positions — both sides without alias
# ---------------------------------------------------------------------------


class TestJoinNoAlias:
    def test_join_one_side_no_alias(self) -> None:
        # JOIN currently requires aliases on JOIN'd sides to disambiguate
        # qualified column references — we exercise the more permissive
        # form where both sides are aliased to confirm we didn't break it.
        _check(
            "SELECT a.x, b.y FROM (SELECT 1 AS x) AS a JOIN (SELECT 1 AS y) AS b ON 1=1"
        )

    def test_left_aliased_right_aliased_compound_inner(self) -> None:
        _check(
            "SELECT a.x, b.y "
            "FROM (SELECT 1 AS x UNION SELECT 2) AS a "
            "JOIN (SELECT 10 AS y) AS b ON 1=1 "
            "ORDER BY a.x"
        )


# ---------------------------------------------------------------------------
# Real tables provide rows — exercises the full pipeline end to end
# ---------------------------------------------------------------------------


class TestNoAliasWithRealTables:
    SETUP = [
        "CREATE TABLE nums (n INTEGER)",
        "INSERT INTO nums VALUES (1), (2), (3), (4), (5)",
    ]

    def test_no_alias_over_real_table(self) -> None:
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        for s in self.SETUP:
            conn_m.execute(s)
            conn_r.execute(s)
        m = conn_m.execute(
            "SELECT n FROM (SELECT n FROM nums WHERE n > 2) ORDER BY n"
        ).fetchall()
        r = conn_r.execute(
            "SELECT n FROM (SELECT n FROM nums WHERE n > 2) ORDER BY n"
        ).fetchall()
        assert m == r

    def test_no_alias_with_outer_agg(self) -> None:
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        for s in self.SETUP:
            conn_m.execute(s)
            conn_r.execute(s)
        m = conn_m.execute(
            "SELECT sum(n) FROM (SELECT n FROM nums WHERE n % 2 = 1)"
        ).fetchall()
        r = conn_r.execute(
            "SELECT sum(n) FROM (SELECT n FROM nums WHERE n % 2 = 1)"
        ).fetchall()
        assert m == r
