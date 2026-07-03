"""Tests for SQLite's ``INDEXED BY`` / ``NOT INDEXED`` table hints.

SQLite supports two query hints on a base-table reference:

* ``... FROM t INDEXED BY ix WHERE ...`` — force the planner to use
  the named index ``ix``.  Errors at plan time if ``ix`` doesn't
  exist on ``t``.
* ``... FROM t NOT INDEXED WHERE ...`` — instruct the planner to
  ignore any matching indexes and use a full table scan.

These tests verify mini-sqlite's parser accepts the syntax and the
planner honours both hints, and that an unknown index name surfaces a
clear error.
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite
from mini_sqlite import errors as mini_errors


class TestSyntaxAccepted:
    """Parser accepts the new INDEXED BY / NOT INDEXED forms."""

    def test_indexed_by_returns_correct_rows(self) -> None:
        # When the index covers the predicate, the result must be the
        # same as without the hint (correctness wins over performance).
        c = mini_sqlite.connect(":memory:", auto_index=False)
        c.execute("CREATE TABLE t (x INTEGER, y INTEGER)")
        c.execute("CREATE INDEX ix_x ON t(x)")
        c.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
        rows = c.execute(
            "SELECT y FROM t INDEXED BY ix_x WHERE x = 2"
        ).fetchall()
        assert rows == [(20,)]

    def test_not_indexed_returns_correct_rows(self) -> None:
        c = mini_sqlite.connect(":memory:", auto_index=False)
        c.execute("CREATE TABLE t (x INTEGER, y INTEGER)")
        c.execute("CREATE INDEX ix_x ON t(x)")
        c.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
        # The result must match even with the index disabled — the
        # full-scan path produces the same rows.
        rows = c.execute(
            "SELECT y FROM t NOT INDEXED WHERE x = 2"
        ).fetchall()
        assert rows == [(20,)]

    def test_oracle_indexed_by_matches_sqlite3(self) -> None:
        # End-to-end byte-compat sanity check against the stdlib driver.
        for hint in ("", " INDEXED BY ix_x", " NOT INDEXED"):
            mini = mini_sqlite.connect(":memory:", auto_index=False)
            ref = sqlite3.connect(":memory:")
            for c in (mini, ref):
                c.execute("CREATE TABLE t (x INTEGER, y INTEGER)")
                c.execute("CREATE INDEX ix_x ON t(x)")
                c.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30)")
            q = f"SELECT y FROM t{hint} WHERE x = 2"
            assert mini.execute(q).fetchall() == ref.execute(q).fetchall(), hint


class TestPlannerHonoursHints:
    """EXPLAIN QUERY PLAN confirms the planner picked the requested strategy."""

    def test_indexed_by_forces_search(self) -> None:
        c = mini_sqlite.connect(":memory:", auto_index=False)
        c.execute("CREATE TABLE t (x INTEGER, y INTEGER)")
        c.execute("CREATE INDEX ix_x ON t(x)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT * FROM t INDEXED BY ix_x WHERE x = 2"
        ).fetchall()
        # Single SEARCH row using the named index.
        # mini-sqlite 1.92+: detail includes the matched bound, mirroring
        # SQLite's ``(x=?)`` format for equality predicates.
        assert rows == [(1, 0, 0, "SEARCH t USING INDEX ix_x (x=?)")]

    def test_not_indexed_falls_back_to_scan(self) -> None:
        c = mini_sqlite.connect(":memory:", auto_index=False)
        c.execute("CREATE TABLE t (x INTEGER, y INTEGER)")
        c.execute("CREATE INDEX ix_x ON t(x)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT * FROM t NOT INDEXED WHERE x = 2"
        ).fetchall()
        # Without the hint the planner would substitute SEARCH ... USING
        # INDEX; NOT INDEXED forces a SCAN.
        assert rows == [(1, 0, 0, "SCAN t")]

    def test_no_hint_uses_planner_default(self) -> None:
        # Sanity check: without a hint, the planner picks the index
        # automatically when one matches the predicate.
        c = mini_sqlite.connect(":memory:", auto_index=False)
        c.execute("CREATE TABLE t (x INTEGER, y INTEGER)")
        c.execute("CREATE INDEX ix_x ON t(x)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT * FROM t WHERE x = 2"
        ).fetchall()
        # mini-sqlite 1.92+: detail includes the matched bound, mirroring
        # SQLite's ``(x=?)`` format for equality predicates.
        assert rows == [(1, 0, 0, "SEARCH t USING INDEX ix_x (x=?)")]


class TestErrorOnMissingIndex:
    """``INDEXED BY`` with an unknown index name raises a planner error."""

    def test_unknown_index_raises_operational_error(self) -> None:
        c = mini_sqlite.connect(":memory:", auto_index=False)
        c.execute("CREATE TABLE t (x INTEGER)")
        c.execute("INSERT INTO t VALUES (1)")
        # We don't define any index — the hint must error.
        with pytest.raises(mini_errors.OperationalError) as exc:
            c.execute(
                "SELECT * FROM t INDEXED BY no_such_index WHERE x = 1"
            ).fetchall()
        assert "no_such_index" in str(exc.value)

    def test_real_sqlite_also_rejects_unknown_index(self) -> None:
        # Sanity-check: real SQLite errors on unknown INDEXED BY too.
        ref = sqlite3.connect(":memory:")
        ref.execute("CREATE TABLE t (x INTEGER)")
        with pytest.raises(sqlite3.Error):
            ref.execute(
                "SELECT * FROM t INDEXED BY no_such_index WHERE x = 1"
            ).fetchall()


class TestHintInJoin:
    """The hint applies to the table it's attached to, not the whole query."""

    def test_indexed_by_on_right_side_of_join(self) -> None:
        c = mini_sqlite.connect(":memory:", auto_index=False)
        c.execute("CREATE TABLE a (x INTEGER)")
        c.execute("CREATE TABLE b (y INTEGER)")
        c.execute("CREATE INDEX ix_y ON b(y)")
        c.execute("INSERT INTO a VALUES (1), (2)")
        c.execute("INSERT INTO b VALUES (1), (2), (3)")
        # The query still produces correct rows.
        rows = c.execute(
            "SELECT a.x, b.y FROM a JOIN b INDEXED BY ix_y "
            "ON a.x = b.y WHERE a.x = 1"
        ).fetchall()
        assert rows == [(1, 1)]
