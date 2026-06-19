"""Oracle tests: CREATE TRIGGER IF NOT EXISTS.

SQLite allows the optional ``IF NOT EXISTS`` guard on ``CREATE TRIGGER``::

    CREATE TRIGGER IF NOT EXISTS name BEFORE/AFTER event ON table ...

When ``IF NOT EXISTS`` is present and a trigger with that name already
exists, the statement is silently ignored — exactly like the same guard
on ``CREATE TABLE``, ``CREATE INDEX``, and ``CREATE VIEW``.

Without the guard, creating a duplicate trigger raises an error:
``OperationalError: trigger already exists: 'name'``

These tests verify:

1. ``CREATE TRIGGER IF NOT EXISTS`` parses and executes without error
   for a fresh trigger (base case).
2. A duplicate ``CREATE TRIGGER IF NOT EXISTS`` is silently ignored —
   the existing trigger continues to fire correctly.
3. A duplicate ``CREATE TRIGGER`` (no guard) raises ``OperationalError``.
4. The guard works for all three trigger events: INSERT, UPDATE, DELETE.
5. The guard works with both BEFORE and AFTER timing.
6. The trigger body executes normally after an idempotent re-creation.

Mini-sqlite deviates from real SQLite in one expected way: when the
duplicate is silently ignored, we compare against sqlite3's behaviour
on a single creation (not a duplicate), because sqlite3 also raises an
error on duplicate triggers — there is no cross-engine oracle for the
idempotent case.  The single-creation oracle tests confirm the trigger
fires exactly as expected.
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite


def _our_exec(*stmts: str) -> mini_sqlite.Connection:
    """Run *stmts* against a fresh in-memory mini-sqlite db and return it."""
    con = mini_sqlite.connect(":memory:")
    for s in stmts:
        con.execute(s)
    return con


def _ref_exec(*stmts: str) -> sqlite3.Connection:
    """Run *stmts* against a fresh in-memory sqlite3 db and return it."""
    con = sqlite3.connect(":memory:")
    for s in stmts:
        con.execute(s)
    return con


class TestCreateTriggerIfNotExistsBasic:
    """CREATE TRIGGER IF NOT EXISTS — parse and single-creation semantics."""

    def test_after_insert_if_not_exists_parses(self) -> None:
        """IF NOT EXISTS on an AFTER INSERT trigger must parse without error."""
        _our_exec(
            "CREATE TABLE t (x INT)",
            "CREATE TRIGGER IF NOT EXISTS trg AFTER INSERT ON t FOR EACH ROW BEGIN SELECT 1; END",
        )

    def test_before_insert_if_not_exists_parses(self) -> None:
        _our_exec(
            "CREATE TABLE t (x INT)",
            "CREATE TRIGGER IF NOT EXISTS trg BEFORE INSERT ON t FOR EACH ROW BEGIN SELECT 1; END",
        )

    def test_after_update_if_not_exists_parses(self) -> None:
        _our_exec(
            "CREATE TABLE t (x INT)",
            "CREATE TRIGGER IF NOT EXISTS trg AFTER UPDATE ON t FOR EACH ROW BEGIN SELECT 1; END",
        )

    def test_before_update_if_not_exists_parses(self) -> None:
        _our_exec(
            "CREATE TABLE t (x INT)",
            "CREATE TRIGGER IF NOT EXISTS trg BEFORE UPDATE ON t FOR EACH ROW BEGIN SELECT 1; END",
        )

    def test_after_delete_if_not_exists_parses(self) -> None:
        _our_exec(
            "CREATE TABLE t (x INT)",
            "CREATE TRIGGER IF NOT EXISTS trg AFTER DELETE ON t FOR EACH ROW BEGIN SELECT 1; END",
        )

    def test_before_delete_if_not_exists_parses(self) -> None:
        _our_exec(
            "CREATE TABLE t (x INT)",
            "CREATE TRIGGER IF NOT EXISTS trg BEFORE DELETE ON t FOR EACH ROW BEGIN SELECT 1; END",
        )

    def test_trigger_fires_after_creation_with_guard(self) -> None:
        """Trigger created with IF NOT EXISTS must still fire normally."""
        con = _our_exec(
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (n INT)",
            "CREATE TRIGGER IF NOT EXISTS trg AFTER INSERT ON t "
            "FOR EACH ROW BEGIN INSERT INTO log VALUES (1); END",
            "INSERT INTO t VALUES (42)",
            "INSERT INTO t VALUES (43)",
        )
        rows = con.execute("SELECT COUNT(*) FROM log").fetchall()
        assert rows == [(2,)]


class TestIdempotentDuplicate:
    """IF NOT EXISTS suppresses the 'trigger already exists' error."""

    def test_duplicate_if_not_exists_is_silent(self) -> None:
        """Second CREATE TRIGGER IF NOT EXISTS with the same name is a no-op."""
        con = _our_exec(
            "CREATE TABLE t (x INT)",
            "CREATE TRIGGER IF NOT EXISTS trg AFTER INSERT ON t FOR EACH ROW BEGIN SELECT 1; END",
        )
        # This must not raise:
        con.execute(
            "CREATE TRIGGER IF NOT EXISTS trg AFTER INSERT ON t FOR EACH ROW BEGIN SELECT 2; END"
        )

    def test_original_trigger_survives_duplicate_if_not_exists(self) -> None:
        """The ORIGINAL trigger body must remain active after a silently ignored duplicate."""
        con = _our_exec(
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (n INT)",
            # Original trigger inserts 99 into log.
            "CREATE TRIGGER IF NOT EXISTS trg AFTER INSERT ON t "
            "FOR EACH ROW BEGIN INSERT INTO log VALUES (99); END",
        )
        # Duplicate with different body — must be silently ignored.
        con.execute(
            "CREATE TRIGGER IF NOT EXISTS trg AFTER INSERT ON t "
            "FOR EACH ROW BEGIN INSERT INTO log VALUES (0); END"
        )
        # Fire the trigger once.
        con.execute("INSERT INTO t VALUES (1)")
        rows = con.execute("SELECT n FROM log").fetchall()
        # Only the original body (99) should appear — not 0.
        assert rows == [(99,)]

    def test_triple_create_if_not_exists_is_idempotent(self) -> None:
        """Three consecutive CREATE TRIGGER IF NOT EXISTS for the same name is fine."""
        con = _our_exec(
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (n INT)",
            "CREATE TRIGGER IF NOT EXISTS trg AFTER INSERT ON t "
            "FOR EACH ROW BEGIN INSERT INTO log VALUES (7); END",
        )
        con.execute(
            "CREATE TRIGGER IF NOT EXISTS trg AFTER INSERT ON t "
            "FOR EACH ROW BEGIN INSERT INTO log VALUES (8); END"
        )
        con.execute(
            "CREATE TRIGGER IF NOT EXISTS trg AFTER INSERT ON t "
            "FOR EACH ROW BEGIN INSERT INTO log VALUES (9); END"
        )
        con.execute("INSERT INTO t VALUES (1)")
        rows = con.execute("SELECT n FROM log").fetchall()
        assert rows == [(7,)]


class TestDuplicateWithoutGuardRaises:
    """Without IF NOT EXISTS, duplicate trigger must raise."""

    def test_duplicate_without_guard_raises(self) -> None:
        con = _our_exec(
            "CREATE TABLE t (x INT)",
            "CREATE TRIGGER trg AFTER INSERT ON t FOR EACH ROW BEGIN SELECT 1; END",
        )
        with pytest.raises(mini_sqlite.InternalError):
            con.execute(
                "CREATE TRIGGER trg AFTER INSERT ON t FOR EACH ROW BEGIN SELECT 2; END"
            )

    def test_no_guard_first_creation_is_always_fine(self) -> None:
        """First CREATE TRIGGER without IF NOT EXISTS must succeed."""
        _our_exec(
            "CREATE TABLE t (x INT)",
            "CREATE TRIGGER trg AFTER INSERT ON t FOR EACH ROW BEGIN SELECT 1; END",
        )


class TestOracleSingleCreation:
    """Oracle: single creation with IF NOT EXISTS matches sqlite3 behaviour."""

    def _setup(self) -> tuple[str, list[str]]:
        query = "SELECT n FROM log ORDER BY rowid"
        setup = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (n INT)",
            "CREATE TRIGGER IF NOT EXISTS trg AFTER INSERT ON t "
            "FOR EACH ROW BEGIN INSERT INTO log VALUES (1); END",
            "INSERT INTO t VALUES (10)",
            "INSERT INTO t VALUES (20)",
        ]
        return query, setup

    def _ref(self, sql: str, setup: list[str]) -> list[tuple]:
        con = sqlite3.connect(":memory:")
        for s in setup:
            con.execute(s)
        return con.execute(sql).fetchall()

    def _our(self, sql: str, setup: list[str]) -> list[tuple]:
        con = mini_sqlite.connect(":memory:")
        for s in setup:
            con.execute(s)
        return con.execute(sql).fetchall()

    def test_after_insert_trigger_oracle(self) -> None:
        sql, setup = self._setup()
        assert self._our(sql, setup) == self._ref(sql, setup)

    def test_before_insert_trigger_oracle(self) -> None:
        query = "SELECT n FROM log ORDER BY rowid"
        setup = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (n INT)",
            "CREATE TRIGGER IF NOT EXISTS trg BEFORE INSERT ON t "
            "FOR EACH ROW BEGIN INSERT INTO log VALUES (2); END",
            "INSERT INTO t VALUES (10)",
        ]
        assert self._our(query, setup) == self._ref(query, setup)

    def test_after_delete_trigger_oracle(self) -> None:
        query = "SELECT n FROM log ORDER BY rowid"
        setup = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (n INT)",
            "INSERT INTO t VALUES (1)",
            "INSERT INTO t VALUES (2)",
            "CREATE TRIGGER IF NOT EXISTS trg AFTER DELETE ON t "
            "FOR EACH ROW BEGIN INSERT INTO log VALUES (3); END",
            "DELETE FROM t WHERE x = 1",
        ]
        assert self._our(query, setup) == self._ref(query, setup)

    def test_after_update_trigger_oracle(self) -> None:
        query = "SELECT n FROM log ORDER BY rowid"
        setup = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (n INT)",
            "INSERT INTO t VALUES (1)",
            "CREATE TRIGGER IF NOT EXISTS trg AFTER UPDATE ON t "
            "FOR EACH ROW BEGIN INSERT INTO log VALUES (4); END",
            "UPDATE t SET x = 99",
        ]
        assert self._our(query, setup) == self._ref(query, setup)
