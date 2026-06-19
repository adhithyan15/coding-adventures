"""Oracle tests: column-level ON CONFLICT clause in CREATE TABLE.

SQLite lets each NOT NULL, UNIQUE, and PRIMARY KEY column constraint carry
its own conflict-resolution policy via an optional ``ON CONFLICT`` suffix::

    x INT NOT NULL ON CONFLICT IGNORE
    x INT UNIQUE ON CONFLICT REPLACE
    x INT PRIMARY KEY ON CONFLICT ABORT

Previously mini-sqlite raised a parse error for every such form because
the ``col_constraint`` grammar rule did not include the optional clause.
The fix adds a nested ``col_conflict_clause`` sub-rule so the adapter's
keyword-sequence matching remains unaffected (the ON/CONFLICT/action
keywords live in the sub-node, not in col_constraint's direct children).

Mini-sqlite does not enforce the per-column conflict action — it always
uses ABORT semantics on constraint violations.  The tests confirm that:

  1. All five actions (ROLLBACK, ABORT, FAIL, IGNORE, REPLACE) parse
     without error on all three constraint types.
  2. The schema is usable: rows can be inserted and queried normally.
  3. The explicit ABORT action (which *is* mini-sqlite's default) matches
     real SQLite behaviour end-to-end via oracle comparison.

Pattern: every oracle test runs the same SQL against both sqlite3 (the
reference engine) and mini_sqlite and asserts byte-for-byte identical
output.
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


def _our_exec(sql: str) -> None:
    mini_sqlite.connect(":memory:").execute(sql)


class TestNotNullOnConflict:
    """NOT NULL with each conflict action."""

    def test_not_null_ignore_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT NOT NULL ON CONFLICT IGNORE, y TEXT)")

    def test_not_null_replace_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT NOT NULL ON CONFLICT REPLACE, y TEXT)")

    def test_not_null_abort_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT NOT NULL ON CONFLICT ABORT, y TEXT)")

    def test_not_null_fail_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT NOT NULL ON CONFLICT FAIL, y TEXT)")

    def test_not_null_rollback_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT NOT NULL ON CONFLICT ROLLBACK, y TEXT)")

    def test_not_null_abort_insert_and_select(self) -> None:
        """ABORT is mini-sqlite's native behaviour — full oracle comparison."""
        sql = "SELECT x, y FROM t ORDER BY x"
        setup = [
            "CREATE TABLE t (x INT NOT NULL ON CONFLICT ABORT, y TEXT)",
            "INSERT INTO t VALUES (1, 'a')",
            "INSERT INTO t VALUES (2, 'b')",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_not_null_with_default(self) -> None:
        """ON CONFLICT may coexist with a DEFAULT clause."""
        sql = "SELECT x FROM t"
        setup = [
            "CREATE TABLE t (x INT NOT NULL ON CONFLICT ABORT DEFAULT 0, y TEXT)",
            "INSERT INTO t (y) VALUES ('hello')",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_pragma_table_info_not_null_flag(self) -> None:
        """NOT NULL ON CONFLICT must still register notnull=1 in table_info."""
        setup = ["CREATE TABLE t (x INT NOT NULL ON CONFLICT ABORT, y TEXT)"]
        sql = "PRAGMA table_info('t')"
        assert _our(sql, setup) == _ref(sql, setup)


class TestUniqueOnConflict:
    """UNIQUE with each conflict action."""

    def test_unique_ignore_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT UNIQUE ON CONFLICT IGNORE, y TEXT)")

    def test_unique_replace_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT UNIQUE ON CONFLICT REPLACE, y TEXT)")

    def test_unique_abort_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT UNIQUE ON CONFLICT ABORT, y TEXT)")

    def test_unique_fail_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT UNIQUE ON CONFLICT FAIL, y TEXT)")

    def test_unique_rollback_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT UNIQUE ON CONFLICT ROLLBACK, y TEXT)")

    def test_unique_abort_insert_and_select(self) -> None:
        sql = "SELECT x, y FROM t ORDER BY x"
        setup = [
            "CREATE TABLE t (x INT UNIQUE ON CONFLICT ABORT, y TEXT)",
            "INSERT INTO t VALUES (1, 'a')",
            "INSERT INTO t VALUES (2, 'b')",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_pragma_table_info_unique_abort(self) -> None:
        setup = ["CREATE TABLE t (x INT UNIQUE ON CONFLICT ABORT, y TEXT)"]
        sql = "PRAGMA table_info('t')"
        assert _our(sql, setup) == _ref(sql, setup)


class TestPrimaryKeyOnConflict:
    """PRIMARY KEY with each conflict action."""

    def test_pk_ignore_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT PRIMARY KEY ON CONFLICT IGNORE, y TEXT)")

    def test_pk_replace_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT PRIMARY KEY ON CONFLICT REPLACE, y TEXT)")

    def test_pk_abort_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT PRIMARY KEY ON CONFLICT ABORT, y TEXT)")

    def test_pk_fail_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT PRIMARY KEY ON CONFLICT FAIL, y TEXT)")

    def test_pk_rollback_parses(self) -> None:
        _our_exec("CREATE TABLE t (x INT PRIMARY KEY ON CONFLICT ROLLBACK, y TEXT)")

    def test_pk_abort_insert_and_select(self) -> None:
        sql = "SELECT x, y FROM t ORDER BY x"
        setup = [
            "CREATE TABLE t (x INT PRIMARY KEY ON CONFLICT ABORT, y TEXT)",
            "INSERT INTO t VALUES (1, 'a')",
            "INSERT INTO t VALUES (2, 'b')",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_pk_autoincrement_with_conflict(self) -> None:
        """PRIMARY KEY AUTOINCREMENT ON CONFLICT must parse."""
        _our_exec(
            "CREATE TABLE t (x INTEGER PRIMARY KEY AUTOINCREMENT ON CONFLICT REPLACE, y TEXT)"
        )


class TestMixedConstraints:
    """ON CONFLICT may coexist with other column and table constraints."""

    def test_multiple_cols_different_actions(self) -> None:
        """Different ON CONFLICT actions on different columns in one table."""
        sql = "SELECT x, y FROM t ORDER BY x"
        setup = [
            "CREATE TABLE t ("
            "x INT NOT NULL ON CONFLICT ABORT, "
            "y TEXT UNIQUE ON CONFLICT IGNORE"
            ")",
            "INSERT INTO t VALUES (1, 'a')",
            "INSERT INTO t VALUES (2, 'b')",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_col_conflict_plus_collate(self) -> None:
        """UNIQUE ON CONFLICT and COLLATE NOCASE on the same column."""
        sql = "SELECT x FROM t ORDER BY x"
        setup = [
            "CREATE TABLE t (x TEXT UNIQUE ON CONFLICT ABORT COLLATE NOCASE)",
            "INSERT INTO t VALUES ('hello')",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_col_conflict_plus_table_constraint(self) -> None:
        """Column-level ON CONFLICT and table-level constraint in the same DDL."""
        sql = "SELECT x, y FROM t ORDER BY x"
        setup = [
            "CREATE TABLE t (x INT NOT NULL ON CONFLICT ABORT, y INT, UNIQUE(y))",
            "INSERT INTO t VALUES (1, 10)",
            "INSERT INTO t VALUES (2, 20)",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_without_rowid_plus_pk_conflict(self) -> None:
        """PRIMARY KEY ON CONFLICT ABORT plus WITHOUT ROWID must parse."""
        _our_exec(
            "CREATE TABLE t (x INT PRIMARY KEY ON CONFLICT ABORT, y TEXT) WITHOUT ROWID"
        )
