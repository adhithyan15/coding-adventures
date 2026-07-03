"""End-to-end tests for ``sqlite_master`` / ``sqlite_schema`` introspection.

SQLite exposes the schema catalog through a virtual table named both
``sqlite_master`` (legacy) and ``sqlite_schema`` (SQLite 3.33+).  ORMs,
migration tools, and ``.tables`` shells all rely on it.

Mini-sqlite synthesizes the rows from current backend state on every
scan — no storage, no maintenance.  These tests verify that the
synthesized table matches SQLite's structure tightly enough that
migration-tool queries return identical rows.  We don't oracle-compare
the literal ``sql`` text (mini-sqlite reconstructs it from ColumnDef
metadata, so whitespace and keyword casing may differ from the user's
original) — but ``type``, ``name``, and ``tbl_name`` must match.
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite
from mini_sqlite import errors as mini_errors


def _setup_both(*ddls: str) -> tuple:
    """Run *ddls* on both engines and return (mini, ref) connections."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        for sql in ddls:
            c.execute(sql)
    return mini, ref


class TestBasicListing:
    """The migration-tool happy path: list tables and indexes."""

    def test_list_user_tables(self) -> None:
        mini, ref = _setup_both(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)",
            "CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER)",
        )
        q = "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
        assert mini.execute(q).fetchall() == ref.execute(q).fetchall()

    def test_count_tables(self) -> None:
        mini, ref = _setup_both(
            "CREATE TABLE a (x INT)",
            "CREATE TABLE b (x INT)",
            "CREATE TABLE c (x INT)",
        )
        q = "SELECT COUNT(*) FROM sqlite_master WHERE type='table'"
        assert mini.execute(q).fetchone() == ref.execute(q).fetchone() == (3,)

    def test_empty_database_has_no_rows(self) -> None:
        mini, ref = _setup_both()
        q = "SELECT COUNT(*) FROM sqlite_master"
        assert mini.execute(q).fetchone() == ref.execute(q).fetchone()

    def test_list_indexes(self) -> None:
        # User-created indexes show up; auto-indexes from PRIMARY KEY do
        # not (SQLite hides those whose sql is NULL via ORM convention,
        # but they're still in sqlite_master with sql=NULL).  We filter
        # to user indexes only by requiring sql IS NOT NULL.
        mini, ref = _setup_both(
            "CREATE TABLE t (x INTEGER, y INTEGER)",
            "CREATE INDEX ix_t_x ON t (x)",
            "CREATE UNIQUE INDEX ix_t_y ON t (y)",
        )
        q = (
            "SELECT name FROM sqlite_master "
            "WHERE type='index' AND sql IS NOT NULL ORDER BY name"
        )
        assert mini.execute(q).fetchall() == ref.execute(q).fetchall()


class TestSchemaAlias:
    """``sqlite_schema`` is the modern alias for ``sqlite_master``."""

    def test_sqlite_schema_returns_same_names(self) -> None:
        mini, ref = _setup_both(
            "CREATE TABLE users (id INTEGER PRIMARY KEY)",
            "CREATE TABLE orders (id INTEGER PRIMARY KEY)",
        )
        q1 = "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
        q2 = "SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name"
        # Both engines: alias yields identical rows.
        assert mini.execute(q1).fetchall() == mini.execute(q2).fetchall()
        assert ref.execute(q1).fetchall() == ref.execute(q2).fetchall()


class TestProjection:
    """Each column projects independently."""

    def test_select_all_columns(self) -> None:
        mini, ref = _setup_both("CREATE TABLE t (x INTEGER, y TEXT)")
        # rowid/rootpage value differs between engines (mini returns 0,
        # sqlite picks a real page number) — exclude rootpage.
        q = "SELECT type, name, tbl_name FROM sqlite_master WHERE type='table'"
        assert mini.execute(q).fetchall() == ref.execute(q).fetchall()

    def test_tbl_name_equals_name_for_tables(self) -> None:
        mini, ref = _setup_both("CREATE TABLE foo (x INT)")
        # SQLite invariant: for ``type='table'`` rows, ``tbl_name == name``.
        q = "SELECT name, tbl_name FROM sqlite_master WHERE type='table'"
        m = mini.execute(q).fetchall()
        r = ref.execute(q).fetchall()
        assert m == r
        for name, tbl in m:
            assert name == tbl

    def test_tbl_name_for_index_points_to_indexed_table(self) -> None:
        mini, ref = _setup_both(
            "CREATE TABLE t (x INTEGER)",
            "CREATE INDEX ix_x ON t (x)",
        )
        q = (
            "SELECT name, tbl_name FROM sqlite_master "
            "WHERE type='index' AND sql IS NOT NULL"
        )
        assert mini.execute(q).fetchall() == ref.execute(q).fetchall()
        m = mini.execute(q).fetchall()
        assert m == [("ix_x", "t")]


class TestReadOnly:
    """Mini-sqlite rejects modifications to sqlite_master, matching SQLite."""

    def test_insert_rejected(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        with pytest.raises(mini_errors.Error):
            mini.execute(
                "INSERT INTO sqlite_master VALUES "
                "('table', 'x', 'x', 0, 'CREATE TABLE x(a INT)')"
            )

    def test_drop_table_rejected(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        with pytest.raises(mini_errors.Error):
            mini.execute("DROP TABLE sqlite_master")

    def test_create_table_with_reserved_name_rejected(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        with pytest.raises(mini_errors.Error):
            mini.execute("CREATE TABLE sqlite_master (a INT)")

    def test_real_sqlite_also_rejects_modification(self) -> None:
        # Sanity-check that real SQLite forbids the same writes — if a
        # future SQLite ever loosens this, the test will start failing
        # and we'll know to revisit the mini-sqlite policy.
        ref = sqlite3.connect(":memory:")
        with pytest.raises(sqlite3.Error):
            ref.execute("DROP TABLE sqlite_master")


class TestWithExistingTier:
    """The existing ``test_oracle_schema_visible_in_sqlite3`` test
    (file backend) demonstrates that mini→sqlite3 schema roundtrip works
    via the on-disk format.  This class targets the *in-memory* path
    that previously had no sqlite_master at all.
    """

    def test_orm_style_table_discovery(self) -> None:
        """The query an ORM uses to enumerate tables works."""
        mini, ref = _setup_both(
            "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY)",
            "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT NOT NULL)",
            "CREATE TABLE posts (id INTEGER PRIMARY KEY, body TEXT)",
        )
        q = "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
        assert mini.execute(q).fetchall() == ref.execute(q).fetchall()
