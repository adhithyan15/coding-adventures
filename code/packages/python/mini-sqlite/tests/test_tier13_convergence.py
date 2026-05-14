"""Tier 13 SQL convergence tests — oracle-grade comparison against real sqlite3.

Covers the specific gaps closed in this PR:

1. ``x % 0`` returns NULL (SQLite behaviour) instead of raising an exception.
2. Doubled-quote ``''`` escape inside single-quoted string literals.
3. ``COUNT(DISTINCT col)`` / ``SUM(DISTINCT col)`` aggregate deduplication.
4. ``REPLACE(str, from, to)`` scalar function — REPLACE is a keyword so the
   grammar previously rejected it in function-call position.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------

def _both(sql: str, *, setup: list[str] | None = None) -> tuple[list, list]:
    """Run *sql* against both real sqlite3 and mini-sqlite; return (ref, got)."""
    ref_con = sqlite3.connect(":memory:")
    got_con = mini_sqlite.connect(":memory:")
    for s in (setup or []):
        ref_con.execute(s)
        got_con.execute(s)
    ref_rows = ref_con.execute(sql).fetchall()
    got_rows = got_con.execute(sql).fetchall()
    return ref_rows, got_rows


# ---------------------------------------------------------------------------
# 1. Modulo by zero → NULL
# ---------------------------------------------------------------------------

class TestModuloByZero:
    """SQLite returns NULL for ``x % 0``; Python raises ZeroDivisionError."""

    def test_integer_mod_zero_returns_null(self) -> None:
        ref, got = _both("SELECT 7 % 0")
        assert got == ref == [(None,)]

    def test_zero_mod_zero_returns_null(self) -> None:
        ref, got = _both("SELECT 0 % 0")
        assert got == ref == [(None,)]

    def test_negative_mod_zero_returns_null(self) -> None:
        ref, got = _both("SELECT -5 % 0")
        assert got == ref == [(None,)]

    def test_mod_nonzero_still_works(self) -> None:
        ref, got = _both("SELECT 17 % 5")
        assert got == ref == [(2,)]

    def test_mod_by_zero_in_where_keeps_row(self) -> None:
        """NULL in WHERE predicate eliminates the row (three-valued logic)."""
        setup = [
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (5)",
            "INSERT INTO t VALUES (10)",
        ]
        ref, got = _both("SELECT x FROM t WHERE x % 0 = 0", setup=setup)
        assert got == ref == []

    def test_mod_by_zero_column_expression(self) -> None:
        """x % 0 inside a SELECT list → NULL per row."""
        setup = [
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (3)",
            "INSERT INTO t VALUES (9)",
        ]
        ref, got = _both("SELECT x % 0 FROM t", setup=setup)
        assert got == ref


# ---------------------------------------------------------------------------
# 2. Doubled-quote '' escape in string literals
# ---------------------------------------------------------------------------

class TestDoubledQuoteEscape:
    """ANSI SQL: two consecutive single quotes inside a string literal stand
    for one literal single-quote character."""

    def test_apostrophe_in_string(self) -> None:
        ref, got = _both("SELECT 'O''Brien'")
        assert got == ref == [("O'Brien",)]

    def test_leading_apostrophe(self) -> None:
        ref, got = _both("SELECT '''hello'")
        assert got == ref == [("'hello",)]

    def test_trailing_apostrophe(self) -> None:
        ref, got = _both("SELECT 'hello'''")
        assert got == ref == [("hello'",)]

    def test_only_apostrophe(self) -> None:
        """A string consisting of nothing but an apostrophe: ''''."""
        ref, got = _both("SELECT ''''")
        assert got == ref == [("'",)]

    def test_multiple_apostrophes(self) -> None:
        ref, got = _both("SELECT 'it''s a dog''s life'")
        assert got == ref == [("it's a dog's life",)]

    def test_escaped_quote_in_insert_and_select(self) -> None:
        setup = [
            "CREATE TABLE names (id INTEGER, name TEXT)",
            "INSERT INTO names VALUES (1, 'O''Brien')",
            "INSERT INTO names VALUES (2, 'it''s fine')",
        ]
        ref, got = _both("SELECT id, name FROM names ORDER BY id", setup=setup)
        assert got == ref

    def test_where_clause_with_escaped_quote(self) -> None:
        setup = [
            "CREATE TABLE names (id INTEGER, name TEXT)",
            "INSERT INTO names VALUES (1, 'O''Brien')",
            "INSERT INTO names VALUES (2, 'Smith')",
        ]
        ref, got = _both("SELECT id FROM names WHERE name = 'O''Brien'", setup=setup)
        assert got == ref == [(1,)]


# ---------------------------------------------------------------------------
# 3. COUNT(DISTINCT col) / aggregate DISTINCT
# ---------------------------------------------------------------------------

class TestCountDistinct:
    """COUNT(DISTINCT col) must deduplicate values before counting."""

    def test_basic_count_distinct(self) -> None:
        setup = [
            "CREATE TABLE sales (region TEXT, amount INTEGER)",
            "INSERT INTO sales VALUES ('A', 10)",
            "INSERT INTO sales VALUES ('A', 20)",
            "INSERT INTO sales VALUES ('B', 30)",
        ]
        ref, got = _both("SELECT COUNT(DISTINCT region) FROM sales", setup=setup)
        assert got == ref == [(2,)]

    def test_count_distinct_all_same(self) -> None:
        setup = [
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (5)",
            "INSERT INTO t VALUES (5)",
            "INSERT INTO t VALUES (5)",
        ]
        ref, got = _both("SELECT COUNT(DISTINCT x) FROM t", setup=setup)
        assert got == ref == [(1,)]

    def test_count_distinct_all_unique(self) -> None:
        setup = [
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (1)",
            "INSERT INTO t VALUES (2)",
            "INSERT INTO t VALUES (3)",
        ]
        ref, got = _both("SELECT COUNT(DISTINCT x) FROM t", setup=setup)
        assert got == ref == [(3,)]

    def test_count_distinct_ignores_null(self) -> None:
        """COUNT(DISTINCT col) does not count NULL values."""
        setup = [
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (1)",
            "INSERT INTO t VALUES (NULL)",
            "INSERT INTO t VALUES (1)",
            "INSERT INTO t VALUES (NULL)",
        ]
        ref, got = _both("SELECT COUNT(DISTINCT x) FROM t", setup=setup)
        assert got == ref == [(1,)]

    def test_count_distinct_empty_table(self) -> None:
        setup = ["CREATE TABLE t (x INTEGER)"]
        ref, got = _both("SELECT COUNT(DISTINCT x) FROM t", setup=setup)
        assert got == ref == [(0,)]

    def test_count_distinct_vs_count_all(self) -> None:
        """COUNT(DISTINCT) < COUNT(*) when duplicates exist."""
        setup = [
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (1)",
            "INSERT INTO t VALUES (1)",
            "INSERT INTO t VALUES (2)",
        ]
        ref_c, got_c = _both("SELECT COUNT(*) FROM t", setup=setup)
        ref_d, got_d = _both("SELECT COUNT(DISTINCT x) FROM t", setup=setup)
        assert got_c == ref_c == [(3,)]
        assert got_d == ref_d == [(2,)]


class TestSumDistinct:
    """SUM(DISTINCT col) sums each distinct value once."""

    def test_sum_distinct_basic(self) -> None:
        setup = [
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (10)",
            "INSERT INTO t VALUES (10)",
            "INSERT INTO t VALUES (20)",
        ]
        ref, got = _both("SELECT SUM(DISTINCT x) FROM t", setup=setup)
        assert got == ref == [(30,)]

    def test_sum_distinct_all_unique(self) -> None:
        setup = [
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (1)",
            "INSERT INTO t VALUES (2)",
            "INSERT INTO t VALUES (3)",
        ]
        ref, got = _both("SELECT SUM(DISTINCT x) FROM t", setup=setup)
        assert got == ref == [(6,)]


class TestCountDistinctWithGroupBy:
    """COUNT(DISTINCT) inside GROUP BY aggregates correctly per group."""

    def test_count_distinct_per_group(self) -> None:
        setup = [
            "CREATE TABLE orders (category TEXT, product TEXT)",
            "INSERT INTO orders VALUES ('A', 'p1')",
            "INSERT INTO orders VALUES ('A', 'p1')",
            "INSERT INTO orders VALUES ('A', 'p2')",
            "INSERT INTO orders VALUES ('B', 'p3')",
        ]
        sql = (
            "SELECT category, COUNT(DISTINCT product) FROM orders"
            " GROUP BY category ORDER BY category"
        )
        ref, got = _both(sql, setup=setup)
        assert got == ref


# ---------------------------------------------------------------------------
# 4. REPLACE() as a scalar function name
# ---------------------------------------------------------------------------

class TestReplaceFunction:
    """REPLACE(str, from, to) is a scalar function whose name is also a SQL
    keyword (used in REPLACE INTO / INSERT OR REPLACE).  The grammar must
    accept it in function-call position."""

    def test_replace_basic(self) -> None:
        ref, got = _both("SELECT REPLACE('hello world', 'world', 'there')")
        assert got == ref == [("hello there",)]

    def test_replace_no_match(self) -> None:
        ref, got = _both("SELECT REPLACE('hello', 'xyz', '!!!')")
        assert got == ref == [("hello",)]

    def test_replace_empty_replacement(self) -> None:
        ref, got = _both("SELECT REPLACE('aababc', 'b', '')")
        assert got == ref == [("aaac",)]

    def test_replace_with_column(self) -> None:
        setup = [
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (1, 'foo bar')",
            "INSERT INTO t VALUES (2, 'bar baz')",
        ]
        ref, got = _both(
            "SELECT id, REPLACE(name, 'bar', 'X') FROM t ORDER BY id", setup=setup
        )
        assert got == ref

    def test_replace_on_null_returns_null(self) -> None:
        """REPLACE(NULL, ...) returns NULL."""
        ref, got = _both("SELECT REPLACE(NULL, 'a', 'b')")
        assert got == ref

    def test_replace_in_where(self) -> None:
        setup = [
            "CREATE TABLE t (id INTEGER, name TEXT)",
            "INSERT INTO t VALUES (1, 'hello world')",
            "INSERT INTO t VALUES (2, 'goodbye world')",
        ]
        ref, got = _both(
            "SELECT id FROM t WHERE REPLACE(name, ' world', '') = 'hello'",
            setup=setup,
        )
        assert got == ref == [(1,)]

    def test_replace_insert_or_replace_not_broken(self) -> None:
        """INSERT OR REPLACE still works after the grammar change."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)")
        con.execute("INSERT INTO t VALUES (1, 'original')")
        con.execute("INSERT OR REPLACE INTO t VALUES (1, 'replaced')")
        rows = con.execute("SELECT val FROM t WHERE id = 1").fetchall()
        assert rows == [("replaced",)]

    def test_replace_shorthand_dml_not_broken(self) -> None:
        """REPLACE INTO t ... shorthand still works."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)")
        con.execute("INSERT INTO t VALUES (1, 'original')")
        con.execute("REPLACE INTO t VALUES (1, 'via replace dml')")
        rows = con.execute("SELECT val FROM t WHERE id = 1").fetchall()
        assert rows == [("via replace dml",)]
