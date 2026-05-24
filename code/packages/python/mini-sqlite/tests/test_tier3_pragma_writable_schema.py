"""Tests for ``PRAGMA writable_schema``.

SQLite's ``writable_schema`` PRAGMA controls whether direct
UPDATE/INSERT/DELETE statements against ``sqlite_master`` are allowed
— used by repair tools and migration frameworks to rename tables or
fix corrupted catalog rows.  Default is OFF (0).

Mini-sqlite synthesises ``sqlite_master`` on every read (it's not a
real table backed by storage), so honouring writes through it would
be a much larger change.  We instead accept and *round-trip* the
PRAGMA value per connection — defensive callers that read the
PRAGMA before deciding whether to run a repair flow see the
expected value, and migrations that just toggle it without using
the writability don't trip on ``unrecognised PRAGMA``.

This brings the PRAGMA's surface behaviour in line with sqlite3
(read returns the current int, write accepts ON/OFF/0/1/TRUE/FALSE,
the value persists for the lifetime of the connection).
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both_match_pragma(*pragmas: str, final_read: str) -> None:
    """Run a sequence of PRAGMA statements then a final read; compare oracles."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        for p in pragmas:
            c.execute(p)
    assert mini.execute(final_read).fetchall() == ref.execute(final_read).fetchall()


class TestDefault:
    def test_default_is_zero(self) -> None:
        _both_match_pragma(final_read="PRAGMA writable_schema")


class TestSetIntegers:
    def test_set_one(self) -> None:
        _both_match_pragma(
            "PRAGMA writable_schema = 1",
            final_read="PRAGMA writable_schema",
        )

    def test_set_zero(self) -> None:
        _both_match_pragma(
            "PRAGMA writable_schema = 1",
            "PRAGMA writable_schema = 0",
            final_read="PRAGMA writable_schema",
        )


class TestSetBoolKeywords:
    """SQLite accepts ON/OFF/TRUE/FALSE/YES/NO as boolean PRAGMA values."""

    def test_set_on(self) -> None:
        _both_match_pragma(
            "PRAGMA writable_schema = ON",
            final_read="PRAGMA writable_schema",
        )

    def test_set_off(self) -> None:
        _both_match_pragma(
            "PRAGMA writable_schema = ON",
            "PRAGMA writable_schema = OFF",
            final_read="PRAGMA writable_schema",
        )

    def test_set_true(self) -> None:
        _both_match_pragma(
            "PRAGMA writable_schema = TRUE",
            final_read="PRAGMA writable_schema",
        )

    def test_set_false(self) -> None:
        _both_match_pragma(
            "PRAGMA writable_schema = TRUE",
            "PRAGMA writable_schema = FALSE",
            final_read="PRAGMA writable_schema",
        )


class TestPragmaListContainsWritableSchema:
    """PRAGMA pragma_list must advertise writable_schema as supported."""

    def test_writable_schema_in_pragma_list(self) -> None:
        m = mini_sqlite.connect(":memory:")
        names = {r[0] for r in m.execute("PRAGMA pragma_list").fetchall()}
        assert "writable_schema" in names


class TestConnectionIsolation:
    """Two connections each carry their own writable_schema state."""

    def test_independent_state(self) -> None:
        a = mini_sqlite.connect(":memory:")
        b = mini_sqlite.connect(":memory:")
        a.execute("PRAGMA writable_schema = 1")
        # Connection ``b`` must still see the default.
        assert b.execute("PRAGMA writable_schema").fetchall() == [(0,)]
        assert a.execute("PRAGMA writable_schema").fetchall() == [(1,)]
