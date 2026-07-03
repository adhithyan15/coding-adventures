"""Oracle tests: CREATE TRIGGER without FOR EACH ROW is accepted.

SQLite makes the ``FOR EACH ROW`` clause optional.  Because SQLite has never
supported statement-level triggers, the clause is always implied and may be
omitted.  mini-sqlite previously required it and raised a parse error when
it was absent.

These tests verify:
  1. Triggers omitting ``FOR EACH ROW`` are parsed and fired correctly.
  2. Triggers including ``FOR EACH ROW`` are unaffected (regression guard).
  3. Both forms produce identical results to the real sqlite3 reference engine.

Pattern: each helper runs the same SQL against both engines and compares the
output, ensuring byte-for-byte parity with the oracle.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _setup_ref(sql_setup: list[str]) -> sqlite3.Connection:
    """Create an in-memory sqlite3 connection and execute setup statements."""
    con = sqlite3.connect(":memory:")
    for sql in sql_setup:
        con.execute(sql)
    con.commit()
    return con


def _setup_our(sql_setup: list[str]) -> mini_sqlite.Connection:
    """Create a mini_sqlite connection and execute setup statements."""
    con = mini_sqlite.connect(":memory:")
    for sql in sql_setup:
        con.execute(sql)
    con.commit()
    return con


# ---------------------------------------------------------------------------
# Tests: triggers WITHOUT FOR EACH ROW
# ---------------------------------------------------------------------------


class TestTriggerWithoutForEachRow:
    """CREATE TRIGGER … BEGIN … END (no FOR EACH ROW) parses and fires."""

    def test_before_insert_trigger_no_for_each_row(self) -> None:
        """BEFORE INSERT trigger without FOR EACH ROW fires and inserts a log row."""
        setup = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (msg TEXT)",
            "CREATE TRIGGER trg BEFORE INSERT ON t BEGIN INSERT INTO log VALUES ('fired'); END",
        ]
        ref = _setup_ref(setup)
        our = _setup_our(setup)

        ref.execute("INSERT INTO t VALUES (1)")
        our.execute("INSERT INTO t VALUES (1)")

        ref_log = ref.execute("SELECT msg FROM log").fetchall()
        our_log = our.execute("SELECT msg FROM log").fetchall()
        assert our_log == ref_log  # [('fired',)]

    def test_after_insert_trigger_no_for_each_row(self) -> None:
        """AFTER INSERT trigger without FOR EACH ROW fires correctly."""
        setup = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (msg TEXT)",
            "CREATE TRIGGER trg AFTER INSERT ON t BEGIN INSERT INTO log VALUES ('after'); END",
        ]
        ref = _setup_ref(setup)
        our = _setup_our(setup)

        ref.execute("INSERT INTO t VALUES (42)")
        our.execute("INSERT INTO t VALUES (42)")

        ref_log = ref.execute("SELECT msg FROM log").fetchall()
        our_log = our.execute("SELECT msg FROM log").fetchall()
        assert our_log == ref_log

    def test_before_delete_trigger_no_for_each_row(self) -> None:
        """BEFORE DELETE trigger without FOR EACH ROW fires once per deleted row."""
        setup = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (msg TEXT)",
            "CREATE TRIGGER trg BEFORE DELETE ON t BEGIN INSERT INTO log VALUES ('deleted'); END",
            "INSERT INTO t VALUES (1)",
            "INSERT INTO t VALUES (2)",
        ]
        ref = _setup_ref(setup)
        our = _setup_our(setup)

        ref.execute("DELETE FROM t WHERE x = 1")
        our.execute("DELETE FROM t WHERE x = 1")

        ref_log = ref.execute("SELECT msg FROM log").fetchall()
        our_log = our.execute("SELECT msg FROM log").fetchall()
        assert our_log == ref_log

    def test_after_update_trigger_no_for_each_row(self) -> None:
        """AFTER UPDATE trigger without FOR EACH ROW fires on UPDATE."""
        setup = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (msg TEXT)",
            "CREATE TRIGGER trg AFTER UPDATE ON t BEGIN INSERT INTO log VALUES ('updated'); END",
            "INSERT INTO t VALUES (10)",
        ]
        ref = _setup_ref(setup)
        our = _setup_our(setup)

        ref.execute("UPDATE t SET x = 20")
        our.execute("UPDATE t SET x = 20")

        ref_log = ref.execute("SELECT msg FROM log").fetchall()
        our_log = our.execute("SELECT msg FROM log").fetchall()
        assert our_log == ref_log

    def test_trigger_no_for_each_row_fires_per_row(self) -> None:
        """Trigger fires once per affected row even without FOR EACH ROW."""
        setup = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (msg TEXT)",
            "CREATE TRIGGER trg AFTER INSERT ON t BEGIN INSERT INTO log VALUES ('row'); END",
        ]
        ref = _setup_ref(setup)
        our = _setup_our(setup)

        for con in (ref, our):
            con.execute("INSERT INTO t VALUES (1)")
            con.execute("INSERT INTO t VALUES (2)")
            con.execute("INSERT INTO t VALUES (3)")

        ref_count = ref.execute("SELECT COUNT(*) FROM log").fetchone()[0]
        our_count = our.execute("SELECT COUNT(*) FROM log").fetchone()[0]
        assert our_count == ref_count == 3

    def test_trigger_no_for_each_row_drop_works(self) -> None:
        """A trigger created without FOR EACH ROW can be dropped normally."""
        setup = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (msg TEXT)",
            "CREATE TRIGGER trg AFTER INSERT ON t BEGIN INSERT INTO log VALUES ('x'); END",
        ]
        ref = _setup_ref(setup)
        our = _setup_our(setup)

        for con in (ref, our):
            con.execute("DROP TRIGGER trg")
            con.execute("INSERT INTO t VALUES (1)")  # trigger gone, no log row

        ref_log = ref.execute("SELECT * FROM log").fetchall()
        our_log = our.execute("SELECT * FROM log").fetchall()
        assert our_log == ref_log == []


# ---------------------------------------------------------------------------
# Tests: triggers WITH FOR EACH ROW (regression guard)
# ---------------------------------------------------------------------------


class TestTriggerWithForEachRow:
    """CREATE TRIGGER … FOR EACH ROW BEGIN … END still works after the change."""

    def test_before_insert_trigger_with_for_each_row(self) -> None:
        """BEFORE INSERT trigger with FOR EACH ROW fires correctly."""
        setup = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (msg TEXT)",
            "CREATE TRIGGER trg BEFORE INSERT ON t FOR EACH ROW"
            " BEGIN INSERT INTO log VALUES ('fired'); END",
        ]
        ref = _setup_ref(setup)
        our = _setup_our(setup)

        ref.execute("INSERT INTO t VALUES (1)")
        our.execute("INSERT INTO t VALUES (1)")

        ref_log = ref.execute("SELECT msg FROM log").fetchall()
        our_log = our.execute("SELECT msg FROM log").fetchall()
        assert our_log == ref_log

    def test_after_delete_trigger_with_for_each_row(self) -> None:
        """AFTER DELETE trigger with FOR EACH ROW fires correctly."""
        setup = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (msg TEXT)",
            "CREATE TRIGGER trg AFTER DELETE ON t FOR EACH ROW"
            " BEGIN INSERT INTO log VALUES ('del'); END",
            "INSERT INTO t VALUES (99)",
        ]
        ref = _setup_ref(setup)
        our = _setup_our(setup)

        ref.execute("DELETE FROM t")
        our.execute("DELETE FROM t")

        ref_log = ref.execute("SELECT * FROM log").fetchall()
        our_log = our.execute("SELECT * FROM log").fetchall()
        assert our_log == ref_log

    def test_both_forms_produce_identical_results(self) -> None:
        """With and without FOR EACH ROW produce identical log rows."""
        setup_without = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (msg TEXT)",
            "CREATE TRIGGER trg AFTER INSERT ON t BEGIN INSERT INTO log VALUES ('row'); END",
        ]
        setup_with = [
            "CREATE TABLE t (x INT)",
            "CREATE TABLE log (msg TEXT)",
            "CREATE TRIGGER trg AFTER INSERT ON t FOR EACH ROW"
            " BEGIN INSERT INTO log VALUES ('row'); END",
        ]
        con_without = _setup_our(setup_without)
        con_with = _setup_our(setup_with)

        for con in (con_without, con_with):
            con.execute("INSERT INTO t VALUES (1)")
            con.execute("INSERT INTO t VALUES (2)")

        rows_without = con_without.execute("SELECT * FROM log").fetchall()
        rows_with = con_with.execute("SELECT * FROM log").fetchall()
        assert rows_without == rows_with  # identical firing behaviour
