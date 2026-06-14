"""Tests for ``PRAGMA foreign_key_check``.

SQLite's ``PRAGMA foreign_key_check`` walks every (or one named) child
table and reports one row per FK violation::

    table   TEXT    — child table holding the bad row
    rowid   INTEGER — the bad row's rowid
    parent  TEXT    — referenced parent table
    fkid    INTEGER — 0-based FK position (matches ``foreign_key_list.id``)

The pragma is read-only and produces no error even when used on tables
with no FK declarations.  ORMs and migration tools call it to validate
schema integrity after disabling FK enforcement (``PRAGMA foreign_keys
= OFF``) for bulk operations.

Mini-sqlite enforces FKs unconditionally on INSERT (a known deviation
from SQLite, which defaults to OFF), so producing violations through
normal SQL is impossible.  These tests use the backend's row-list
directly to inject violations, then verify the pragma surfaces them
correctly.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite

_ROWID_KEY = "\x00rowid"


def _inject_orphan(conn, table: str, row: dict) -> None:
    """Bypass FK enforcement and append a row directly to the backend.

    Used to construct violations the engine wouldn't otherwise let
    happen — necessary because mini-sqlite enforces FKs at INSERT
    time and won't accept an orphan child via normal SQL.
    """
    stamped = dict(row)
    stamped.setdefault(_ROWID_KEY, row.get("id", 1))
    conn._backend._tables[table].rows.append(stamped)  # noqa: SLF001


class TestEmptyCases:
    """Clean schemas return zero rows."""

    def test_no_tables(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        assert mini.execute("PRAGMA foreign_key_check").fetchall() == []

    def test_no_fks(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE t (x INT)")
        mini.execute("INSERT INTO t VALUES (1), (2)")
        assert mini.execute("PRAGMA foreign_key_check").fetchall() == []

    def test_fks_satisfied(self) -> None:
        # Oracle: real sqlite3 returns [] too when every child has a parent.
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute("CREATE TABLE p(id INTEGER PRIMARY KEY)")
            c.execute(
                "CREATE TABLE c(id INTEGER PRIMARY KEY, "
                "p_id INTEGER REFERENCES p(id))"
            )
            c.execute("INSERT INTO p VALUES (1), (2)")
            c.execute("INSERT INTO c VALUES (1, 1), (2, 2)")
        q = "PRAGMA foreign_key_check"
        assert mini.execute(q).fetchall() == ref.execute(q).fetchall() == []


class TestViolations:
    """Injected orphans surface as violation rows."""

    def test_single_violation(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE p(id INTEGER PRIMARY KEY)")
        mini.execute(
            "CREATE TABLE c(id INTEGER PRIMARY KEY, "
            "p_id INTEGER REFERENCES p(id))"
        )
        mini.execute("INSERT INTO p VALUES (1)")
        _inject_orphan(mini, "c", {"id": 99, "p_id": 999})
        assert mini.execute("PRAGMA foreign_key_check").fetchall() \
            == [("c", 99, "p", 0)]

    def test_oracle_match(self) -> None:
        # Same scenario in real sqlite3 (FK off by default → orphan
        # accepted) — verify the violation row matches mini-sqlite's.
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute("CREATE TABLE p(id INTEGER PRIMARY KEY)")
            c.execute(
                "CREATE TABLE c(id INTEGER PRIMARY KEY, "
                "p_id INTEGER REFERENCES p(id))"
            )
            c.execute("INSERT INTO p VALUES (1)")
        _inject_orphan(mini, "c", {"id": 99, "p_id": 999})
        ref.execute("INSERT INTO c VALUES (99, 999)")
        q = "PRAGMA foreign_key_check"
        assert mini.execute(q).fetchall() == ref.execute(q).fetchall()

    def test_multiple_violations(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE p(id INTEGER PRIMARY KEY)")
        mini.execute(
            "CREATE TABLE c(id INTEGER PRIMARY KEY, "
            "p_id INTEGER REFERENCES p(id))"
        )
        _inject_orphan(mini, "c", {"id": 1, "p_id": 100})
        _inject_orphan(mini, "c", {"id": 2, "p_id": 200})
        _inject_orphan(mini, "c", {"id": 3, "p_id": 300})
        rows = mini.execute("PRAGMA foreign_key_check").fetchall()
        # Three violations, all in 'c' referencing 'p'.
        assert len(rows) == 3
        assert all(r[0] == "c" and r[2] == "p" and r[3] == 0 for r in rows)


class TestNullValuePassesUnconditionally:
    """NULL child FK values are not violations (SQL standard)."""

    def test_null_value_skipped(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE p(id INTEGER PRIMARY KEY)")
        mini.execute(
            "CREATE TABLE c(id INTEGER PRIMARY KEY, "
            "p_id INTEGER REFERENCES p(id))"
        )
        _inject_orphan(mini, "c", {"id": 7, "p_id": None})
        assert mini.execute("PRAGMA foreign_key_check").fetchall() == []


class TestTableFilter:
    """``PRAGMA foreign_key_check(<table>)`` restricts the scan."""

    def test_filter_to_violating_table(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE p(id INTEGER PRIMARY KEY)")
        mini.execute(
            "CREATE TABLE c(id INTEGER PRIMARY KEY, "
            "p_id INTEGER REFERENCES p(id))"
        )
        _inject_orphan(mini, "c", {"id": 5, "p_id": 50})
        assert mini.execute("PRAGMA foreign_key_check(c)").fetchall() \
            == [("c", 5, "p", 0)]

    def test_filter_to_unrelated_table_returns_empty(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        mini.execute("CREATE TABLE p(id INTEGER PRIMARY KEY)")
        mini.execute(
            "CREATE TABLE c(id INTEGER PRIMARY KEY, "
            "p_id INTEGER REFERENCES p(id))"
        )
        _inject_orphan(mini, "c", {"id": 5, "p_id": 50})
        # p has no FK declarations, so filtering to p sees no violations.
        assert mini.execute("PRAGMA foreign_key_check(p)").fetchall() == []


class TestListed:
    """``foreign_key_check`` appears in PRAGMA pragma_list."""

    def test_in_pragma_list(self) -> None:
        mini = mini_sqlite.connect(":memory:")
        names = {r[0] for r in mini.execute("PRAGMA pragma_list").fetchall()}
        assert "foreign_key_check" in names
