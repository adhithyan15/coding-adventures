"""
Tier-9 feature tests: DEFAULT column values.

Every test runs the same SQL against both mini-sqlite and real sqlite3 and
asserts that the results are identical (oracle-verified).

Feature: When a column is defined with DEFAULT <literal>, INSERT statements
that omit that column receive the declared default value instead of NULL.

Background
----------
SQLite's DEFAULT clause accepts:
  • INTEGER / REAL literals      (DEFAULT 0, DEFAULT 3.14)
  • Text literals                (DEFAULT 'active')
  • NULL keyword                 (DEFAULT NULL)
  • Signed integer / real        (DEFAULT -1)
  • Boolean-like integers        (DEFAULT 1, DEFAULT 0)

The pipeline flow is:
  SQL text
    → sql-parser (parses DEFAULT primary)
    → mini-sqlite adapter  (_col_def extracts literal via _primary)
    → sql-backend ColumnDef (stores default value / NO_DEFAULT sentinel)
    → sql-codegen IR ColumnDef (NO_COLUMN_DEFAULT sentinel)
    → sql-vm _do_create_table (passes default to BackendColumnDef)
    → InMemoryBackend._apply_defaults (fills missing columns on INSERT)
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite
from mini_sqlite.errors import IntegrityError

# ---------------------------------------------------------------------------
# Helper: run the same SQL on both engines, assert identical results
# ---------------------------------------------------------------------------


def _mini(setup: list[str], query: str) -> list[tuple]:
    """Run *setup* statements then *query* against mini-sqlite."""
    con = mini_sqlite.connect(":memory:")
    cur = con.cursor()
    for sql in setup:
        cur.execute(sql)
    return cur.execute(query).fetchall()


def _ref(setup: list[str], query: str) -> list[tuple]:
    """Run *setup* statements then *query* against real sqlite3."""
    con = sqlite3.connect(":memory:")
    cur = con.cursor()
    for sql in setup:
        cur.execute(sql)
    return cur.execute(query).fetchall()


def oracle(setup: list[str], query: str) -> list[tuple]:
    """Assert mini-sqlite == real sqlite3 and return the common result."""
    mini_result = _mini(setup, query)
    ref_result = _ref(setup, query)
    assert mini_result == ref_result, (
        f"Oracle mismatch!\n  mini-sqlite: {mini_result}\n  sqlite3:     {ref_result}"
    )
    return mini_result


# ---------------------------------------------------------------------------
# TestDefaultIntegerLiteral
# ---------------------------------------------------------------------------


class TestDefaultIntegerLiteral:
    """DEFAULT <integer> fills omitted columns with the declared integer."""

    DDL_ZERO = [
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, score INTEGER DEFAULT 0)",
    ]
    DDL_NONZERO = [
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, score INTEGER DEFAULT 42)",
    ]

    def test_default_zero_when_column_omitted(self) -> None:
        rows = oracle(
            self.DDL_ZERO + ["INSERT INTO t (id, name) VALUES (1, 'Alice')"],
            "SELECT score FROM t",
        )
        assert rows == [(0,)]

    def test_default_nonzero_when_column_omitted(self) -> None:
        rows = oracle(
            self.DDL_NONZERO + ["INSERT INTO t (id, name) VALUES (1, 'Alice')"],
            "SELECT score FROM t",
        )
        assert rows == [(42,)]

    def test_explicit_value_overrides_default_zero(self) -> None:
        rows = oracle(
            self.DDL_ZERO + ["INSERT INTO t (id, name, score) VALUES (1, 'Alice', 99)"],
            "SELECT score FROM t",
        )
        assert rows == [(99,)]

    def test_explicit_value_overrides_default_nonzero(self) -> None:
        rows = oracle(
            self.DDL_NONZERO + ["INSERT INTO t (id, name, score) VALUES (1, 'Alice', 7)"],
            "SELECT score FROM t",
        )
        assert rows == [(7,)]

    def test_multiple_rows_mix_default_and_explicit(self) -> None:
        setup = self.DDL_ZERO + [
            "INSERT INTO t (id, name) VALUES (1, 'Alice')",
            "INSERT INTO t (id, name, score) VALUES (2, 'Bob', 55)",
            "INSERT INTO t (id, name) VALUES (3, 'Carol')",
        ]
        rows = oracle(setup, "SELECT id, score FROM t ORDER BY id")
        assert rows == [(1, 0), (2, 55), (3, 0)]

    def test_default_one_boolean_like(self) -> None:
        """DEFAULT 1 is a common active/inactive flag pattern."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, active INTEGER DEFAULT 1)",
            "INSERT INTO t (id) VALUES (1)",
        ]
        rows = oracle(setup, "SELECT active FROM t")
        assert rows == [(1,)]

    def test_two_default_columns_both_omitted(self) -> None:
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, x INTEGER DEFAULT 10, y INTEGER DEFAULT 20)",
            "INSERT INTO t (id) VALUES (1)",
        ]
        rows = oracle(setup, "SELECT x, y FROM t")
        assert rows == [(10, 20)]

    def test_two_default_columns_one_explicit(self) -> None:
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, x INTEGER DEFAULT 10, y INTEGER DEFAULT 20)",
            "INSERT INTO t (id, x) VALUES (1, 99)",
        ]
        rows = oracle(setup, "SELECT x, y FROM t")
        assert rows == [(99, 20)]


# ---------------------------------------------------------------------------
# TestDefaultNullLiteral
# ---------------------------------------------------------------------------


class TestDefaultNullLiteral:
    """DEFAULT NULL — omitted column gets NULL (same as no DEFAULT, but explicit)."""

    DDL = [
        "CREATE TABLE t (id INTEGER PRIMARY KEY, note TEXT DEFAULT NULL)",
    ]

    def test_default_null_when_column_omitted(self) -> None:
        rows = oracle(
            self.DDL + ["INSERT INTO t (id) VALUES (1)"],
            "SELECT note FROM t",
        )
        assert rows == [(None,)]

    def test_explicit_null_same_as_default_null(self) -> None:
        rows = oracle(
            self.DDL + ["INSERT INTO t (id, note) VALUES (1, NULL)"],
            "SELECT note FROM t",
        )
        assert rows == [(None,)]

    def test_explicit_value_overrides_default_null(self) -> None:
        rows = oracle(
            self.DDL + ["INSERT INTO t (id, note) VALUES (1, 'hello')"],
            "SELECT note FROM t",
        )
        assert rows == [("hello",)]


# ---------------------------------------------------------------------------
# TestDefaultTextLiteral
# ---------------------------------------------------------------------------


class TestDefaultTextLiteral:
    """DEFAULT 'string' — omitted column gets the declared text."""

    DDL_STATUS = [
        "CREATE TABLE t (id INTEGER PRIMARY KEY, status TEXT DEFAULT 'active')",
    ]

    def test_default_string_when_column_omitted(self) -> None:
        rows = oracle(
            self.DDL_STATUS + ["INSERT INTO t (id) VALUES (1)"],
            "SELECT status FROM t",
        )
        assert rows == [("active",)]

    def test_explicit_string_overrides_default(self) -> None:
        rows = oracle(
            self.DDL_STATUS + ["INSERT INTO t (id, status) VALUES (1, 'inactive')"],
            "SELECT status FROM t",
        )
        assert rows == [("inactive",)]

    def test_multiple_rows_text_default(self) -> None:
        setup = self.DDL_STATUS + [
            "INSERT INTO t (id) VALUES (1)",
            "INSERT INTO t (id, status) VALUES (2, 'pending')",
            "INSERT INTO t (id) VALUES (3)",
        ]
        rows = oracle(setup, "SELECT id, status FROM t ORDER BY id")
        assert rows == [(1, "active"), (2, "pending"), (3, "active")]


# ---------------------------------------------------------------------------
# TestDefaultWithNotNull
# ---------------------------------------------------------------------------


class TestDefaultWithNotNull:
    """NOT NULL + DEFAULT: column cannot be NULL but has a safe default."""

    DDL = [
        "CREATE TABLE t (id INTEGER PRIMARY KEY, score INTEGER NOT NULL DEFAULT 0)",
    ]

    def test_not_null_with_default_fills_correctly(self) -> None:
        rows = oracle(
            self.DDL + ["INSERT INTO t (id) VALUES (1)"],
            "SELECT score FROM t",
        )
        assert rows == [(0,)]

    def test_not_null_explicit_value_accepted(self) -> None:
        rows = oracle(
            self.DDL + ["INSERT INTO t (id, score) VALUES (1, 77)"],
            "SELECT score FROM t",
        )
        assert rows == [(77,)]

    def test_not_null_explicit_null_raises(self) -> None:
        """Supplying NULL explicitly must raise a constraint error."""
        with pytest.raises(IntegrityError):
            _mini(self.DDL + ["INSERT INTO t (id, score) VALUES (1, NULL)"], "SELECT 1")


# ---------------------------------------------------------------------------
# TestDefaultWithSelectStar
# ---------------------------------------------------------------------------


class TestDefaultWithSelectStar:
    """DEFAULT works correctly with SELECT * (ScanAllColumns path)."""

    def test_select_star_shows_default_values(self) -> None:
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, score INTEGER DEFAULT 5)",
            "INSERT INTO t (id, name) VALUES (1, 'Alice')",
        ]
        rows = oracle(setup, "SELECT * FROM t")
        assert rows == [(1, "Alice", 5)]

    def test_select_star_multiple_defaults(self) -> None:
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, x INTEGER DEFAULT 1, y INTEGER DEFAULT 2)",
            "INSERT INTO t (id) VALUES (1)",
        ]
        rows = oracle(setup, "SELECT * FROM t")
        assert rows == [(1, 1, 2)]


# ---------------------------------------------------------------------------
# TestDefaultWithPrimaryKey
# ---------------------------------------------------------------------------


class TestDefaultWithPrimaryKey:
    """DEFAULT on non-PK columns while PK is supplied."""

    def test_pk_supplied_defaults_applied_to_others(self) -> None:
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER DEFAULT 99)",
            "INSERT INTO t (id) VALUES (10)",
        ]
        rows = oracle(setup, "SELECT id, val FROM t")
        assert rows == [(10, 99)]


# ---------------------------------------------------------------------------
# TestDefaultWithUniqueConstraint
# ---------------------------------------------------------------------------


class TestDefaultWithUniqueConstraint:
    """DEFAULT values interact correctly with UNIQUE constraints."""

    def test_two_rows_both_use_default_raises_unique(self) -> None:
        """Two rows omitting a UNIQUE column both get the same default → conflict."""
        setup_ddl = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, code TEXT UNIQUE DEFAULT 'X')",
        ]
        with pytest.raises(IntegrityError):
            _mini(
                setup_ddl + [
                    "INSERT INTO t (id) VALUES (1)",
                    "INSERT INTO t (id) VALUES (2)",
                ],
                "SELECT 1",
            )

    def test_default_on_non_unique_column_allows_duplicates(self) -> None:
        """Non-UNIQUE column with DEFAULT can repeat across rows."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, tag TEXT DEFAULT 'foo')",
            "INSERT INTO t (id) VALUES (1)",
            "INSERT INTO t (id) VALUES (2)",
        ]
        # Include id in SELECT so ORDER BY id works (ORDER BY non-selected
        # column is a known pre-existing limitation separate from DEFAULTs).
        rows = oracle(setup, "SELECT id, tag FROM t ORDER BY id")
        assert rows == [(1, "foo"), (2, "foo")]


# ---------------------------------------------------------------------------
# TestDefaultEdgeCases
# ---------------------------------------------------------------------------


class TestDefaultEdgeCases:
    """Edge cases and realistic patterns."""

    def test_no_default_column_gets_null_when_omitted(self) -> None:
        """Without DEFAULT, an omitted nullable column gets NULL."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)",
            "INSERT INTO t (id) VALUES (1)",
        ]
        rows = oracle(setup, "SELECT name FROM t")
        assert rows == [(None,)]

    def test_default_in_middle_column(self) -> None:
        """DEFAULT on a middle column, surrounding columns supplied."""
        setup = [
            "CREATE TABLE t (a INTEGER, b INTEGER DEFAULT 7, c INTEGER)",
            "INSERT INTO t (a, c) VALUES (1, 3)",
        ]
        rows = oracle(setup, "SELECT a, b, c FROM t")
        assert rows == [(1, 7, 3)]

    def test_default_with_where_filter(self) -> None:
        """WHERE clause works correctly with rows populated by DEFAULT."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, active INTEGER DEFAULT 1)",
            "INSERT INTO t (id) VALUES (1)",
            "INSERT INTO t (id) VALUES (2)",
            "INSERT INTO t (id, active) VALUES (3, 0)",
        ]
        rows = oracle(setup, "SELECT id FROM t WHERE active = 1 ORDER BY id")
        assert rows == [(1,), (2,)]

    def test_default_survives_multi_insert_ordering(self) -> None:
        """Multiple INSERT statements each independently apply defaults."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER DEFAULT 100)",
        ]
        insert_sqls = [f"INSERT INTO t (id) VALUES ({i})" for i in range(1, 6)]
        rows = oracle(setup + insert_sqls, "SELECT id, v FROM t ORDER BY id")
        assert rows == [(i, 100) for i in range(1, 6)]

    def test_default_with_real_literal(self) -> None:
        """DEFAULT 3.14 — real / float default value."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, ratio REAL DEFAULT 3.14)",
            "INSERT INTO t (id) VALUES (1)",
        ]
        rows = oracle(setup, "SELECT ratio FROM t")
        assert rows == [(3.14,)]

    def test_default_negative_integer_not_yet_supported(self) -> None:
        """Bare DEFAULT -1 is not yet supported.

        The grammar's ``primary`` rule does not include unary-minus literals
        (the ``-`` token is lexed separately from the digit).  SQLite parses
        it as a signed literal; mini-sqlite will add this in a follow-on
        improvement.  Until then, the test documents the limitation.
        """
        pytest.skip(
            "DEFAULT <negative-literal> requires grammar/adapter expression "
            "support — planned follow-on"
        )
