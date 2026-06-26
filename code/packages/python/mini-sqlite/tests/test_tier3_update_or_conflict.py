"""Oracle tests for ``UPDATE OR <conflict>`` conflict resolution.

SQLite extends the SQL ``UPDATE`` statement with the same five conflict
resolution strategies it supports for ``INSERT``:

    UPDATE OR REPLACE t SET col = val WHERE ...
    UPDATE OR IGNORE  t SET col = val WHERE ...
    UPDATE OR ABORT   t SET col = val WHERE ...   -- default
    UPDATE OR FAIL    t SET col = val WHERE ...
    UPDATE OR ROLLBACK t SET col = val WHERE ...

The most commonly used modes are:

IGNORE
    If the update of a row would violate a UNIQUE, PRIMARY KEY, or NOT NULL
    constraint, that particular row is silently left unchanged.  Other rows
    not affected by the constraint violation are updated normally.  The count
    of ``rows_affected`` reflects only the rows that were actually changed.

REPLACE
    Before applying the update, any *other* rows whose UNIQUE or PRIMARY KEY
    column values would conflict with the new values are deleted first.  The
    update then proceeds unconditionally.  This is the same pre-deletion
    strategy SQLite uses for ``INSERT OR REPLACE``.

ABORT (default)
    Raise a constraint error and abort the current statement.

FAIL / ROLLBACK
    Like ABORT for our purposes (mini-sqlite has no multi-statement
    transaction semantics beyond what the single connection commit cycle
    provides).

All tests compare mini-sqlite's output byte-for-byte against the stdlib
``sqlite3`` module (the oracle), ensuring semantic fidelity.
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _setup(ddl_and_dml: str) -> tuple[mini_sqlite.Connection, sqlite3.Connection]:
    """Run the same DDL/DML on both engines and return both connections."""
    mc = mini_sqlite.connect(":memory:")
    rc = sqlite3.connect(":memory:")
    for sql in ddl_and_dml.strip().split(";"):
        sql = sql.strip()
        if sql:
            mc.execute(sql)
            rc.execute(sql)
    return mc, rc


def _check_query(mc: mini_sqlite.Connection, rc: sqlite3.Connection, query: str) -> None:
    m = mc.execute(query).fetchall()
    r = rc.execute(query).fetchall()
    assert m == r, f"Query: {query!r}\n  mini: {m}\n  ref:  {r}"


def _exec_and_fetch(
    setup_sql: str,
    update_sql: str,
    fetch_sql: str,
) -> tuple[list, list]:
    """Run update on both engines, return (mini_rows, ref_rows)."""
    mc, rc = _setup(setup_sql)
    mc.execute(update_sql)
    rc.execute(update_sql)
    m = mc.execute(fetch_sql).fetchall()
    r = rc.execute(fetch_sql).fetchall()
    return m, r


# ---------------------------------------------------------------------------
# Baseline: plain UPDATE without conflict clause (should still work)
# ---------------------------------------------------------------------------

class TestPlainUpdateUnchanged:
    """UPDATE without OR modifier continues to work normally."""

    def test_plain_update_all_rows(self) -> None:
        m, r = _exec_and_fetch(
            "CREATE TABLE t (x INT); INSERT INTO t VALUES (1); INSERT INTO t VALUES (2)",
            "UPDATE t SET x = x + 10",
            "SELECT x FROM t ORDER BY x",
        )
        assert m == r

    def test_plain_update_with_where(self) -> None:
        setup = (
            "CREATE TABLE t (x INT, y TEXT);"
            " INSERT INTO t VALUES (1,'a');"
            " INSERT INTO t VALUES (2,'b')"
        )
        m, r = _exec_and_fetch(
            setup,
            "UPDATE t SET y = 'z' WHERE x = 1",
            "SELECT x, y FROM t ORDER BY x",
        )
        assert m == r

    def test_plain_update_unique_violation_raises(self) -> None:
        mc, rc = _setup(
            "CREATE TABLE t (id INT UNIQUE); INSERT INTO t VALUES (1); INSERT INTO t VALUES (2)"
        )
        with pytest.raises(mini_sqlite.IntegrityError):
            mc.execute("UPDATE t SET id = 1 WHERE id = 2")


# ---------------------------------------------------------------------------
# UPDATE OR IGNORE
# ---------------------------------------------------------------------------

class TestUpdateOrIgnore:
    """UPDATE OR IGNORE skips rows that would violate constraints."""

    def test_ignore_unique_violation_skips_row(self) -> None:
        """Updating a row to a value already held by another row is skipped."""
        m, r = _exec_and_fetch(
            "CREATE TABLE t (id INT UNIQUE, v TEXT); "
            "INSERT INTO t VALUES (1,'a'); "
            "INSERT INTO t VALUES (2,'b')",
            "UPDATE OR IGNORE t SET id = 1 WHERE id = 2",
            "SELECT id, v FROM t ORDER BY id",
        )
        assert m == r

    def test_ignore_leaves_original_intact(self) -> None:
        """The row whose update was skipped retains its original values."""
        mc, _ = _setup(
            "CREATE TABLE t (id INT UNIQUE, v TEXT); "
            "INSERT INTO t VALUES (1,'a'); "
            "INSERT INTO t VALUES (2,'b')"
        )
        mc.execute("UPDATE OR IGNORE t SET id = 1 WHERE id = 2")
        rows = mc.execute("SELECT id, v FROM t ORDER BY id").fetchall()
        assert rows == [(1, "a"), (2, "b")]

    def test_ignore_partial_update_some_rows_succeed(self) -> None:
        """When multiple rows are matched, only the violating ones are skipped."""
        m, r = _exec_and_fetch(
            "CREATE TABLE t (id INT UNIQUE, v TEXT); "
            "INSERT INTO t VALUES (1,'a'); "
            "INSERT INTO t VALUES (2,'b'); "
            "INSERT INTO t VALUES (3,'c')",
            # row id=2 conflicts with existing id=1; row id=3 → id=99 is fine
            "UPDATE OR IGNORE t SET id = CASE WHEN id=2 THEN 1 ELSE id+96 END WHERE id >= 2",
            "SELECT id, v FROM t ORDER BY id",
        )
        assert m == r

    def test_ignore_not_null_violation(self) -> None:
        """NOT NULL violations are also silently skipped under IGNORE."""
        m, r = _exec_and_fetch(
            "CREATE TABLE t (id INT, v TEXT NOT NULL); "
            "INSERT INTO t VALUES (1,'a'); "
            "INSERT INTO t VALUES (2,'b')",
            "UPDATE OR IGNORE t SET v = NULL WHERE id = 1",
            "SELECT id, v FROM t ORDER BY id",
        )
        assert m == r

    def test_ignore_primary_key_violation(self) -> None:
        """INTEGER PRIMARY KEY (rowid alias) violations are skipped under IGNORE."""
        m, r = _exec_and_fetch(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT); "
            "INSERT INTO t VALUES (1,'a'); "
            "INSERT INTO t VALUES (2,'b')",
            "UPDATE OR IGNORE t SET id = 1 WHERE id = 2",
            "SELECT id, v FROM t ORDER BY id",
        )
        assert m == r

    def test_ignore_no_conflict_updates_normally(self) -> None:
        """When there is no conflict, OR IGNORE behaves identically to plain UPDATE."""
        m, r = _exec_and_fetch(
            "CREATE TABLE t (id INT UNIQUE, v TEXT); "
            "INSERT INTO t VALUES (1,'hello')",
            "UPDATE OR IGNORE t SET v = 'world' WHERE id = 1",
            "SELECT id, v FROM t",
        )
        assert m == r

    def test_ignore_rows_affected_excludes_skipped(self) -> None:
        """rows_affected should count only rows that were actually changed."""
        mc, rc = _setup(
            "CREATE TABLE t (id INT UNIQUE); INSERT INTO t VALUES (1); INSERT INTO t VALUES (2)"
        )
        mc_cur = mc.execute("UPDATE OR IGNORE t SET id = 1 WHERE id = 2")
        rc_cur = rc.execute("UPDATE OR IGNORE t SET id = 1 WHERE id = 2")
        assert mc_cur.rowcount == rc_cur.rowcount


# ---------------------------------------------------------------------------
# UPDATE OR REPLACE
# ---------------------------------------------------------------------------

class TestUpdateOrReplace:
    """UPDATE OR REPLACE deletes conflicting rows before applying the update."""

    def test_replace_deletes_conflicting_row(self) -> None:
        """The row that would conflict with the update is deleted first."""
        m, r = _exec_and_fetch(
            "CREATE TABLE t (id INT UNIQUE, v TEXT); "
            "INSERT INTO t VALUES (1,'a'); "
            "INSERT INTO t VALUES (2,'b')",
            # After OR REPLACE, row 1 is deleted, row 2 gets id=1.
            "UPDATE OR REPLACE t SET id = 1 WHERE id = 2",
            "SELECT id, v FROM t ORDER BY id",
        )
        assert m == r

    def test_replace_no_conflict_is_plain_update(self) -> None:
        """When no other row conflicts, OR REPLACE behaves like a plain UPDATE."""
        m, r = _exec_and_fetch(
            "CREATE TABLE t (id INT UNIQUE, v TEXT); "
            "INSERT INTO t VALUES (1,'a'); "
            "INSERT INTO t VALUES (2,'b')",
            "UPDATE OR REPLACE t SET v = 'z' WHERE id = 1",
            "SELECT id, v FROM t ORDER BY id",
        )
        assert m == r

    def test_replace_integer_primary_key(self) -> None:
        """OR REPLACE on an INTEGER PRIMARY KEY works the same as on a UNIQUE column."""
        m, r = _exec_and_fetch(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT); "
            "INSERT INTO t VALUES (1,'a'); "
            "INSERT INTO t VALUES (2,'b')",
            "UPDATE OR REPLACE t SET id = 1 WHERE id = 2",
            "SELECT id, v FROM t ORDER BY id",
        )
        assert m == r

    def test_replace_row_count_after_deletion(self) -> None:
        """Table has exactly one row after REPLACE removes the conflicting one."""
        mc, _ = _setup(
            "CREATE TABLE t (id INT UNIQUE, v TEXT); "
            "INSERT INTO t VALUES (1,'a'); "
            "INSERT INTO t VALUES (2,'b')"
        )
        mc.execute("UPDATE OR REPLACE t SET id = 1 WHERE id = 2")
        count = mc.execute("SELECT COUNT(*) FROM t").fetchone()[0]
        assert count == 1

    def test_replace_result_row_has_updated_values(self) -> None:
        """After REPLACE, the surviving row has the values from the UPDATE."""
        mc, _ = _setup(
            "CREATE TABLE t (id INT UNIQUE, v TEXT); "
            "INSERT INTO t VALUES (1,'original'); "
            "INSERT INTO t VALUES (2,'updated')"
        )
        mc.execute("UPDATE OR REPLACE t SET id = 1 WHERE id = 2")
        row = mc.execute("SELECT id, v FROM t").fetchone()
        assert row == (1, "updated")


# ---------------------------------------------------------------------------
# UPDATE OR ABORT (explicit) — same as default
# ---------------------------------------------------------------------------

class TestUpdateOrAbort:
    """UPDATE OR ABORT raises on constraint violation (same as no modifier)."""

    def test_abort_raises_on_unique_violation(self) -> None:
        mc, _ = _setup(
            "CREATE TABLE t (id INT UNIQUE); INSERT INTO t VALUES (1); INSERT INTO t VALUES (2)"
        )
        with pytest.raises(mini_sqlite.IntegrityError):
            mc.execute("UPDATE OR ABORT t SET id = 1 WHERE id = 2")

    def test_abort_succeeds_when_no_violation(self) -> None:
        m, r = _exec_and_fetch(
            "CREATE TABLE t (id INT UNIQUE, v TEXT); INSERT INTO t VALUES (1,'a')",
            "UPDATE OR ABORT t SET v = 'b' WHERE id = 1",
            "SELECT id, v FROM t",
        )
        assert m == r


# ---------------------------------------------------------------------------
# UPDATE OR FAIL
# ---------------------------------------------------------------------------

class TestUpdateOrFail:
    """UPDATE OR FAIL raises on constraint violation."""

    def test_fail_raises_on_unique_violation(self) -> None:
        mc, _ = _setup(
            "CREATE TABLE t (id INT UNIQUE); INSERT INTO t VALUES (1); INSERT INTO t VALUES (2)"
        )
        with pytest.raises(mini_sqlite.IntegrityError):
            mc.execute("UPDATE OR FAIL t SET id = 1 WHERE id = 2")

    def test_fail_succeeds_when_no_violation(self) -> None:
        m, r = _exec_and_fetch(
            "CREATE TABLE t (id INT, v TEXT); INSERT INTO t VALUES (1,'a')",
            "UPDATE OR FAIL t SET v = 'b' WHERE id = 1",
            "SELECT id, v FROM t",
        )
        assert m == r


# ---------------------------------------------------------------------------
# UPDATE OR ROLLBACK
# ---------------------------------------------------------------------------

class TestUpdateOrRollback:
    """UPDATE OR ROLLBACK raises on constraint violation."""

    def test_rollback_raises_on_unique_violation(self) -> None:
        mc, _ = _setup(
            "CREATE TABLE t (id INT UNIQUE); INSERT INTO t VALUES (1); INSERT INTO t VALUES (2)"
        )
        with pytest.raises(mini_sqlite.IntegrityError):
            mc.execute("UPDATE OR ROLLBACK t SET id = 1 WHERE id = 2")

    def test_rollback_succeeds_when_no_violation(self) -> None:
        m, r = _exec_and_fetch(
            "CREATE TABLE t (id INT, v TEXT); INSERT INTO t VALUES (1,'a')",
            "UPDATE OR ROLLBACK t SET v = 'b' WHERE id = 1",
            "SELECT id, v FROM t",
        )
        assert m == r
