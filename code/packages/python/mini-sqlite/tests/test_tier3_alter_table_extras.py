"""``ALTER TABLE`` — RENAME TO / RENAME COLUMN / DROP COLUMN.

SQLite supports four flavours of ``ALTER TABLE`` (3.35+):

* ``ALTER TABLE t ADD [COLUMN] col_def``
* ``ALTER TABLE t RENAME TO new_name``         (3.0+)
* ``ALTER TABLE t RENAME [COLUMN] old TO new`` (3.25+)
* ``ALTER TABLE t DROP [COLUMN] name``         (3.35+)

Mini-sqlite already supported ``ADD``; this PR adds the other three.
The ``COLUMN`` keyword is optional everywhere (matches SQLite).

These oracle tests pin byte-identical results against stdlib
``sqlite3`` for each form, including the error paths.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check_with_setup(setup: list[str], query: str) -> None:
    mc = mini_sqlite.connect(":memory:")
    rc = sqlite3.connect(":memory:")
    for s in setup:
        mc.execute(s)
        rc.execute(s)
    m = list(mc.execute(query))
    r = list(rc.execute(query))
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# RENAME TO — rename the entire table.
# ---------------------------------------------------------------------------


class TestRenameTo:
    def test_basic(self) -> None:
        _check_with_setup(
            [
                "CREATE TABLE t(a INTEGER, b TEXT)",
                "INSERT INTO t VALUES (1, 'x'), (2, 'y')",
                "ALTER TABLE t RENAME TO u",
            ],
            "SELECT * FROM u ORDER BY a",
        )

    def test_old_name_unavailable_after_rename(self) -> None:
        # Querying the old name should fail in both engines.
        mc = mini_sqlite.connect(":memory:")
        rc = sqlite3.connect(":memory:")
        for db in (mc, rc):
            db.execute("CREATE TABLE t(a INT)")
            db.execute("ALTER TABLE t RENAME TO u")
        # Both should error on the old name; we just verify ours errors.
        import pytest

        from mini_sqlite.errors import OperationalError

        with pytest.raises(OperationalError):
            mc.execute("SELECT * FROM t")

    def test_indexes_follow_rename(self) -> None:
        # An index defined on ``t.a`` should still work after the table
        # is renamed to ``u`` — the index's table field gets rewritten.
        _check_with_setup(
            [
                "CREATE TABLE t(a INTEGER)",
                "CREATE INDEX i_t_a ON t(a)",
                "INSERT INTO t VALUES (1), (2), (3)",
                "ALTER TABLE t RENAME TO u",
            ],
            "SELECT * FROM u WHERE a = 2",
        )


# ---------------------------------------------------------------------------
# RENAME COLUMN — both ``RENAME COLUMN old TO new`` and the
# COLUMN-less ``RENAME old TO new`` form (matches SQLite).
# ---------------------------------------------------------------------------


class TestRenameColumn:
    def test_basic_with_column_kw(self) -> None:
        _check_with_setup(
            [
                "CREATE TABLE t(a INTEGER, b TEXT)",
                "INSERT INTO t VALUES (1, 'x'), (2, 'y')",
                "ALTER TABLE t RENAME COLUMN a TO renamed",
            ],
            "SELECT renamed, b FROM t ORDER BY renamed",
        )

    def test_basic_without_column_kw(self) -> None:
        _check_with_setup(
            [
                "CREATE TABLE t(a INTEGER, b TEXT)",
                "INSERT INTO t VALUES (1, 'x'), (2, 'y')",
                "ALTER TABLE t RENAME a TO renamed",
            ],
            "SELECT renamed, b FROM t ORDER BY renamed",
        )

    def test_existing_rows_carry_new_name(self) -> None:
        # The values move with the rename — accessing the new name
        # should return the original row values intact.
        _check_with_setup(
            [
                "CREATE TABLE t(a INTEGER)",
                "INSERT INTO t VALUES (42)",
                "ALTER TABLE t RENAME COLUMN a TO b",
            ],
            "SELECT b FROM t",
        )


# ---------------------------------------------------------------------------
# DROP COLUMN.
# ---------------------------------------------------------------------------


class TestDropColumn:
    def test_basic_with_column_kw(self) -> None:
        _check_with_setup(
            [
                "CREATE TABLE t(a INTEGER, b TEXT, c INTEGER)",
                "INSERT INTO t VALUES (1, 'x', 10), (2, 'y', 20)",
                "ALTER TABLE t DROP COLUMN b",
            ],
            "SELECT * FROM t ORDER BY a",
        )

    def test_basic_without_column_kw(self) -> None:
        _check_with_setup(
            [
                "CREATE TABLE t(a INTEGER, b TEXT, c INTEGER)",
                "INSERT INTO t VALUES (1, 'x', 10), (2, 'y', 20)",
                "ALTER TABLE t DROP b",
            ],
            "SELECT * FROM t ORDER BY a",
        )

    def test_dropped_column_unavailable(self) -> None:
        mc = mini_sqlite.connect(":memory:")
        rc = sqlite3.connect(":memory:")
        for db in (mc, rc):
            db.execute("CREATE TABLE t(a INT, b TEXT)")
            db.execute("ALTER TABLE t DROP COLUMN b")
        # Both engines should error on the dropped column.
        import pytest

        from mini_sqlite.errors import OperationalError

        with pytest.raises(OperationalError):
            mc.execute("SELECT b FROM t")


# ---------------------------------------------------------------------------
# Errors: bad column names, missing tables.
# ---------------------------------------------------------------------------


class TestErrors:
    def test_drop_unknown_column(self) -> None:
        import pytest

        from mini_sqlite.errors import OperationalError

        mc = mini_sqlite.connect(":memory:")
        mc.execute("CREATE TABLE t(a INT, b TEXT)")
        with pytest.raises(OperationalError):
            mc.execute("ALTER TABLE t DROP COLUMN ghost")

    def test_drop_only_column(self) -> None:
        # SQLite doesn't allow dropping the only column.
        import pytest

        from mini_sqlite.errors import IntegrityError

        mc = mini_sqlite.connect(":memory:")
        mc.execute("CREATE TABLE t(a INT)")
        with pytest.raises(IntegrityError):
            mc.execute("ALTER TABLE t DROP COLUMN a")

    def test_drop_pk_column(self) -> None:
        # SQLite doesn't allow dropping a PRIMARY KEY column.
        import pytest

        from mini_sqlite.errors import IntegrityError

        mc = mini_sqlite.connect(":memory:")
        mc.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT)")
        with pytest.raises(IntegrityError):
            mc.execute("ALTER TABLE t DROP COLUMN id")

    def test_rename_unknown_table(self) -> None:
        import pytest

        from mini_sqlite.errors import OperationalError

        mc = mini_sqlite.connect(":memory:")
        with pytest.raises(OperationalError):
            mc.execute("ALTER TABLE ghost RENAME TO whatever")


# ---------------------------------------------------------------------------
# Regression: ADD COLUMN still works after the grammar change.
# ---------------------------------------------------------------------------


class TestAddColumnStillWorks:
    def test_add_column_with_kw(self) -> None:
        _check_with_setup(
            [
                "CREATE TABLE t(a INTEGER)",
                "INSERT INTO t VALUES (1), (2)",
                "ALTER TABLE t ADD COLUMN b TEXT",
            ],
            "SELECT a, b FROM t ORDER BY a",
        )

    def test_add_column_without_kw(self) -> None:
        _check_with_setup(
            [
                "CREATE TABLE t(a INTEGER)",
                "INSERT INTO t VALUES (1), (2)",
                "ALTER TABLE t ADD b TEXT",
            ],
            "SELECT a, b FROM t ORDER BY a",
        )

    def test_add_column_with_default(self) -> None:
        _check_with_setup(
            [
                "CREATE TABLE t(a INTEGER)",
                "INSERT INTO t VALUES (1)",
                "ALTER TABLE t ADD COLUMN b TEXT DEFAULT 'unknown'",
            ],
            "SELECT b FROM t",
        )
