"""Tests for ``PRAGMA read_uncommitted`` and ``PRAGMA query_only``.

Both PRAGMAs are recognised as accept-and-store boolean settings
that round-trip per connection (default ``0``).  Mini-sqlite does
not honour the underlying semantics — ``read_uncommitted`` selects
SQLite's shared-cache isolation level (mini-sqlite has no shared
cache) and ``query_only`` should reject writes when ON (enforcement
is a future increment).  Callers that read the PRAGMA back to
confirm settings — common in ORMs and migration tools — see the
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


class TestQueryOnlyEnforcementNotIncluded:
    """Document the scope limit: ``query_only`` does NOT yet block writes.

    SQLite raises ``OperationalError: attempt to write a readonly
    database`` when ``query_only = 1`` and a write is attempted.
    Mini-sqlite currently allows the write through — enforcement is
    deferred to a future increment.  Pin the current behaviour so a
    future fix that wires the gate is detected (the test will need
    updating then, with a matching CHANGELOG note).
    """

    def test_query_only_does_not_block_writes_yet(self) -> None:
        m = mini_sqlite.connect(":memory:")
        m.execute("CREATE TABLE t (a INT)")
        m.execute("PRAGMA query_only = 1")
        # Currently no error — mini-sqlite does not enforce read-only mode.
        m.execute("INSERT INTO t VALUES (1)")
        assert m.execute("SELECT * FROM t").fetchall() == [(1,)]
