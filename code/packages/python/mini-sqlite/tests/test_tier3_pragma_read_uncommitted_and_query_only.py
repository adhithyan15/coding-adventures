"""Tests for ``PRAGMA read_uncommitted`` and ``PRAGMA query_only``.

Both PRAGMAs are recognised as accept-and-store boolean settings
that round-trip per connection (default ``0``).  ``read_uncommitted``
selects SQLite's shared-cache isolation level — mini-sqlite has no
shared cache so the value is purely round-tripped with no semantic
effect.  ``query_only`` is **enforced** as of mini-sqlite 2.16.0:
when ON, mini-sqlite raises ``OperationalError: attempt to write
a readonly database`` for any DML or DDL statement, matching
SQLite's behaviour.  Callers that read the PRAGMA back to confirm
settings — common in ORMs and migration tools — see the
SQLite-compatible values.

Previously mini-sqlite returned ``[]`` for both reads (vs sqlite3's
``[(0,)]`` default) and silently dropped writes.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both_match_pragma(*pragmas: str, final_read: str) -> None:
    """Run a sequence of PRAGMAs then a final read, comparing oracles."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        for p in pragmas:
            c.execute(p)
    assert mini.execute(final_read).fetchall() == ref.execute(final_read).fetchall()


class TestReadUncommittedDefault:
    def test_default_is_zero(self) -> None:
        _both_match_pragma(final_read="PRAGMA read_uncommitted")


class TestReadUncommittedSet:
    def test_set_one(self) -> None:
        _both_match_pragma(
            "PRAGMA read_uncommitted = 1",
            final_read="PRAGMA read_uncommitted",
        )

    def test_set_zero(self) -> None:
        _both_match_pragma(
            "PRAGMA read_uncommitted = 1",
            "PRAGMA read_uncommitted = 0",
            final_read="PRAGMA read_uncommitted",
        )

    def test_set_on(self) -> None:
        _both_match_pragma(
            "PRAGMA read_uncommitted = ON",
            final_read="PRAGMA read_uncommitted",
        )

    def test_set_off(self) -> None:
        _both_match_pragma(
            "PRAGMA read_uncommitted = ON",
            "PRAGMA read_uncommitted = OFF",
            final_read="PRAGMA read_uncommitted",
        )


class TestQueryOnlyDefault:
    def test_default_is_zero(self) -> None:
        _both_match_pragma(final_read="PRAGMA query_only")


class TestQueryOnlySet:
    def test_set_one(self) -> None:
        _both_match_pragma(
            "PRAGMA query_only = 1",
            final_read="PRAGMA query_only",
        )

    def test_set_zero(self) -> None:
        _both_match_pragma(
            "PRAGMA query_only = 1",
            "PRAGMA query_only = 0",
            final_read="PRAGMA query_only",
        )

    def test_set_true(self) -> None:
        _both_match_pragma(
            "PRAGMA query_only = TRUE",
            final_read="PRAGMA query_only",
        )

    def test_set_false(self) -> None:
        _both_match_pragma(
            "PRAGMA query_only = TRUE",
            "PRAGMA query_only = FALSE",
            final_read="PRAGMA query_only",
        )


class TestPragmaListContains:
    """Both must be advertised in PRAGMA pragma_list."""

    def test_read_uncommitted_in_pragma_list(self) -> None:
        m = mini_sqlite.connect(":memory:")
        names = {r[0] for r in m.execute("PRAGMA pragma_list").fetchall()}
        assert "read_uncommitted" in names

    def test_query_only_in_pragma_list(self) -> None:
        m = mini_sqlite.connect(":memory:")
        names = {r[0] for r in m.execute("PRAGMA pragma_list").fetchall()}
        assert "query_only" in names


class TestConnectionIsolation:
    """Each connection carries its own state."""

    def test_read_uncommitted_isolation(self) -> None:
        a = mini_sqlite.connect(":memory:")
        b = mini_sqlite.connect(":memory:")
        a.execute("PRAGMA read_uncommitted = 1")
        assert b.execute("PRAGMA read_uncommitted").fetchall() == [(0,)]
        assert a.execute("PRAGMA read_uncommitted").fetchall() == [(1,)]

    def test_query_only_isolation(self) -> None:
        a = mini_sqlite.connect(":memory:")
        b = mini_sqlite.connect(":memory:")
        a.execute("PRAGMA query_only = 1")
        assert b.execute("PRAGMA query_only").fetchall() == [(0,)]
        assert a.execute("PRAGMA query_only").fetchall() == [(1,)]


class TestQueryOnlyEnforcement:
    """``query_only = 1`` now rejects writes (mini-sqlite 2.16.0).

    SQLite raises ``OperationalError: attempt to write a readonly
    database`` when ``query_only = 1`` and a write is attempted.
    The previous ``TestQueryOnlyEnforcementNotIncluded`` class pinned
    the divergence; this class pins the matching SQLite-compatible
    enforcement.  Coverage spans DML (INSERT/UPDATE/DELETE) and DDL
    (CREATE TABLE / CREATE INDEX / DROP TABLE / ALTER / CREATE VIEW),
    plus the negative-space checks (SELECT works, lifting the gate
    works, isolation across connections).
    """

    _ERR = "attempt to write a readonly database"

    # ------------------------------------------------------------------
    # DML: INSERT / UPDATE / DELETE all hit the gate
    # ------------------------------------------------------------------

    def _setup_with_row(self) -> mini_sqlite.Connection:
        m = mini_sqlite.connect(":memory:")
        m.execute("CREATE TABLE t (a INT)")
        m.execute("INSERT INTO t VALUES (1)")
        m.execute("PRAGMA query_only = 1")
        return m

    def test_insert_rejected(self) -> None:
        m = self._setup_with_row()
        try:
            m.execute("INSERT INTO t VALUES (2)")
        except mini_sqlite.OperationalError as e:
            assert self._ERR in str(e)
        else:
            raise AssertionError("expected OperationalError")

    def test_update_rejected(self) -> None:
        m = self._setup_with_row()
        try:
            m.execute("UPDATE t SET a = 99")
        except mini_sqlite.OperationalError as e:
            assert self._ERR in str(e)
        else:
            raise AssertionError("expected OperationalError")

    def test_delete_rejected(self) -> None:
        m = self._setup_with_row()
        try:
            m.execute("DELETE FROM t")
        except mini_sqlite.OperationalError as e:
            assert self._ERR in str(e)
        else:
            raise AssertionError("expected OperationalError")

    # ------------------------------------------------------------------
    # DDL: CREATE TABLE / CREATE INDEX / DROP TABLE / ALTER / CREATE VIEW
    # ------------------------------------------------------------------

    def test_create_table_rejected(self) -> None:
        m = mini_sqlite.connect(":memory:")
        m.execute("PRAGMA query_only = 1")
        try:
            m.execute("CREATE TABLE t (a INT)")
        except mini_sqlite.OperationalError as e:
            assert self._ERR in str(e)
        else:
            raise AssertionError("expected OperationalError")

    def test_create_index_rejected(self) -> None:
        m = mini_sqlite.connect(":memory:")
        m.execute("CREATE TABLE t (a INT)")
        m.execute("PRAGMA query_only = 1")
        try:
            m.execute("CREATE INDEX idx ON t(a)")
        except mini_sqlite.OperationalError as e:
            assert self._ERR in str(e)
        else:
            raise AssertionError("expected OperationalError")

    def test_drop_table_rejected(self) -> None:
        m = mini_sqlite.connect(":memory:")
        m.execute("CREATE TABLE t (a INT)")
        m.execute("PRAGMA query_only = 1")
        try:
            m.execute("DROP TABLE t")
        except mini_sqlite.OperationalError as e:
            assert self._ERR in str(e)
        else:
            raise AssertionError("expected OperationalError")

    def test_alter_table_rejected(self) -> None:
        m = mini_sqlite.connect(":memory:")
        m.execute("CREATE TABLE t (a INT)")
        m.execute("PRAGMA query_only = 1")
        try:
            m.execute("ALTER TABLE t RENAME TO t2")
        except mini_sqlite.OperationalError as e:
            assert self._ERR in str(e)
        else:
            raise AssertionError("expected OperationalError")

    def test_create_view_rejected(self) -> None:
        m = mini_sqlite.connect(":memory:")
        m.execute("CREATE TABLE t (a INT)")
        m.execute("PRAGMA query_only = 1")
        try:
            m.execute("CREATE VIEW v AS SELECT * FROM t")
        except mini_sqlite.OperationalError as e:
            assert self._ERR in str(e)
        else:
            raise AssertionError("expected OperationalError")

    # ------------------------------------------------------------------
    # Negative space: SELECT permitted; PRAGMA lift permitted; isolation
    # ------------------------------------------------------------------

    def test_select_still_permitted(self) -> None:
        m = self._setup_with_row()
        # SELECT is a pure read; the gate must let it through.
        assert m.execute("SELECT * FROM t").fetchall() == [(1,)]

    def test_pragma_can_lift_gate(self) -> None:
        m = self._setup_with_row()
        # ``PRAGMA query_only = 0`` always succeeds, even when the
        # gate is currently engaged — otherwise the connection would
        # be wedged read-only with no escape.
        m.execute("PRAGMA query_only = 0")
        m.execute("INSERT INTO t VALUES (2)")
        assert sorted(m.execute("SELECT a FROM t").fetchall()) == [(1,), (2,)]

    def test_gate_is_per_connection(self) -> None:
        # Connection A's query_only must not leak into connection B.
        a = mini_sqlite.connect(":memory:")
        b = mini_sqlite.connect(":memory:")
        for c in (a, b):
            c.execute("CREATE TABLE t (a INT)")
        a.execute("PRAGMA query_only = 1")
        # B's write proceeds normally.
        b.execute("INSERT INTO t VALUES (7)")
        assert b.execute("SELECT a FROM t").fetchall() == [(7,)]
        # A's write is still gated.
        try:
            a.execute("INSERT INTO t VALUES (8)")
        except mini_sqlite.OperationalError as e:
            assert self._ERR in str(e)
        else:
            raise AssertionError("expected OperationalError on A")
