"""Oracle tests: SELECT column aliases without the AS keyword.

SQLite allows column aliases in SELECT items without the ``AS`` keyword::

    SELECT 1 x            -- equivalent to SELECT 1 AS x
    SELECT a + b total    -- equivalent to SELECT a + b AS total
    SELECT x val FROM t   -- equivalent to SELECT x AS val FROM t

Previously mini-sqlite required ``AS`` and raised a parse error on the bare
form.  The fix makes ``AS`` optional in the ``select_item`` grammar rule:
``expr [ "AS" NAME ]`` → ``expr [ [ "AS" ] NAME ]``.

Pattern: every test runs the same SQL against both sqlite3 (reference) and
mini_sqlite, and asserts byte-for-byte identical output.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _ref(sql: str, setup: list[str] | None = None) -> list[tuple]:
    con = sqlite3.connect(":memory:")
    if setup:
        for s in setup:
            con.execute(s)
    return con.execute(sql).fetchall()


def _our(sql: str, setup: list[str] | None = None) -> list[tuple]:
    con = mini_sqlite.connect(":memory:")
    if setup:
        for s in setup:
            con.execute(s)
    return con.execute(sql).fetchall()


class TestBareAliasLiterals:
    """Bare alias on literal projections: SELECT 1 x, SELECT 'a' s, …"""

    def test_integer_literal_bare_alias(self) -> None:
        sql = "SELECT 1 x"
        assert _our(sql) == _ref(sql)

    def test_string_literal_bare_alias(self) -> None:
        sql = "SELECT 'hello' greeting"
        assert _our(sql) == _ref(sql)

    def test_float_literal_bare_alias(self) -> None:
        sql = "SELECT 3.14 pi_approx"
        assert _our(sql) == _ref(sql)

    def test_null_bare_alias(self) -> None:
        sql = "SELECT NULL missing"
        assert _our(sql) == _ref(sql)

    def test_multiple_bare_aliases(self) -> None:
        sql = "SELECT 1 a, 2 b, 3 c"
        assert _our(sql) == _ref(sql)

    def test_mixed_bare_and_as_aliases(self) -> None:
        """AS and bare aliases may be mixed in the same SELECT list."""
        sql = "SELECT 1 AS a, 2 b, 3 AS c"
        assert _our(sql) == _ref(sql)


class TestBareAliasColumns:
    """Bare alias on column references: SELECT x val FROM t."""

    def test_column_bare_alias(self) -> None:
        sql = "SELECT x val FROM t"
        setup = ["CREATE TABLE t (x INT)", "INSERT INTO t VALUES (42)"]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_expression_bare_alias(self) -> None:
        """Expressions (not just names) can have bare aliases."""
        sql = "SELECT x + 1 incremented FROM t"
        setup = ["CREATE TABLE t (x INT)", "INSERT INTO t VALUES (10)"]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_bare_alias_accessible_in_subquery(self) -> None:
        """A bare alias defined in a subquery is accessible in the outer query."""
        sql = "SELECT a.val FROM (SELECT 42 val) a"
        assert _our(sql) == _ref(sql)

    def test_bare_alias_in_derived_table(self) -> None:
        """Bare aliases from derived tables propagate correctly."""
        sql = "SELECT x, y FROM (SELECT 1 x, 2 y)"
        assert _our(sql) == _ref(sql)


class TestBareAliasInSubquery:
    """Bare aliases inside subqueries used as FROM sources."""

    def test_group_concat_with_bare_alias_subquery(self) -> None:
        """group_concat over a subquery that uses bare aliases works."""
        sql = "SELECT group_concat(x, '|') FROM (SELECT 1 x UNION SELECT 2 UNION SELECT 3)"
        assert _our(sql) == _ref(sql)

    def test_count_with_bare_alias_subquery(self) -> None:
        sql = "SELECT count(n) FROM (SELECT 1 n UNION SELECT 2 UNION SELECT 3)"
        assert _our(sql) == _ref(sql)

    def test_window_rank_with_bare_alias_subquery(self) -> None:
        """rank() OVER (ORDER BY x) works when the source uses bare aliases."""
        sql = (
            "SELECT rank() OVER (ORDER BY x) "
            "FROM (SELECT 1 x UNION ALL SELECT 1 UNION ALL SELECT 2) "
            "ORDER BY x"
        )
        assert _our(sql) == _ref(sql)

    def test_lag_with_bare_alias_subquery(self) -> None:
        sql = (
            "SELECT lag(x, 1, 0) OVER (ORDER BY x) "
            "FROM (SELECT 1 x UNION SELECT 2 UNION SELECT 3) "
            "ORDER BY x"
        )
        assert _our(sql) == _ref(sql)


class TestWithAsAliasRegression:
    """Explicit AS alias must continue to work after the grammar change."""

    def test_as_alias_still_works(self) -> None:
        sql = "SELECT 1 AS one, 2 AS two"
        assert _our(sql) == _ref(sql)

    def test_as_alias_on_column(self) -> None:
        sql = "SELECT x AS val FROM t"
        setup = ["CREATE TABLE t (x INT)", "INSERT INTO t VALUES (7)"]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_no_alias(self) -> None:
        """A SELECT item with no alias at all must still work."""
        sql = "SELECT 1, 'hello', NULL"
        assert _our(sql) == _ref(sql)
