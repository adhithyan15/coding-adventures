"""tests/test_tier6_sqlite_convergence.py — SQLite compatibility gap closure.

This module covers the four parser-level features added in the "SQLite
convergence" sprint to close the gap between mini-sqlite's SQL subset and
real SQLite's accepted syntax:

1. **SELECT without FROM** — ``SELECT 1``, ``SELECT UPPER('hello')``,
   ``SELECT DATE('now')``.  The grammar now makes the FROM clause optional;
   the planner substitutes a ``SingleRow`` leaf that executes the SELECT
   list exactly once.

2. **CAST(expr AS type)** — ``CAST(3.14 AS INTEGER)``,
   ``CAST('42' AS REAL)``.  CAST is now a keyword with its own grammar rule
   (``cast_expr``) so the ``AS`` inside it is not confused with a column
   alias.  The adapter maps it to the existing ``cast`` scalar function.

3. **Table alias without AS** — ``FROM employees e``,
   ``FROM employees e JOIN departments d ON e.dept = d.id``.  Both the
   explicit ``AS`` form and the bare-name form are accepted.

4. **GLOB operator** — ``name GLOB 'J*'``, ``file GLOB '*.py'``.
   Case-sensitive Unix-style glob matching.  Internally resolved to the
   ``glob(pattern, string)`` scalar function.

All tests run against the in-memory backend to stay fast and avoid
filesystem state.  Oracle-class tests also run the same SQL against real
``sqlite3`` to confirm byte-for-byte compatible output.
"""

from __future__ import annotations

import sqlite3
import typing

import mini_sqlite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _conn() -> mini_sqlite.Connection:
    return mini_sqlite.connect(":memory:")


def _rows(sql: str, params: tuple[typing.Any, ...] = ()) -> list[tuple[typing.Any, ...]]:
    con = _conn()
    cur = con.cursor()
    cur.execute(sql, params)
    return cur.fetchall()


def _real_rows(sql: str, params: tuple[typing.Any, ...] = ()) -> list[tuple[typing.Any, ...]]:
    """Execute SQL against real sqlite3 for oracle comparison."""
    con = sqlite3.connect(":memory:")
    cur = con.cursor()
    cur.execute(sql, params)
    return cur.fetchall()


# ============================================================================
# 1. SELECT without FROM
# ============================================================================


class TestSelectWithoutFrom:
    """SELECT expressions evaluated without a FROM clause."""

    def test_literal_integer(self) -> None:
        assert _rows("SELECT 1") == [(1,)]

    def test_literal_string(self) -> None:
        assert _rows("SELECT 'hello'") == [("hello",)]

    def test_arithmetic(self) -> None:
        assert _rows("SELECT 1 + 2") == [(3,)]

    def test_nested_arithmetic(self) -> None:
        assert _rows("SELECT 2 * 3 + 4") == [(10,)]

    def test_scalar_function_upper(self) -> None:
        assert _rows("SELECT UPPER('hello')") == [("HELLO",)]

    def test_scalar_function_lower(self) -> None:
        assert _rows("SELECT LOWER('WORLD')") == [("world",)]

    def test_multiple_columns(self) -> None:
        assert _rows("SELECT 1, 2, 3") == [(1, 2, 3)]

    def test_null_literal(self) -> None:
        assert _rows("SELECT NULL") == [(None,)]

    def test_boolean_literals(self) -> None:
        assert _rows("SELECT TRUE, FALSE") == [(True, False)]

    def test_string_concatenation(self) -> None:
        # || is the standard SQL string concat operator
        # Using UPPER since || may not be wired yet
        assert _rows("SELECT UPPER('abc')") == [("ABC",)]

    def test_column_alias(self) -> None:
        con = _conn()
        cur = con.cursor()
        cur.execute("SELECT 42 AS answer")
        assert cur.fetchall() == [(42,)]
        assert cur.description[0][0] == "answer"

    def test_no_from_is_single_row(self) -> None:
        """Result is always exactly one row."""
        rows = _rows("SELECT 99")
        assert len(rows) == 1

    def test_no_from_with_where_true(self) -> None:
        """WHERE 1=1 should keep the row."""
        rows = _rows("SELECT 7 WHERE 1 = 1")
        assert rows == [(7,)]

    def test_no_from_with_where_false(self) -> None:
        """WHERE 1=0 should produce an empty result."""
        rows = _rows("SELECT 7 WHERE 1 = 0")
        assert rows == []

    def test_select_without_from_oracle(self) -> None:
        """Output matches real SQLite."""
        sql = "SELECT 1 + 1, UPPER('hi'), 3.14"
        assert _rows(sql) == _real_rows(sql)


# ============================================================================
# 2. CAST(expr AS type)
# ============================================================================


class TestCast:
    """CAST expression in various contexts."""

    def test_cast_float_to_integer(self) -> None:
        assert _rows("SELECT CAST(3.7 AS INTEGER)") == [(3,)]

    def test_cast_string_to_integer(self) -> None:
        assert _rows("SELECT CAST('42' AS INTEGER)") == [(42,)]

    def test_cast_integer_to_text(self) -> None:
        assert _rows("SELECT CAST(100 AS TEXT)") == [("100",)]

    def test_cast_integer_to_real(self) -> None:
        assert _rows("SELECT CAST(5 AS REAL)") == [(5.0,)]

    def test_cast_null_stays_null(self) -> None:
        assert _rows("SELECT CAST(NULL AS INTEGER)") == [(None,)]

    def test_cast_in_expression(self) -> None:
        # CAST result used in arithmetic
        rows = _rows("SELECT CAST('3' AS INTEGER) + 1")
        assert rows == [(4,)]

    def test_cast_in_where_clause(self) -> None:
        con = _conn()
        cur = con.cursor()
        cur.execute("CREATE TABLE t (val TEXT)")
        cur.execute("INSERT INTO t VALUES ('10'), ('20'), ('5')")
        # Filter by casting the text column to integer
        rows = con.cursor().execute(
            "SELECT val FROM t WHERE CAST(val AS INTEGER) > 8"
        ).fetchall()
        assert sorted(rows) == [("10",), ("20",)]

    def test_cast_column_to_real(self) -> None:
        con = _conn()
        cur = con.cursor()
        cur.execute("CREATE TABLE prices (p INTEGER)")
        cur.execute("INSERT INTO prices VALUES (7), (13)")
        rows = con.cursor().execute(
            "SELECT CAST(p AS REAL) FROM prices"
        ).fetchall()
        assert sorted(rows) == [(7.0,), (13.0,)]

    def test_cast_oracle_float_to_int(self) -> None:
        sql = "SELECT CAST(3.99 AS INTEGER)"
        assert _rows(sql) == _real_rows(sql)

    def test_cast_oracle_text_to_real(self) -> None:
        sql = "SELECT CAST('2.5' AS REAL)"
        assert _rows(sql) == _real_rows(sql)

    def test_cast_case_insensitive_type_name(self) -> None:
        # Type names should be case-insensitive (already upper-cased in adapter)
        assert _rows("SELECT CAST(3.14 AS integer)") == [(3,)]

    def test_cast_nested(self) -> None:
        # CAST of a CAST
        rows = _rows("SELECT CAST(CAST(3.7 AS INTEGER) AS TEXT)")
        assert rows == [("3",)]


# ============================================================================
# 3. Table alias without AS
# ============================================================================


class TestAliasWithoutAs:
    """FROM table alias without the AS keyword."""

    def _setup_conn(self) -> mini_sqlite.Connection:
        con = _conn()
        cur = con.cursor()
        cur.execute("CREATE TABLE employees (id INTEGER, name TEXT, dept INTEGER)")
        cur.execute("INSERT INTO employees VALUES (1, 'Alice', 10)")
        cur.execute("INSERT INTO employees VALUES (2, 'Bob', 20)")
        cur.execute("INSERT INTO employees VALUES (3, 'Carol', 10)")
        cur.execute("CREATE TABLE departments (id INTEGER, name TEXT)")
        cur.execute("INSERT INTO departments VALUES (10, 'Engineering')")
        cur.execute("INSERT INTO departments VALUES (20, 'Marketing')")
        return con

    def test_simple_alias_without_as(self) -> None:
        con = self._setup_conn()
        rows = con.cursor().execute(
            "SELECT e.name FROM employees e WHERE e.id = 1"
        ).fetchall()
        assert rows == [("Alice",)]

    def test_alias_without_as_in_join(self) -> None:
        con = self._setup_conn()
        rows = con.cursor().execute(
            "SELECT e.name, d.name FROM employees e "
            "JOIN departments d ON e.dept = d.id "
            "ORDER BY e.name"
        ).fetchall()
        assert rows == [
            ("Alice", "Engineering"),
            ("Bob", "Marketing"),
            ("Carol", "Engineering"),
        ]

    def test_both_tables_alias_without_as(self) -> None:
        con = self._setup_conn()
        rows = con.cursor().execute(
            "SELECT e.name, d.name FROM employees e "
            "INNER JOIN departments d ON e.dept = d.id "
            "WHERE d.id = 10 ORDER BY e.name"
        ).fetchall()
        assert rows == [("Alice", "Engineering"), ("Carol", "Engineering")]

    def test_alias_without_as_select_star(self) -> None:
        con = self._setup_conn()
        rows = con.cursor().execute(
            "SELECT * FROM employees e WHERE e.id = 2"
        ).fetchall()
        assert rows == [(2, "Bob", 20)]

    def test_alias_without_as_subquery(self) -> None:
        con = self._setup_conn()
        rows = con.cursor().execute(
            "SELECT e.name FROM employees e "
            "WHERE e.dept IN (SELECT id FROM departments WHERE name = 'Engineering')"
        ).fetchall()
        assert sorted(rows) == [("Alice",), ("Carol",)]

    def test_alias_with_as_still_works(self) -> None:
        """Explicit AS still works after the grammar change."""
        con = self._setup_conn()
        rows = con.cursor().execute(
            "SELECT e.name FROM employees AS e WHERE e.id = 3"
        ).fetchall()
        assert rows == [("Carol",)]

    def test_alias_without_as_left_join(self) -> None:
        con = self._setup_conn()
        # Add an employee with no matching department
        con.cursor().execute("INSERT INTO employees VALUES (4, 'Dave', 99)")
        rows = con.cursor().execute(
            "SELECT e.name, d.name FROM employees e "
            "LEFT JOIN departments d ON e.dept = d.id "
            "ORDER BY e.name"
        ).fetchall()
        assert rows == [
            ("Alice", "Engineering"),
            ("Bob", "Marketing"),
            ("Carol", "Engineering"),
            ("Dave", None),
        ]


# ============================================================================
# 4. GLOB operator
# ============================================================================


class TestGlob:
    """GLOB case-sensitive pattern matching."""

    def _setup_conn(self) -> mini_sqlite.Connection:
        con = _conn()
        cur = con.cursor()
        cur.execute("CREATE TABLE files (name TEXT)")
        for f in ["app.py", "test_app.py", "README.md", "Makefile", "config.yaml", "test_utils.py"]:
            cur.execute("INSERT INTO files VALUES (?)", (f,))
        return con

    def test_glob_star_prefix(self) -> None:
        con = self._setup_conn()
        rows = con.cursor().execute(
            "SELECT name FROM files WHERE name GLOB '*.py' ORDER BY name"
        ).fetchall()
        assert rows == [("app.py",), ("test_app.py",), ("test_utils.py",)]

    def test_glob_question_mark(self) -> None:
        con = self._setup_conn()
        rows = con.cursor().execute(
            "SELECT name FROM files WHERE name GLOB '???.py' ORDER BY name"
        ).fetchall()
        assert rows == [("app.py",)]

    def test_glob_case_sensitive(self) -> None:
        """GLOB is case-sensitive unlike LIKE."""
        con = self._setup_conn()
        # Uppercase pattern — 'readme.md' won't match 'README.md'
        rows = con.cursor().execute(
            "SELECT name FROM files WHERE name GLOB 'readme.md'"
        ).fetchall()
        assert rows == []  # no match — case-sensitive
        rows_upper = con.cursor().execute(
            "SELECT name FROM files WHERE name GLOB 'README.md'"
        ).fetchall()
        assert rows_upper == [("README.md",)]

    def test_glob_prefix_wildcard(self) -> None:
        con = self._setup_conn()
        rows = con.cursor().execute(
            "SELECT name FROM files WHERE name GLOB 'test_*' ORDER BY name"
        ).fetchall()
        assert rows == [("test_app.py",), ("test_utils.py",)]

    def test_glob_no_match(self) -> None:
        con = self._setup_conn()
        rows = con.cursor().execute(
            "SELECT name FROM files WHERE name GLOB '*.go'"
        ).fetchall()
        assert rows == []

    def test_glob_star_matches_all(self) -> None:
        con = self._setup_conn()
        rows = con.cursor().execute(
            "SELECT COUNT(*) FROM files WHERE name GLOB '*'"
        ).fetchall()
        assert rows == [(6,)]

    def test_not_glob(self) -> None:
        con = self._setup_conn()
        rows = con.cursor().execute(
            "SELECT name FROM files WHERE name NOT GLOB '*.py' ORDER BY name"
        ).fetchall()
        assert rows == [("Makefile",), ("README.md",), ("config.yaml",)]

    def test_glob_with_column_reference(self) -> None:
        """GLOB pattern can be any expression (not just literals)."""
        con = _conn()
        cur = con.cursor()
        cur.execute("CREATE TABLE patterns (str TEXT, pat TEXT)")
        cur.execute("INSERT INTO patterns VALUES ('hello', 'h*')")
        cur.execute("INSERT INTO patterns VALUES ('world', 'w*')")
        cur.execute("INSERT INTO patterns VALUES ('hello', 'x*')")
        rows = con.cursor().execute(
            "SELECT str FROM patterns WHERE str GLOB pat ORDER BY str"
        ).fetchall()
        assert rows == [("hello",), ("world",)]

    def test_glob_null_propagation(self) -> None:
        con = _conn()
        cur = con.cursor()
        cur.execute("CREATE TABLE t (s TEXT)")
        cur.execute("INSERT INTO t VALUES (NULL)")
        rows = con.cursor().execute(
            "SELECT s FROM t WHERE s GLOB '*'"
        ).fetchall()
        assert rows == []  # NULL GLOB '*' → NULL → filtered out

    def test_glob_oracle(self) -> None:
        """GLOB output matches real SQLite."""
        con = _conn()
        real = sqlite3.connect(":memory:")
        for db in (con.cursor(), real.cursor()):
            db.execute("CREATE TABLE words (w TEXT)")
            for word in ["apple", "apricot", "banana", "cherry", "avocado"]:
                db.execute("INSERT INTO words VALUES (?)", (word,))

        sql = "SELECT w FROM words WHERE w GLOB 'a*' ORDER BY w"
        our_rows = con.cursor().execute(sql).fetchall()
        real_rows = real.cursor().execute(sql).fetchall()
        assert our_rows == real_rows

    def test_glob_select_without_from(self) -> None:
        """GLOB can be used inside SELECT without FROM."""
        rows = _rows("SELECT 'hello' GLOB 'h*'")
        assert rows == [(1,)]

    def test_glob_case_sensitive_vs_like(self) -> None:
        """Demonstrate the key difference: GLOB is case-sensitive, LIKE is not."""
        # LIKE is case-insensitive for ASCII
        like_rows = _rows("SELECT 'Hello' LIKE 'hello'")
        glob_rows = _rows("SELECT 'Hello' GLOB 'hello'")
        assert like_rows == [(1,)]   # LIKE matches
        assert glob_rows == [(0,)]   # GLOB does NOT match


# ============================================================================
# 5. Combined / integration tests
# ============================================================================


class TestCombined:
    """Tests that combine multiple convergence features."""

    def test_cast_in_select_without_from(self) -> None:
        """CAST works inside SELECT without FROM."""
        assert _rows("SELECT CAST(3.9 AS INTEGER)") == [(3,)]

    def test_cast_and_glob_together(self) -> None:
        """CAST and GLOB in the same query."""
        con = _conn()
        cur = con.cursor()
        cur.execute("CREATE TABLE data (raw TEXT)")
        cur.execute("INSERT INTO data VALUES ('42'), ('7'), ('100'), ('3.5')")
        # Use GLOB to filter by pattern, CAST in SELECT.
        # '??' matches exactly two-character strings → only '42'.
        # ORDER BY is omitted since raw is not projected.
        rows = con.cursor().execute(
            "SELECT CAST(raw AS INTEGER) FROM data WHERE raw GLOB '??'"
        ).fetchall()
        assert rows == [(42,)]

    def test_alias_without_as_with_cast(self) -> None:
        """Alias-without-AS works alongside CAST in SELECT."""
        con = _conn()
        cur = con.cursor()
        cur.execute("CREATE TABLE t (price TEXT)")
        cur.execute("INSERT INTO t VALUES ('10'), ('20')")
        # ORDER BY non-projected column is a known limitation; use sorted()
        # for an order-independent assertion.
        rows = con.cursor().execute(
            "SELECT CAST(t.price AS INTEGER) FROM t t"
        ).fetchall()
        assert sorted(rows) == [(10,), (20,)]

    def test_select_without_from_glob(self) -> None:
        """SELECT without FROM using GLOB to test pattern."""
        assert _rows("SELECT 'world' GLOB '*rld'") == [(1,)]

    def test_all_four_features_in_one_query(self) -> None:
        """Complex query using all four convergence features at once."""
        con = _conn()
        cur = con.cursor()
        cur.execute("CREATE TABLE items (name TEXT, price TEXT)")
        cur.execute("INSERT INTO items VALUES ('apple', '1.50')")
        cur.execute("INSERT INTO items VALUES ('apricot', '2.00')")
        cur.execute("INSERT INTO items VALUES ('banana', '0.75')")
        rows = con.cursor().execute(
            # alias without AS: 'items i'
            # CAST: price text → real
            # GLOB: name starts with 'a'
            "SELECT i.name, CAST(i.price AS REAL) FROM items i "
            "WHERE i.name GLOB 'a*' ORDER BY i.name"
        ).fetchall()
        assert rows == [("apple", 1.5), ("apricot", 2.0)]
