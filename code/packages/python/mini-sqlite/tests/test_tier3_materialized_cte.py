"""``[NOT] MATERIALIZED`` CTE hints — parse-and-ignore semantics.

SQLite 3.35+ accepts an optional hint on every CTE definition:

    WITH x AS [NOT] MATERIALIZED (SELECT …) …

The hint tells SQLite's cost-based optimizer whether to materialize the
CTE into a temporary table (MATERIALIZED) or inline it like a view (NOT
MATERIALIZED).  Without the hint SQLite chooses based on its own
heuristics.

Mini-sqlite has *no* cost-based optimizer — CTEs are always inlined as
subqueries by the planner — so the hint is purely advisory.  We parse it
to stay byte-compatible with applications that use it for portability
with real SQLite, then silently discard it.

This test file pins the parse-and-ignore contract: every query is run
through both ``mini_sqlite`` and stdlib ``sqlite3`` and must return
identical rows.  A regression that, say, started raising on the
keyword, or worse, that produced *different* rows when the hint is
present versus absent, would be caught here.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = list(mini_sqlite.connect(":memory:").execute(query))
    r = list(sqlite3.connect(":memory:").execute(query))
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


def _check_with_table(setup: list[str], query: str) -> None:
    mc = mini_sqlite.connect(":memory:")
    rc = sqlite3.connect(":memory:")
    for s in setup:
        mc.execute(s)
        rc.execute(s)
    m = list(mc.execute(query))
    r = list(rc.execute(query))
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Single-CTE forms.
# ---------------------------------------------------------------------------


class TestSingleCTE:
    def test_materialized(self) -> None:
        _check("WITH x AS MATERIALIZED (SELECT 1 AS n) SELECT * FROM x")

    def test_not_materialized(self) -> None:
        _check("WITH x AS NOT MATERIALIZED (SELECT 1 AS n) SELECT * FROM x")

    def test_no_hint_baseline(self) -> None:
        _check("WITH x AS (SELECT 1 AS n) SELECT * FROM x")

    def test_materialized_with_column_alias(self) -> None:
        _check("WITH x(a) AS MATERIALIZED (SELECT 42) SELECT a FROM x")

    def test_not_materialized_with_column_alias(self) -> None:
        _check("WITH x(a) AS NOT MATERIALIZED (SELECT 42) SELECT a FROM x")


# ---------------------------------------------------------------------------
# Multiple CTEs in one WITH clause, mixed hints.
# ---------------------------------------------------------------------------


class TestMultipleCTEs:
    def test_both_materialized(self) -> None:
        _check(
            "WITH a AS MATERIALIZED (SELECT 1 AS x),"
            "     b AS MATERIALIZED (SELECT 2 AS y) "
            "SELECT a.x, b.y FROM a, b"
        )

    def test_mixed_hints(self) -> None:
        _check(
            "WITH a AS MATERIALIZED (SELECT 1 AS x),"
            "     b AS NOT MATERIALIZED (SELECT x*2 AS y FROM a) "
            "SELECT * FROM b"
        )

    def test_hint_then_no_hint(self) -> None:
        _check(
            "WITH a AS MATERIALIZED (SELECT 1 AS x),"
            "     b AS (SELECT x+1 AS y FROM a) "
            "SELECT y FROM b"
        )

    def test_no_hint_then_hint(self) -> None:
        _check(
            "WITH a AS (SELECT 10 AS x),"
            "     b AS NOT MATERIALIZED (SELECT x FROM a) "
            "SELECT * FROM b"
        )


# ---------------------------------------------------------------------------
# Recursive CTEs with MATERIALIZED — the hint is parsed but the recursion
# semantics must still work end-to-end.
# ---------------------------------------------------------------------------


class TestRecursiveMaterialized:
    def test_recursive_materialized(self) -> None:
        _check(
            "WITH RECURSIVE c(n) AS MATERIALIZED "
            "(SELECT 1 UNION ALL SELECT n+1 FROM c WHERE n < 5) "
            "SELECT n FROM c"
        )

    def test_recursive_not_materialized(self) -> None:
        _check(
            "WITH RECURSIVE c(n) AS NOT MATERIALIZED "
            "(SELECT 1 UNION ALL SELECT n+1 FROM c WHERE n < 3) "
            "SELECT n FROM c"
        )

    def test_recursive_two_columns_materialized(self) -> None:
        _check(
            "WITH RECURSIVE p(n, sq) AS MATERIALIZED "
            "(SELECT 1, 1 UNION ALL SELECT n+1, (n+1)*(n+1) FROM p WHERE n < 4) "
            "SELECT * FROM p"
        )


# ---------------------------------------------------------------------------
# Equivalence: query-with-hint == query-without-hint.  This is the
# strongest expression of "parse and ignore" — any divergence between
# these pairs would be a bug.
# ---------------------------------------------------------------------------


class TestHintIsAdvisory:
    """The hint never changes results, only the planner's strategy."""

    setup = [
        "CREATE TABLE t(id INTEGER, v INTEGER)",
        "INSERT INTO t VALUES (1, 10), (2, 20), (3, 30), (4, 40)",
    ]

    def _both(self, with_hint: str, without_hint: str) -> None:
        # Both spellings must agree with sqlite3 AND with each other.
        _check_with_table(self.setup, with_hint)
        _check_with_table(self.setup, without_hint)

        mc = mini_sqlite.connect(":memory:")
        for s in self.setup:
            mc.execute(s)
        a = list(mc.execute(with_hint))
        b = list(mc.execute(without_hint))
        assert a == b, f"hint changed results!\n  with: {a}\n  without: {b}"

    def test_filter_then_join(self) -> None:
        self._both(
            "WITH big AS MATERIALIZED (SELECT * FROM t WHERE v >= 20) "
            "SELECT id, v FROM big ORDER BY id",
            "WITH big AS (SELECT * FROM t WHERE v >= 20) "
            "SELECT id, v FROM big ORDER BY id",
        )

    def test_aggregation_inside_cte(self) -> None:
        self._both(
            "WITH agg AS NOT MATERIALIZED (SELECT SUM(v) AS s FROM t) "
            "SELECT s FROM agg",
            "WITH agg AS (SELECT SUM(v) AS s FROM t) "
            "SELECT s FROM agg",
        )

    def test_self_join_via_cte(self) -> None:
        self._both(
            "WITH base AS MATERIALIZED (SELECT id, v FROM t) "
            "SELECT a.id, a.v, b.v FROM base a JOIN base b ON a.id = b.id ORDER BY a.id",
            "WITH base AS (SELECT id, v FROM t) "
            "SELECT a.id, a.v, b.v FROM base a JOIN base b ON a.id = b.id ORDER BY a.id",
        )


# ---------------------------------------------------------------------------
# Set operations inside a CTE body — both with and without the
# MATERIALIZED hint.  This was a known gap until this PR; the parser
# accepted the syntax but the adapter raised
# "CTE body must be a plain SELECT, not a set operation".  PR #3817
# fixed the same gap for *derived tables* (anonymous subqueries in
# FROM); this commit extends the fix to *named* CTEs.
# ---------------------------------------------------------------------------


class TestSetOpInsideCTE:
    def test_union_in_materialized(self) -> None:
        _check(
            "WITH u AS MATERIALIZED "
            "(SELECT 1 AS n UNION ALL SELECT 2 UNION ALL SELECT 3) "
            "SELECT n FROM u ORDER BY n"
        )

    def test_intersect_in_not_materialized(self) -> None:
        _check(
            "WITH i AS NOT MATERIALIZED "
            "(SELECT 1 AS n UNION SELECT 2 INTERSECT SELECT 2) "
            "SELECT n FROM i"
        )

    def test_union_in_plain_cte_no_hint(self) -> None:
        # The fix applies regardless of MATERIALIZED — the original gap
        # affected hint-less CTEs too.
        _check(
            "WITH u AS (SELECT 1 AS n UNION SELECT 2 UNION SELECT 3) "
            "SELECT n FROM u ORDER BY n"
        )

    def test_except_in_cte(self) -> None:
        _check(
            "WITH e AS (SELECT 1 AS n UNION SELECT 2 UNION SELECT 3 EXCEPT SELECT 2) "
            "SELECT n FROM e ORDER BY n"
        )

    def test_union_with_column_aliases(self) -> None:
        # CTE column aliases on a set-op body — applied to the LEFTMOST
        # SelectStmt of the set-op tree (SQLite derives output column
        # names from the leftmost operand).
        _check(
            "WITH u(label) AS (SELECT 'a' UNION ALL SELECT 'b') "
            "SELECT label FROM u ORDER BY label"
        )

    def test_chain_of_union_in_cte(self) -> None:
        _check(
            "WITH u AS ("
            "  SELECT 1 AS n UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4"
            ") SELECT n FROM u WHERE n > 2 ORDER BY n"
        )
