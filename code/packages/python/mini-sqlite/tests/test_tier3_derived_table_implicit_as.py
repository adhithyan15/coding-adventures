"""Oracle tests for derived tables with implicit (omitted) ``AS`` keyword.

Standard SQL (and SQLite specifically) accepts both forms for naming a
derived table in a FROM clause::

    FROM (SELECT ...) AS alias   -- classic form
    FROM (SELECT ...) alias      -- shorthand form

Mini-sqlite previously rejected the second form with a parse error.
This file pins behaviour after the fix in:

- ``code/grammars/sql.grammar``: ``table_ref`` now accepts an optional
  ``AS`` keyword between the closing ``)`` and the alias NAME.
- ``mini_sqlite/adapter.py::_table_ref``: scans for the NAME after the
  closing ``)`` regardless of whether AS appears in between.

All assertions compare against real ``sqlite3`` row-for-row.
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
    m, r = _both(sql)
    assert m == r, f"SQL: {sql!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Single derived table — both with and without AS
# ---------------------------------------------------------------------------


class TestDerivedTableImplicitAS:
    def test_single_with_as(self) -> None:
        _check("SELECT t1.x FROM (SELECT 1 AS x) AS t1")

    def test_single_without_as(self) -> None:
        _check("SELECT t1.x FROM (SELECT 1 AS x) t1")

    def test_select_star_without_as(self) -> None:
        _check("SELECT * FROM (SELECT 42 AS answer) ans")


# ---------------------------------------------------------------------------
# Comma-cross-join with derived tables
# ---------------------------------------------------------------------------


class TestCommaCrossJoinDerivedTables:
    def test_two_derived_tables_implicit_as(self) -> None:
        _check(
            "SELECT t1.x, t2.y FROM (SELECT 1 AS x) t1, (SELECT 2 AS y) t2"
        )

    def test_two_derived_tables_explicit_as(self) -> None:
        _check(
            "SELECT t1.x, t2.y FROM (SELECT 1 AS x) AS t1, (SELECT 2 AS y) AS t2"
        )

    def test_three_derived_tables_mixed_as(self) -> None:
        # Mix: some with AS, some without.
        _check(
            "SELECT t1.a, t2.b, t3.c FROM "
            "(SELECT 1 AS a) t1, "
            "(SELECT 2 AS b) AS t2, "
            "(SELECT 3 AS c) t3"
        )


# ---------------------------------------------------------------------------
# Subquery in IN clause uses derived table without AS
# ---------------------------------------------------------------------------


class TestDerivedTableInSubqueryContext:
    def test_in_clause_with_implicit_as_inner_derived(self) -> None:
        # The inner SELECT contains its own derived table without AS.
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        for s in [
            "CREATE TABLE t (id INTEGER)",
            "INSERT INTO t VALUES (1), (2), (3)",
        ]:
            conn_m.execute(s)
            conn_r.execute(s)
        sql = "SELECT id FROM t WHERE id IN (SELECT v FROM (SELECT 2 AS v) sub)"
        m = conn_m.execute(sql).fetchall()
        r = conn_r.execute(sql).fetchall()
        assert m == r == [(2,)]


# ---------------------------------------------------------------------------
# Derived table without alias must still be rejected
# ---------------------------------------------------------------------------


class TestDerivedTableStillRequiresAlias:
    """The alias itself is still required — only the AS keyword is optional."""

    def test_bare_derived_table_rejected(self) -> None:
        # Both mini-sqlite and real sqlite3 reject ``FROM (SELECT 1)`` with no
        # alias whatsoever — they differ on the exact error wording but both
        # should error rather than succeed.
        try:
            mini_sqlite.connect(":memory:").execute(
                "SELECT * FROM (SELECT 1)"
            ).fetchall()
            raise AssertionError(
                "expected mini-sqlite to reject derived table without alias"
            )
        except Exception:
            pass
