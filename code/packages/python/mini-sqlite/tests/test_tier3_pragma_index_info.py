"""Tests for ``PRAGMA index_info`` and the corrected ``PRAGMA index_list`` shape.

``PRAGMA index_info(<name>)`` returns one row per indexed column with
the SQLite-standard ``(seqno, cid, name)`` triple, where:

* ``seqno`` is the 0-based position in the index key
* ``cid`` is the column's 0-based position in the parent table
* ``name`` is the indexed column's name

``PRAGMA index_list(<table>)`` previously returned three columns
``(seq, name, unique)``.  SQLite returns five; the two additions are
``origin`` (``'c'``/``'u'``/``'pk'``) and ``partial`` (always 0 here —
mini-sqlite doesn't support partial indexes).

Both pragmas are foundational for ORM index inspection.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


class TestIndexListShape:
    """``index_list`` now returns the SQLite-standard 5-column shape."""

    def test_columns_match_sqlite(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute("CREATE TABLE t(x INT)")
            c.execute("CREATE INDEX ix_x ON t(x)")
        m_desc = [d[0] for d in mini.execute("PRAGMA index_list(t)").description]
        r_desc = [d[0] for d in ref.execute("PRAGMA index_list(t)").description]
        assert m_desc == r_desc == ["seq", "name", "unique", "origin", "partial"]

    def test_user_index_origin_is_c(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE t(x INT)")
        mini.execute("CREATE INDEX ix_x ON t(x)")
        rows = mini.execute("PRAGMA index_list(t)").fetchall()
        assert rows == [(0, "ix_x", 0, "c", 0)]

    def test_unique_user_index(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE t(x INT)")
        mini.execute("CREATE UNIQUE INDEX ix_x ON t(x)")
        rows = mini.execute("PRAGMA index_list(t)").fetchall()
        # unique=1, origin='c', partial=0
        assert rows == [(0, "ix_x", 1, "c", 0)]


class TestIndexInfoSingleColumn:
    """Single-column index returns one row."""

    def test_basic(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute("CREATE TABLE t(x INT, y INT, z INT)")
            c.execute("CREATE INDEX ix_y ON t(y)")
        assert mini.execute("PRAGMA index_info(ix_y)").fetchall() \
            == ref.execute("PRAGMA index_info(ix_y)").fetchall() \
            == [(0, 1, "y")]


class TestIndexInfoCompositeIndex:
    """Multi-column indexes return one row per indexed column."""

    def test_two_columns(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute("CREATE TABLE t(x INT, y INT, z INT)")
            c.execute("CREATE INDEX ix_multi ON t(z, x)")
        # seqno=0 → cid=2 (z), seqno=1 → cid=0 (x)
        expected = [(0, 2, "z"), (1, 0, "x")]
        assert mini.execute("PRAGMA index_info(ix_multi)").fetchall() == expected
        assert ref.execute("PRAGMA index_info(ix_multi)").fetchall() == expected


class TestIndexInfoMissing:
    """An unknown index name returns zero rows (no error) — matches SQLite."""

    def test_missing_index_returns_empty(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute("CREATE TABLE t(x INT)")
        assert mini.execute("PRAGMA index_info(no_such)").fetchall() == []
        assert ref.execute("PRAGMA index_info(no_such)").fetchall() == []


class TestIndexInfoRequiresName:
    """Calling without an argument raises a ProgrammingError."""

    def test_no_arg(self) -> None:
        import pytest

        from mini_sqlite import errors as mini_errors

        mini = mini_sqlite.connect(":memory:")
        with pytest.raises(mini_errors.ProgrammingError):
            mini.execute("PRAGMA index_info").fetchall()


class TestIndexInfoListed:
    """``index_info`` appears in PRAGMA pragma_list."""

    def test_appears_in_pragma_list(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        names = {r[0] for r in mini.execute("PRAGMA pragma_list").fetchall()}
        assert "index_info" in names
