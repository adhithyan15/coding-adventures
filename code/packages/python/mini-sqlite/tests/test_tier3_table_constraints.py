"""Oracle tests: table-level constraints in CREATE TABLE.

SQLite allows PRIMARY KEY and UNIQUE to be expressed as table-level
constraints that appear after the column list::

    CREATE TABLE t (x INT, y INT, PRIMARY KEY(x))
    CREATE TABLE t (x INT, y INT, UNIQUE(x, y))
    CREATE TABLE t (x INT, y INT, PRIMARY KEY(x, y))
    CREATE TABLE t (x INT, y TEXT, x INT, CHECK(x > 0))

Previously mini-sqlite only understood column-level constraints
(``col INT PRIMARY KEY``) and raised a parse error for the table-level
form.  The fix adds a ``table_constraint`` rule to the PEG grammar and
wires it into the adapter so that PRIMARY KEY and UNIQUE promote the flag
onto the matching column definition(s).

Pattern: every test runs the same SQL against both sqlite3 (reference)
and mini_sqlite, and asserts byte-for-byte identical output.
"""

from __future__ import annotations

import contextlib
import sqlite3

import pytest

import mini_sqlite
from mini_sqlite.errors import IntegrityError


def _ref(sql: str, setup: list[str] | None = None) -> list[tuple]:
    con = sqlite3.connect(":memory:")
    if setup:
        for s in setup:
            with contextlib.suppress(Exception):
                con.execute(s)
    return con.execute(sql).fetchall()


def _our(sql: str, setup: list[str] | None = None) -> list[tuple]:
    con = mini_sqlite.connect(":memory:")
    if setup:
        for s in setup:
            with contextlib.suppress(Exception):
                con.execute(s)
    return con.execute(sql).fetchall()


def _ref_exec(sql: str) -> None:
    """Execute DDL against sqlite3 (for smoke-testing parse succeeds)."""
    con = sqlite3.connect(":memory:")
    con.execute(sql)


def _our_exec(sql: str) -> None:
    """Execute DDL against mini_sqlite (for smoke-testing parse succeeds)."""
    con = mini_sqlite.connect(":memory:")
    con.execute(sql)


class TestTableLevelPrimaryKey:
    """Single-column table-level PRIMARY KEY."""

    def test_single_col_pk_parse(self) -> None:
        """CREATE TABLE with a table-level PRIMARY KEY must parse without error."""
        _our_exec("CREATE TABLE t (x INT, y TEXT, PRIMARY KEY(x))")

    def test_single_col_pk_insert_and_select(self) -> None:
        sql = "SELECT x, y FROM t"
        setup = [
            "CREATE TABLE t (x INT, y TEXT, PRIMARY KEY(x))",
            "INSERT INTO t VALUES (1, 'a')",
            "INSERT INTO t VALUES (2, 'b')",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_single_col_pk_enforces_uniqueness(self) -> None:
        """Duplicate PK value must raise an IntegrityError."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (x INT, y TEXT, PRIMARY KEY(x))")
        con.execute("INSERT INTO t VALUES (1, 'a')")
        with pytest.raises(IntegrityError):
            con.execute("INSERT INTO t VALUES (1, 'b')")

    def test_single_col_pk_insert_distinct_rows(self) -> None:
        """Rows with distinct PK values all survive — no false uniqueness collision."""
        sql = "SELECT x, y FROM t ORDER BY x"
        setup = [
            "CREATE TABLE t (x INT, y TEXT, PRIMARY KEY(x))",
            "INSERT INTO t VALUES (10, 'a')",
            "INSERT INTO t VALUES (20, 'b')",
            "INSERT INTO t VALUES (30, 'c')",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_pk_integer_autorowid(self) -> None:
        """INTEGER PRIMARY KEY at table-level enables auto-assigned rowid."""
        sql = "SELECT x FROM t"
        setup = [
            "CREATE TABLE t (x INTEGER, y TEXT, PRIMARY KEY(x))",
            "INSERT INTO t (y) VALUES ('hello')",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_table_info_pk_flag(self) -> None:
        """PRAGMA table_info must report pk=1 for the PK column."""
        setup = ["CREATE TABLE t (x INT, y TEXT, PRIMARY KEY(x))"]
        sql = "PRAGMA table_info('t')"
        assert _our(sql, setup) == _ref(sql, setup)


class TestTableLevelMultiColumnPrimaryKey:
    """Multi-column (composite) table-level PRIMARY KEY."""

    def test_two_col_pk_parse(self) -> None:
        """CREATE TABLE with a two-column PRIMARY KEY must parse."""
        _our_exec("CREATE TABLE t (x INT, y INT, PRIMARY KEY(x, y))")

    def test_two_col_pk_insert_and_select(self) -> None:
        sql = "SELECT x, y FROM t ORDER BY x, y"
        setup = [
            "CREATE TABLE t (x INT, y INT, PRIMARY KEY(x, y))",
            "INSERT INTO t VALUES (1, 1)",
            "INSERT INTO t VALUES (1, 2)",
            "INSERT INTO t VALUES (2, 1)",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_three_col_pk_parse(self) -> None:
        """CREATE TABLE with a three-column PRIMARY KEY must parse."""
        _our_exec("CREATE TABLE t (a INT, b INT, c TEXT, PRIMARY KEY(a, b, c))")


class TestTableLevelUnique:
    """Table-level UNIQUE constraint."""

    def test_single_col_unique_parse(self) -> None:
        _our_exec("CREATE TABLE t (x INT, y INT, UNIQUE(x))")

    def test_single_col_unique_enforces(self) -> None:
        """UNIQUE constraint must reject duplicate values."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (x INT, y INT, UNIQUE(x))")
        con.execute("INSERT INTO t VALUES (1, 10)")
        with pytest.raises(IntegrityError):
            con.execute("INSERT INTO t VALUES (1, 20)")

    def test_two_col_unique_parse(self) -> None:
        _our_exec("CREATE TABLE t (x INT, y INT, UNIQUE(x, y))")

    def test_unique_allows_nulls(self) -> None:
        """UNIQUE does not reject NULL values (matches SQLite behaviour)."""
        sql = "SELECT x, y FROM t"
        setup = [
            "CREATE TABLE t (x INT, y INT, UNIQUE(x))",
            "INSERT INTO t VALUES (NULL, 1)",
            "INSERT INTO t VALUES (NULL, 2)",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_table_info_unique_flag(self) -> None:
        setup = ["CREATE TABLE t (x INT, y INT, UNIQUE(x))"]
        sql = "PRAGMA table_info('t')"
        assert _our(sql, setup) == _ref(sql, setup)


class TestTableLevelCheck:
    """Table-level CHECK constraint (parsed and accepted; not enforced)."""

    def test_check_parse(self) -> None:
        """CREATE TABLE with a CHECK constraint must parse without error."""
        _our_exec("CREATE TABLE t (x INT, y INT, CHECK(x > 0))")

    def test_check_with_pk(self) -> None:
        """CHECK and PRIMARY KEY together must parse."""
        _our_exec("CREATE TABLE t (x INT, y INT, PRIMARY KEY(x), CHECK(y >= 0))")


class TestTableLevelForeignKey:
    """Table-level FOREIGN KEY (parsed and accepted; not enforced)."""

    def test_foreign_key_parse(self) -> None:
        _our_exec(
            "CREATE TABLE orders (id INT PRIMARY KEY, "
            "customer_id INT, FOREIGN KEY(customer_id) REFERENCES customers(id))"
        )

    def test_fk_with_pk(self) -> None:
        _our_exec(
            "CREATE TABLE line_item (order_id INT, product_id INT, qty INT, "
            "PRIMARY KEY(order_id, product_id), "
            "FOREIGN KEY(order_id) REFERENCES orders(id))"
        )


class TestMixedConstraints:
    """Column-level and table-level constraints may be mixed."""

    def test_col_level_and_table_level(self) -> None:
        sql = "SELECT x, y, z FROM t"
        setup = [
            "CREATE TABLE t (x INT NOT NULL, y INT, z TEXT, UNIQUE(y))",
            "INSERT INTO t VALUES (1, 10, 'a')",
            "INSERT INTO t VALUES (2, 20, 'b')",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_without_rowid_plus_table_pk(self) -> None:
        """WITHOUT ROWID with a table-level PRIMARY KEY must parse and work."""
        sql = "SELECT x, y FROM t ORDER BY x"
        setup = [
            "CREATE TABLE t (x INT, y TEXT, PRIMARY KEY(x)) WITHOUT ROWID",
            "INSERT INTO t VALUES (1, 'a')",
            "INSERT INTO t VALUES (2, 'b')",
        ]
        assert _our(sql, setup) == _ref(sql, setup)

    def test_strict_plus_table_pk(self) -> None:
        """STRICT combined with a table-level PRIMARY KEY must parse."""
        _our_exec("CREATE TABLE t (x INTEGER, y TEXT, PRIMARY KEY(x)) STRICT")
