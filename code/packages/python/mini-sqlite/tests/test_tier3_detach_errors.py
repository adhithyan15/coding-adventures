"""Tests for ``DETACH DATABASE`` error wording (SQLite parity).

SQLite raises specific ``OperationalError`` messages for DETACH:

* ``DETACH DATABASE <name>`` where ``<name>`` is not an attached
  schema → ``OperationalError: no such database: <name>``
* ``DETACH DATABASE main`` → ``OperationalError: cannot detach
  database main``

Mini-sqlite previously silently returned an empty result for all
DETACH statements (no-op).  This was wrong: callers that rely on the
error to detect mis-spelled schema names or invalid detach attempts
would silently succeed and miss the problem.

``ATTACH DATABASE`` remains a no-op (returns success) because
mini-sqlite does not implement multi-database schema routing and there
is no meaningful error to produce for a fresh ATTACH.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


class TestDetachNonExistentSchema:
    """DETACH of a schema that was never attached → 'no such database'."""

    def _both_raise_no_such_db(self, sql: str) -> None:
        """Assert mini and sqlite3 both raise OperationalError with the
        same 'no such database' message."""
        ref = sqlite3.connect(":memory:")
        try:
            ref.execute(sql)
            ref_err = None
        except sqlite3.OperationalError as e:
            ref_err = str(e)

        m = mini_sqlite.connect(":memory:")
        try:
            m.execute(sql)
            mini_err = None
        except mini_sqlite.OperationalError as e:
            mini_err = str(e)

        assert ref_err is not None, "sqlite3 should have raised OperationalError"
        assert mini_err is not None, "mini_sqlite should have raised OperationalError"
        assert mini_err == ref_err, f"message mismatch: mini={mini_err!r} ref={ref_err!r}"

    def test_detach_database_aux(self) -> None:
        self._both_raise_no_such_db("DETACH DATABASE aux")

    def test_detach_database_temp(self) -> None:
        self._both_raise_no_such_db("DETACH DATABASE temp")

    def test_detach_no_keyword(self) -> None:
        self._both_raise_no_such_db("DETACH aux")

    def test_detach_quoted_name(self) -> None:
        self._both_raise_no_such_db('DETACH DATABASE "mydb"')


class TestDetachMain:
    """DETACH main → 'cannot detach database main'."""

    def test_detach_main(self) -> None:
        ref = sqlite3.connect(":memory:")
        try:
            ref.execute("DETACH DATABASE main")
            ref_err = None
        except sqlite3.OperationalError as e:
            ref_err = str(e)

        m = mini_sqlite.connect(":memory:")
        try:
            m.execute("DETACH DATABASE main")
            mini_err = None
        except mini_sqlite.OperationalError as e:
            mini_err = str(e)

        assert ref_err is not None
        assert mini_err is not None
        assert mini_err == ref_err

    def test_detach_main_no_keyword(self) -> None:
        ref = sqlite3.connect(":memory:")
        try:
            ref.execute("DETACH main")
            ref_err = None
        except sqlite3.OperationalError as e:
            ref_err = str(e)

        m = mini_sqlite.connect(":memory:")
        try:
            m.execute("DETACH main")
            mini_err = None
        except mini_sqlite.OperationalError as e:
            mini_err = str(e)

        assert ref_err is not None
        assert mini_err is not None
        assert mini_err == ref_err


class TestAttachStillSucceeds:
    """ATTACH DATABASE remains a no-op (returns success)."""

    def test_attach_memory(self) -> None:
        m = mini_sqlite.connect(":memory:")
        result = m.execute("ATTACH ':memory:' AS aux")
        assert result.fetchall() == []

    def test_attach_does_not_raise(self) -> None:
        m = mini_sqlite.connect(":memory:")
        # Should complete without raising — the attached DB is a no-op
        # but mini-sqlite accepts the statement for compatibility.
        m.execute("ATTACH DATABASE ':memory:' AS helper")
