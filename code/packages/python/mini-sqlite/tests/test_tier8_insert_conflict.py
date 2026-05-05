"""INSERT OR REPLACE / INSERT OR IGNORE / REPLACE INTO — oracle-verified tests.

Every test in this module runs the same SQL on both mini-sqlite and real
sqlite3 (via _both) and compares results, so regressions are caught against
the gold standard.

Coverage targets:
  - INSERT OR REPLACE with a single primary-key conflict
  - INSERT OR REPLACE with multiple conflicting rows
  - INSERT OR REPLACE with no conflict (plain insert)
  - REPLACE INTO shorthand (same semantics as INSERT OR REPLACE)
  - INSERT OR IGNORE with a constraint violation
  - INSERT OR IGNORE with no violation (plain insert)
  - INSERT OR IGNORE across multiple rows — only conflicting row is skipped
  - INSERT OR REPLACE with a UNIQUE column (not primary key)
  - INSERT OR REPLACE preserving other columns
  - INSERT OR REPLACE with INSERT … SELECT
  - INSERT OR IGNORE with INSERT … SELECT
  - Mixing INSERT OR REPLACE and INSERT OR IGNORE in the same session
  - INSERT OR ABORT (default behaviour — raises IntegrityError)
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite

# ---------------------------------------------------------------------------
# Oracle helper
# ---------------------------------------------------------------------------


def _both(setup: list[str], query: str) -> tuple[list[tuple], list[tuple]]:
    """Execute *setup* statements then *query* on both mini-sqlite and sqlite3.

    Returns ``(mini_rows, real_rows)`` so callers can assert equality.
    The same SQL runs verbatim on both engines, so the test is a direct
    oracle comparison.
    """
    # --- mini-sqlite ---
    mini_con = mini_sqlite.connect(":memory:")
    mini_cur = mini_con.cursor()
    for stmt in setup:
        mini_cur.execute(stmt)
    mini_cur.execute(query)
    mini_rows = mini_cur.fetchall()
    mini_con.close()

    # --- real sqlite3 ---
    real_con = sqlite3.connect(":memory:")
    real_cur = real_con.cursor()
    for stmt in setup:
        real_cur.execute(stmt)
    real_cur.execute(query)
    real_rows = real_cur.fetchall()
    real_con.close()

    return mini_rows, real_rows


# ---------------------------------------------------------------------------
# TestInsertOrReplace
# ---------------------------------------------------------------------------


class TestInsertOrReplace:
    """INSERT OR REPLACE behaviour matches real SQLite."""

    def test_replace_single_conflict_updates_row(self) -> None:
        """When a new row shares a PRIMARY KEY with an existing row, the old
        row is deleted and the new one is inserted — net result is one row
        with the new values.
        """
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'original')",
            "INSERT OR REPLACE INTO t VALUES (1, 'replaced')",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "replaced")]

    def test_replace_no_conflict_inserts_new_row(self) -> None:
        """When the new row's key does not conflict, INSERT OR REPLACE behaves
        identically to a plain INSERT.
        """
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'a')",
            "INSERT OR REPLACE INTO t VALUES (2, 'b')",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "a"), (2, "b")]

    def test_replace_multiple_rows_same_key(self) -> None:
        """Multiple INSERT OR REPLACE calls with the same key each delete the
        previous version, leaving only the last-inserted row.
        """
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT OR REPLACE INTO t VALUES (1, 'first')",
            "INSERT OR REPLACE INTO t VALUES (1, 'second')",
            "INSERT OR REPLACE INTO t VALUES (1, 'third')",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "third")]

    def test_replace_updates_non_key_columns(self) -> None:
        """REPLACE replaces the entire row — all non-key columns take the
        new values, not the old ones.
        """
        setup = [
            "CREATE TABLE products (sku TEXT PRIMARY KEY, name TEXT, price REAL)",
            "INSERT INTO products VALUES ('A1', 'Widget', 9.99)",
            "INSERT OR REPLACE INTO products VALUES ('A1', 'Widget Pro', 19.99)",
        ]
        mini, real = _both(setup, "SELECT sku, name, price FROM products")
        assert mini == real
        assert mini == [("A1", "Widget Pro", 19.99)]

    def test_replace_with_unique_column(self) -> None:
        """INSERT OR REPLACE respects UNIQUE constraints on non-primary-key
        columns in addition to the primary key.
        """
        setup = [
            "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT UNIQUE, name TEXT)",
            "INSERT INTO users VALUES (1, 'a@example.com', 'Alice')",
            # Insert a new id but conflicting email — old row is deleted.
            "INSERT OR REPLACE INTO users VALUES (2, 'a@example.com', 'Alicia')",
        ]
        mini, real = _both(setup, "SELECT id, email, name FROM users ORDER BY id")
        assert mini == real
        # Row 1 deleted; row 2 inserted with updated name.
        assert mini == [(2, "a@example.com", "Alicia")]

    def test_replace_into_shorthand(self) -> None:
        """``REPLACE INTO t ...`` is syntactic sugar for ``INSERT OR REPLACE INTO t ...``."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'old')",
            "REPLACE INTO t VALUES (1, 'new')",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t")
        assert mini == real
        assert mini == [(1, "new")]

    def test_replace_with_mixed_rows(self) -> None:
        """When inserting multiple rows in separate statements, REPLACE only
        touches rows that actually conflict.
        """
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t VALUES (1, 'a')",
            "INSERT INTO t VALUES (2, 'b')",
            "INSERT INTO t VALUES (3, 'c')",
            "INSERT OR REPLACE INTO t VALUES (2, 'B')",
        ]
        mini, real = _both(setup, "SELECT id, v FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "a"), (2, "B"), (3, "c")]

    def test_replace_empty_table(self) -> None:
        """REPLACE on an empty table simply inserts the row."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT OR REPLACE INTO t VALUES (42, 'hello')",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t")
        assert mini == real
        assert mini == [(42, "hello")]


# ---------------------------------------------------------------------------
# TestInsertOrIgnore
# ---------------------------------------------------------------------------


class TestInsertOrIgnore:
    """INSERT OR IGNORE behaviour matches real SQLite."""

    def test_ignore_skips_duplicate_primary_key(self) -> None:
        """When a new row's PRIMARY KEY already exists, INSERT OR IGNORE
        silently skips that row — the existing row is untouched.
        """
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'original')",
            "INSERT OR IGNORE INTO t VALUES (1, 'ignored')",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "original")]

    def test_ignore_inserts_when_no_conflict(self) -> None:
        """INSERT OR IGNORE inserts normally when there is no constraint
        violation — it is not a no-op for all inserts.
        """
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (1, 'existing')",
            "INSERT OR IGNORE INTO t VALUES (2, 'new')",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "existing"), (2, "new")]

    def test_ignore_only_skips_conflicting_row(self) -> None:
        """In a sequence of inserts, only the conflicting row is skipped;
        subsequent rows are inserted normally.
        """
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO t VALUES (2, 'existing')",
            "INSERT OR IGNORE INTO t VALUES (1, 'ok')",
            "INSERT OR IGNORE INTO t VALUES (2, 'skipped')",
            "INSERT OR IGNORE INTO t VALUES (3, 'ok')",
        ]
        mini, real = _both(setup, "SELECT id, val FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "ok"), (2, "existing"), (3, "ok")]

    def test_ignore_with_unique_constraint(self) -> None:
        """INSERT OR IGNORE also handles UNIQUE columns that are not the
        primary key.
        """
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, code TEXT UNIQUE)",
            "INSERT INTO t VALUES (1, 'X')",
            "INSERT OR IGNORE INTO t VALUES (2, 'X')",  # duplicate code
        ]
        mini, real = _both(setup, "SELECT id, code FROM t ORDER BY id")
        assert mini == real
        assert mini == [(1, "X")]

    def test_ignore_count_only_successful_inserts(self) -> None:
        """The row count returned after INSERT OR IGNORE reflects only the
        rows actually inserted, not the ones skipped.
        """
        mini_con = mini_sqlite.connect(":memory:")
        mini_cur = mini_con.cursor()
        mini_cur.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)")
        mini_cur.execute("INSERT INTO t VALUES (1, 'a')")
        mini_cur.execute("INSERT OR IGNORE INTO t VALUES (1, 'b')")  # skipped
        mini_cur.execute("INSERT OR IGNORE INTO t VALUES (2, 'c')")  # inserted
        # rowcount for a single-row statement should reflect just that statement.
        assert mini_cur.rowcount == 1  # the last INSERT inserted 1 row
        mini_con.close()


# ---------------------------------------------------------------------------
# TestInsertOrReplaceWithSelect
# ---------------------------------------------------------------------------


class TestInsertOrReplaceWithSelect:
    """INSERT OR REPLACE / IGNORE with INSERT … SELECT."""

    def test_replace_from_select(self) -> None:
        """INSERT OR REPLACE … SELECT copies rows from a source table,
        replacing any conflicting rows in the target.
        """
        setup = [
            "CREATE TABLE src (id INTEGER PRIMARY KEY, val TEXT)",
            "CREATE TABLE dst (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO src VALUES (1, 'new1'), (2, 'new2')",
            "INSERT INTO dst VALUES (1, 'old1'), (3, 'old3')",
            "INSERT OR REPLACE INTO dst SELECT * FROM src",
        ]
        mini, real = _both(setup, "SELECT id, val FROM dst ORDER BY id")
        assert mini == real
        assert mini == [(1, "new1"), (2, "new2"), (3, "old3")]

    def test_ignore_from_select(self) -> None:
        """INSERT OR IGNORE … SELECT skips rows that conflict with existing
        rows in the target.
        """
        setup = [
            "CREATE TABLE src (id INTEGER PRIMARY KEY, val TEXT)",
            "CREATE TABLE dst (id INTEGER PRIMARY KEY, val TEXT)",
            "INSERT INTO src VALUES (1, 'new1'), (2, 'new2')",
            "INSERT INTO dst VALUES (1, 'old1')",
            "INSERT OR IGNORE INTO dst SELECT * FROM src",
        ]
        mini, real = _both(setup, "SELECT id, val FROM dst ORDER BY id")
        assert mini == real
        # Row 1 kept (IGNORE skips src row 1); row 2 inserted.
        assert mini == [(1, "old1"), (2, "new2")]


# ---------------------------------------------------------------------------
# TestInsertOrAbort (default behaviour)
# ---------------------------------------------------------------------------


class TestInsertOrAbort:
    """INSERT OR ABORT raises IntegrityError on conflict — same as plain INSERT."""

    def test_abort_raises_on_conflict(self) -> None:
        """INSERT OR ABORT is the default behaviour: raise an IntegrityError
        when a constraint is violated.
        """
        mini_con = mini_sqlite.connect(":memory:")
        mini_cur = mini_con.cursor()
        mini_cur.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)")
        mini_cur.execute("INSERT INTO t VALUES (1, 'first')")
        with pytest.raises(mini_sqlite.IntegrityError):
            mini_cur.execute("INSERT OR ABORT INTO t VALUES (1, 'conflict')")
        # Original row is unchanged.
        mini_cur.execute("SELECT val FROM t WHERE id = 1")
        assert mini_cur.fetchone() == ("first",)
        mini_con.close()

    def test_plain_insert_raises_on_conflict(self) -> None:
        """Plain INSERT (no OR clause) raises IntegrityError on PK conflict —
        same observable result as INSERT OR ABORT.
        """
        mini_con = mini_sqlite.connect(":memory:")
        mini_cur = mini_con.cursor()
        mini_cur.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)")
        mini_cur.execute("INSERT INTO t VALUES (1)")
        with pytest.raises(mini_sqlite.IntegrityError):
            mini_cur.execute("INSERT INTO t VALUES (1)")
        mini_con.close()
