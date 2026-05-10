"""tests/test_tier5_group_concat.py — End-to-end tests for GROUP_CONCAT.

GROUP_CONCAT is SQLite's string-aggregation function.  It concatenates all
non-NULL column values within a group, joining them with a separator.

Syntax
------
    GROUP_CONCAT(col)          — joins with ',' (SQLite default)
    GROUP_CONCAT(col, sep)     — joins with explicit literal separator

SQL standard analogue: STRING_AGG(col, sep) (PostgreSQL / SQL:2016).

Behaviour
---------
- NULL inputs are silently ignored (consistent with COUNT, SUM, etc.)
- An empty group (no non-NULL rows) returns NULL
- The separator must be a string literal — expressions are not allowed

Coverage targets
----------------
- mini_sqlite/adapter.py  : GROUP_CONCAT parsed to AggregateExpr
- sql-codegen/compiler.py : separator baked into InitAgg
- sql-vm/vm.py            : _AggState.items accumulation + finalization
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite
from mini_sqlite import ProgrammingError

# ---------------------------------------------------------------------------
# Fixture — shared in-memory connection
# ---------------------------------------------------------------------------


@pytest.fixture
def conn():
    """Fresh mini_sqlite in-memory connection with an employees table."""
    c = mini_sqlite.connect(":memory:")
    cur = c.cursor()
    cur.execute(
        "CREATE TABLE employees (id INTEGER PRIMARY KEY, name TEXT, dept TEXT, salary INTEGER)"
    )
    for row in [
        (1, "Alice", "eng", 90000),
        (2, "Bob",   "eng", 80000),
        (3, "Carol", "sales", 70000),
        (4, "Dave",  "sales", 75000),
        (5, "Eve",   "eng", 85000),
    ]:
        cur.execute("INSERT INTO employees VALUES (?, ?, ?, ?)", row)
    return c


# ---------------------------------------------------------------------------
# Basic GROUP_CONCAT behaviour
# ---------------------------------------------------------------------------


class TestGroupConcatBasic:
    """Core GROUP_CONCAT semantics verified against sqlite3 oracle."""

    def test_default_separator_global(self, conn) -> None:
        """GROUP_CONCAT over all rows uses ',' as default separator."""
        # ORDER BY on a global aggregate is a no-op (single result row) so we
        # omit it — the test checks sorted names, which is order-independent.
        result = conn.cursor().execute(
            "SELECT GROUP_CONCAT(name) FROM employees WHERE dept = 'eng'"
        ).fetchone()[0]
        # eng names: Alice, Bob, Eve — order depends on scan order, check via set
        assert sorted(result.split(",")) == ["Alice", "Bob", "Eve"]

    def test_custom_separator(self, conn) -> None:
        """GROUP_CONCAT with explicit ' | ' separator."""
        result = conn.cursor().execute(
            "SELECT GROUP_CONCAT(name, ' | ') FROM employees WHERE dept = 'eng'"
        ).fetchone()[0]
        assert sorted(result.split(" | ")) == ["Alice", "Bob", "Eve"]

    def test_per_group_default_sep(self, conn) -> None:
        """GROUP_CONCAT within GROUP BY produces per-group strings."""
        rows = conn.cursor().execute(
            "SELECT dept, GROUP_CONCAT(name) FROM employees GROUP BY dept ORDER BY dept"
        ).fetchall()
        assert len(rows) == 2
        dept_map = {r[0]: sorted(r[1].split(",")) for r in rows}
        assert dept_map["eng"] == ["Alice", "Bob", "Eve"]
        assert dept_map["sales"] == ["Carol", "Dave"]

    def test_per_group_custom_sep(self, conn) -> None:
        """GROUP_CONCAT with separator within GROUP BY."""
        rows = conn.cursor().execute(
            "SELECT dept, GROUP_CONCAT(name, '; ') FROM employees GROUP BY dept ORDER BY dept"
        ).fetchall()
        dept_map = {r[0]: sorted(r[1].split("; ")) for r in rows}
        assert dept_map["eng"] == ["Alice", "Bob", "Eve"]
        assert dept_map["sales"] == ["Carol", "Dave"]

    def test_numeric_column(self, conn) -> None:
        """GROUP_CONCAT converts numeric values to strings."""
        result = conn.cursor().execute(
            "SELECT GROUP_CONCAT(salary, ',') FROM employees WHERE dept = 'sales'"
        ).fetchone()[0]
        # Carol=70000, Dave=75000
        parts = sorted(result.split(","))
        assert parts == ["70000", "75000"]

    def test_single_row_group(self, conn) -> None:
        """GROUP_CONCAT on a group with one row returns that row's value (no separator)."""
        result = conn.cursor().execute(
            "SELECT GROUP_CONCAT(name) FROM employees WHERE name = 'Alice'"
        ).fetchone()[0]
        assert result == "Alice"

    def test_empty_separator(self, conn) -> None:
        """GROUP_CONCAT with an empty string separator concatenates with no delimiter."""
        # ORDER BY on a global aggregate is a no-op (single result row); omit it.
        result = conn.cursor().execute(
            "SELECT GROUP_CONCAT(name, '') FROM employees WHERE dept = 'sales'"
        ).fetchone()[0]
        # Carol and Dave with '' separator — order is insertion-order so we
        # verify the character set and total length rather than exact sequence.
        assert set(result) == set("CarolDave")
        assert len(result) == len("CarolDave")


# ---------------------------------------------------------------------------
# NULL handling
# ---------------------------------------------------------------------------


class TestGroupConcatNullHandling:
    """GROUP_CONCAT ignores NULL inputs (consistent with SUM, AVG, etc.)."""

    def test_null_inputs_ignored(self) -> None:
        """NULL values in the column are silently skipped."""
        conn = mini_sqlite.connect(":memory:")
        cur = conn.cursor()
        cur.execute("CREATE TABLE t (v TEXT)")
        cur.execute("INSERT INTO t VALUES ('a')")
        cur.execute("INSERT INTO t VALUES (NULL)")
        cur.execute("INSERT INTO t VALUES ('b')")
        cur.execute("INSERT INTO t VALUES (NULL)")
        cur.execute("INSERT INTO t VALUES ('c')")
        result = cur.execute("SELECT GROUP_CONCAT(v) FROM t").fetchone()[0]
        assert result == "a,b,c"

    def test_all_null_returns_null(self) -> None:
        """If all values are NULL, GROUP_CONCAT returns NULL (not empty string)."""
        conn = mini_sqlite.connect(":memory:")
        cur = conn.cursor()
        cur.execute("CREATE TABLE t (v TEXT)")
        cur.execute("INSERT INTO t VALUES (NULL)")
        cur.execute("INSERT INTO t VALUES (NULL)")
        result = cur.execute("SELECT GROUP_CONCAT(v) FROM t").fetchone()[0]
        assert result is None

    def test_empty_table_returns_null(self) -> None:
        """GROUP_CONCAT over an empty table returns NULL."""
        conn = mini_sqlite.connect(":memory:")
        cur = conn.cursor()
        cur.execute("CREATE TABLE t (v TEXT)")
        result = cur.execute("SELECT GROUP_CONCAT(v) FROM t").fetchone()[0]
        assert result is None

    def test_mixed_null_and_non_null_with_separator(self) -> None:
        """Separator is only inserted between non-NULL values."""
        conn = mini_sqlite.connect(":memory:")
        cur = conn.cursor()
        cur.execute("CREATE TABLE t (v TEXT)")
        for v in ["x", None, "y", None, "z"]:
            cur.execute("INSERT INTO t VALUES (?)", (v,))
        result = cur.execute("SELECT GROUP_CONCAT(v, '-') FROM t").fetchone()[0]
        assert result == "x-y-z"


# ---------------------------------------------------------------------------
# Oracle comparison
# ---------------------------------------------------------------------------


class TestGroupConcatOracle:
    """Verify GROUP_CONCAT results match sqlite3's output."""

    def _oracle(self, setup_sql: str, query: str) -> list:
        """Run query against real sqlite3 and return fetchall()."""
        db = sqlite3.connect(":memory:")
        db.executescript(setup_sql)
        return db.execute(query).fetchall()

    def test_oracle_default_sep_per_group(self, conn) -> None:
        """Per-group GROUP_CONCAT matches sqlite3 oracle."""
        setup = (
            "CREATE TABLE employees (id INTEGER PRIMARY KEY, name TEXT, dept TEXT, salary INTEGER);"
            "INSERT INTO employees VALUES (1,'Alice','eng',90000);"
            "INSERT INTO employees VALUES (2,'Bob','eng',80000);"
            "INSERT INTO employees VALUES (3,'Carol','sales',70000);"
            "INSERT INTO employees VALUES (4,'Dave','sales',75000);"
            "INSERT INTO employees VALUES (5,'Eve','eng',85000);"
        )
        query = (
            "SELECT dept, GROUP_CONCAT(name) FROM employees "
            "GROUP BY dept ORDER BY dept"
        )
        oracle_rows = self._oracle(setup, query)
        our_rows = conn.cursor().execute(query).fetchall()

        # Compare sets within each group (order of names may differ).
        assert len(oracle_rows) == len(our_rows)
        for (o_dept, o_names), (m_dept, m_names) in zip(
            sorted(oracle_rows), sorted(our_rows), strict=True
        ):
            assert o_dept == m_dept
            assert sorted(o_names.split(",")) == sorted(m_names.split(","))

    def test_oracle_custom_sep_global(self, conn) -> None:
        """Global GROUP_CONCAT with custom separator matches sqlite3."""
        setup = (
            "CREATE TABLE employees (id INTEGER PRIMARY KEY, name TEXT, dept TEXT, salary INTEGER);"
            "INSERT INTO employees VALUES (1,'Alice','eng',90000);"
            "INSERT INTO employees VALUES (2,'Bob','eng',80000);"
            "INSERT INTO employees VALUES (3,'Carol','sales',70000);"
            "INSERT INTO employees VALUES (4,'Dave','sales',75000);"
            "INSERT INTO employees VALUES (5,'Eve','eng',85000);"
        )
        query = "SELECT GROUP_CONCAT(name, ' & ') FROM employees"
        oracle = self._oracle(setup, query)[0][0]
        ours = conn.cursor().execute(query).fetchone()[0]
        # Both should contain all 5 names joined by ' & '.
        assert sorted(oracle.split(" & ")) == sorted(ours.split(" & "))


# ---------------------------------------------------------------------------
# Error cases
# ---------------------------------------------------------------------------


class TestGroupConcatErrors:
    """GROUP_CONCAT rejects invalid argument forms at parse time."""

    def test_no_args_raises(self) -> None:
        """GROUP_CONCAT() with no arguments is invalid."""
        conn = mini_sqlite.connect(":memory:")
        cur = conn.cursor()
        cur.execute("CREATE TABLE t (v TEXT)")
        with pytest.raises(ProgrammingError):
            cur.execute("SELECT GROUP_CONCAT() FROM t")

    def test_too_many_args_raises(self) -> None:
        """GROUP_CONCAT with 3+ args is invalid."""
        conn = mini_sqlite.connect(":memory:")
        cur = conn.cursor()
        cur.execute("CREATE TABLE t (v TEXT)")
        with pytest.raises(ProgrammingError):
            cur.execute("SELECT GROUP_CONCAT(v, ',', 'extra') FROM t")
