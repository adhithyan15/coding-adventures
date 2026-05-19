"""Oracle tests for newly added PRAGMAs (gap audit batch 3).

This file pins behaviour for six additional PRAGMAs that real-world
SQLite applications probe defensively:

- ``reverse_unordered_selects`` (bool, default 0)
- ``cell_size_check`` (bool, default 0)
- ``fullfsync`` (bool, default 0)
- ``wal_autocheckpoint`` (int, default 1000) — *echoes value on set*
- ``journal_size_limit`` (int, default -1) — *echoes value on set*
- ``threads`` (int, default 0) — *echoes value on set*

Mini-sqlite has no on-disk file, no WAL, and no thread pool, so all six
are accept-and-store cosmetic state: the value round-trips through
``PRAGMA name = X; PRAGMA name;`` but has no semantic effect on
execution.  Defaults mirror SQLite so a fresh read matches the oracle.

The trio of "echo-on-set" int PRAGMAs is a SQLite quirk — most
``PRAGMA name = X`` forms return an empty result, but these three
return a one-row scalar of the new value.  Tests pin that distinction.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both(sql: str):
    """Run the same PRAGMA on both engines (fresh connection each)."""
    m = mini_sqlite.connect(":memory:").execute(sql).fetchone()
    r = sqlite3.connect(":memory:").execute(sql).fetchone()
    return m, r


def _round_trip(sql_set: str, sql_get: str, value):
    """Set + get on a single connection (each engine independently)."""
    conn_m = mini_sqlite.connect(":memory:")
    conn_r = sqlite3.connect(":memory:")
    m_set = conn_m.execute(sql_set).fetchone()
    r_set = conn_r.execute(sql_set).fetchone()
    m_get = conn_m.execute(sql_get).fetchone()
    r_get = conn_r.execute(sql_get).fetchone()
    return (m_set, r_set), (m_get, r_get)


# ---------------------------------------------------------------------------
# Bool-valued PRAGMAs: reverse_unordered_selects, cell_size_check, fullfsync
# ---------------------------------------------------------------------------


class TestReverseUnorderedSelects:
    def test_default_is_zero(self) -> None:
        m, r = _both("PRAGMA reverse_unordered_selects")
        assert m == r == (0,)

    def test_set_to_one(self) -> None:
        (m_set, r_set), (m_get, r_get) = _round_trip(
            "PRAGMA reverse_unordered_selects = 1",
            "PRAGMA reverse_unordered_selects",
            1,
        )
        assert m_set == r_set  # both engines return None on set
        assert m_get == r_get == (1,)

    def test_set_then_unset(self) -> None:
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        conn_m.execute("PRAGMA reverse_unordered_selects = 1")
        conn_r.execute("PRAGMA reverse_unordered_selects = 1")
        conn_m.execute("PRAGMA reverse_unordered_selects = 0")
        conn_r.execute("PRAGMA reverse_unordered_selects = 0")
        m = conn_m.execute("PRAGMA reverse_unordered_selects").fetchone()
        r = conn_r.execute("PRAGMA reverse_unordered_selects").fetchone()
        assert m == r == (0,)


class TestCellSizeCheck:
    def test_default_is_zero(self) -> None:
        m, r = _both("PRAGMA cell_size_check")
        assert m == r == (0,)

    def test_set_to_one(self) -> None:
        (m_set, r_set), (m_get, r_get) = _round_trip(
            "PRAGMA cell_size_check = 1",
            "PRAGMA cell_size_check",
            1,
        )
        assert m_set == r_set
        assert m_get == r_get == (1,)


class TestFullfsync:
    def test_default_is_zero(self) -> None:
        m, r = _both("PRAGMA fullfsync")
        assert m == r == (0,)

    def test_set_to_one(self) -> None:
        (m_set, r_set), (m_get, r_get) = _round_trip(
            "PRAGMA fullfsync = 1",
            "PRAGMA fullfsync",
            1,
        )
        assert m_set == r_set
        assert m_get == r_get == (1,)


# ---------------------------------------------------------------------------
# Int-valued PRAGMAs: wal_autocheckpoint, journal_size_limit, threads
# Note: these THREE echo the new value back on set (SQLite quirk).
# ---------------------------------------------------------------------------


class TestWalAutocheckpoint:
    def test_default_is_1000(self) -> None:
        m, r = _both("PRAGMA wal_autocheckpoint")
        assert m == r == (1000,)

    def test_set_echoes_value(self) -> None:
        # Unlike most int PRAGMAs, the set form returns the new value.
        m_set, r_set = _both("PRAGMA wal_autocheckpoint = 500")
        assert m_set == r_set == (500,)

    def test_round_trip_500(self) -> None:
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        conn_m.execute("PRAGMA wal_autocheckpoint = 500")
        conn_r.execute("PRAGMA wal_autocheckpoint = 500")
        m = conn_m.execute("PRAGMA wal_autocheckpoint").fetchone()
        r = conn_r.execute("PRAGMA wal_autocheckpoint").fetchone()
        assert m == r == (500,)


class TestJournalSizeLimit:
    def test_default_is_negative_one(self) -> None:
        m, r = _both("PRAGMA journal_size_limit")
        assert m == r == (-1,)

    def test_set_echoes_value(self) -> None:
        m_set, r_set = _both("PRAGMA journal_size_limit = 1048576")
        assert m_set == r_set == (1048576,)

    def test_round_trip_1mb(self) -> None:
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        conn_m.execute("PRAGMA journal_size_limit = 1048576")
        conn_r.execute("PRAGMA journal_size_limit = 1048576")
        m = conn_m.execute("PRAGMA journal_size_limit").fetchone()
        r = conn_r.execute("PRAGMA journal_size_limit").fetchone()
        assert m == r == (1048576,)


class TestThreads:
    def test_default_is_zero(self) -> None:
        m, r = _both("PRAGMA threads")
        assert m == r == (0,)

    def test_set_echoes_value(self) -> None:
        m_set, r_set = _both("PRAGMA threads = 4")
        assert m_set == r_set == (4,)

    def test_round_trip_four(self) -> None:
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        conn_m.execute("PRAGMA threads = 4")
        conn_r.execute("PRAGMA threads = 4")
        m = conn_m.execute("PRAGMA threads").fetchone()
        r = conn_r.execute("PRAGMA threads").fetchone()
        assert m == r == (4,)


# ---------------------------------------------------------------------------
# Make sure we didn't accidentally turn ``application_id`` into echoing.
# ---------------------------------------------------------------------------


class TestSilentSetPragmasUnchanged:
    """Pre-existing silent-set int PRAGMAs must still return None on set."""

    def test_application_id_set_stays_silent(self) -> None:
        m, r = _both("PRAGMA application_id = 12345")
        # Both engines return None for the SET form of application_id.
        assert m == r is None

    def test_cache_size_set_stays_silent(self) -> None:
        m, r = _both("PRAGMA cache_size = 1000")
        assert m == r is None
