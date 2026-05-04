"""
Tier-3 integration tests — SAVEPOINT / RELEASE / ROLLBACK TO
============================================================

Tests cover:
  1. Grammar  — the parser can produce the three new statement types
  2. Adapter  — the adapter correctly translates parse nodes to typed ASTs
  3. End-to-end SQL execution via mini_sqlite.connect(":memory:")
  4. Error handling — unknown savepoint names, unsupported backend

The test structure mirrors test_tier3_views.py.
"""

from __future__ import annotations

import pytest
from sql_parser import parse_sql
from sql_planner import ReleaseSavepointStmt, RollbackToStmt, SavepointStmt

import mini_sqlite
from mini_sqlite.adapter import to_statement
from mini_sqlite.errors import NotSupportedError

# ---------------------------------------------------------------------------
# Helpers.
# ---------------------------------------------------------------------------


def _parse(sql: str):
    return parse_sql(sql)


def _stmt(sql: str):
    return to_statement(_parse(sql))


# ===========================================================================
# 1. Grammar tests — parser produces the correct rule name.
# ===========================================================================


class TestSavepointGrammar:
    def test_savepoint_parses(self):
        ast = _parse("SAVEPOINT sp1")
        # The statement node's single child should be a savepoint_stmt node.
        stmt_node = ast.children[0]
        inner = stmt_node.children[0]
        assert inner.rule_name == "savepoint_stmt"

    def test_release_parses(self):
        ast = _parse("RELEASE sp1")
        inner = ast.children[0].children[0]
        assert inner.rule_name == "release_stmt"

    def test_release_savepoint_keyword_parses(self):
        ast = _parse("RELEASE SAVEPOINT sp1")
        inner = ast.children[0].children[0]
        assert inner.rule_name == "release_stmt"

    def test_rollback_to_parses(self):
        ast = _parse("ROLLBACK TO sp1")
        inner = ast.children[0].children[0]
        assert inner.rule_name == "rollback_to_stmt"

    def test_rollback_to_savepoint_keyword_parses(self):
        ast = _parse("ROLLBACK TO SAVEPOINT sp1")
        inner = ast.children[0].children[0]
        assert inner.rule_name == "rollback_to_stmt"

    def test_plain_rollback_still_parses(self):
        # "ROLLBACK" without "TO" must still be parsed as rollback_stmt.
        ast = _parse("ROLLBACK")
        inner = ast.children[0].children[0]
        assert inner.rule_name == "rollback_stmt"

    def test_rollback_transaction_still_parses(self):
        ast = _parse("ROLLBACK TRANSACTION")
        inner = ast.children[0].children[0]
        assert inner.rule_name == "rollback_stmt"

    def test_savepoint_case_insensitive(self):
        ast = _parse("savepoint mypoint")
        inner = ast.children[0].children[0]
        assert inner.rule_name == "savepoint_stmt"


# ===========================================================================
# 2. Adapter tests — parse tree → typed AST.
# ===========================================================================


class TestSavepointAdapter:
    def test_savepoint_stmt_name(self):
        stmt = _stmt("SAVEPOINT sp1")
        assert isinstance(stmt, SavepointStmt)
        assert stmt.name == "sp1"

    def test_release_stmt_name(self):
        stmt = _stmt("RELEASE sp1")
        assert isinstance(stmt, ReleaseSavepointStmt)
        assert stmt.name == "sp1"

    def test_release_with_savepoint_keyword(self):
        stmt = _stmt("RELEASE SAVEPOINT sp1")
        assert isinstance(stmt, ReleaseSavepointStmt)
        assert stmt.name == "sp1"

    def test_rollback_to_stmt_name(self):
        stmt = _stmt("ROLLBACK TO sp1")
        assert isinstance(stmt, RollbackToStmt)
        assert stmt.name == "sp1"

    def test_rollback_to_with_savepoint_keyword(self):
        stmt = _stmt("ROLLBACK TO SAVEPOINT sp1")
        assert isinstance(stmt, RollbackToStmt)
        assert stmt.name == "sp1"


# ===========================================================================
# 3. End-to-end integration — SAVEPOINT behavior via Connection.
# ===========================================================================


class TestSavepointIntegration:
    def setup_method(self):
        self.conn = mini_sqlite.connect(":memory:", auto_index=False)
        self.conn.execute("CREATE TABLE t (id INTEGER, val TEXT)")

    def teardown_method(self):
        self.conn.close()

    def test_savepoint_basic_rollback(self):
        """ROLLBACK TO restores state to the savepoint."""
        self.conn.execute("INSERT INTO t VALUES (1, 'a')")
        self.conn.execute("SAVEPOINT sp1")
        self.conn.execute("INSERT INTO t VALUES (2, 'b')")
        # Two rows visible before rollback.
        rows = self.conn.execute("SELECT id FROM t ORDER BY id").fetchall()
        assert rows == [(1,), (2,)]
        self.conn.execute("ROLLBACK TO sp1")
        # Only one row after rollback.
        rows = self.conn.execute("SELECT id FROM t ORDER BY id").fetchall()
        assert rows == [(1,)]

    def test_savepoint_release_commits_within_txn(self):
        """RELEASE drops the savepoint; outer transaction still in flight."""
        self.conn.execute("BEGIN")
        self.conn.execute("INSERT INTO t VALUES (1, 'a')")
        self.conn.execute("SAVEPOINT sp1")
        self.conn.execute("INSERT INTO t VALUES (2, 'b')")
        self.conn.execute("RELEASE sp1")
        # Both rows still visible — release doesn't undo changes.
        rows = self.conn.execute("SELECT id FROM t ORDER BY id").fetchall()
        assert rows == [(1,), (2,)]
        self.conn.execute("ROLLBACK")
        # Outer rollback removes everything.
        rows = self.conn.execute("SELECT id FROM t ORDER BY id").fetchall()
        assert rows == []

    def test_rollback_to_then_continue(self):
        """After ROLLBACK TO, subsequent inserts work and can be committed."""
        self.conn.execute("BEGIN")
        self.conn.execute("INSERT INTO t VALUES (1, 'a')")
        self.conn.execute("SAVEPOINT sp1")
        self.conn.execute("INSERT INTO t VALUES (2, 'b')")
        self.conn.execute("ROLLBACK TO sp1")
        self.conn.execute("INSERT INTO t VALUES (3, 'c')")
        self.conn.execute("COMMIT")
        rows = self.conn.execute("SELECT id FROM t ORDER BY id").fetchall()
        assert rows == [(1,), (3,)]

    def test_nested_savepoints(self):
        """Multiple nested savepoints; rollback to inner only."""
        self.conn.execute("BEGIN")
        self.conn.execute("INSERT INTO t VALUES (1, 'a')")
        self.conn.execute("SAVEPOINT outer_sp")
        self.conn.execute("INSERT INTO t VALUES (2, 'b')")
        self.conn.execute("SAVEPOINT inner_sp")
        self.conn.execute("INSERT INTO t VALUES (3, 'c')")
        self.conn.execute("ROLLBACK TO inner_sp")
        rows = self.conn.execute("SELECT id FROM t ORDER BY id").fetchall()
        assert rows == [(1,), (2,)]
        self.conn.execute("COMMIT")

    def test_rollback_to_outer_savepoint_discards_inner(self):
        """Rolling back to outer savepoint discards changes after it."""
        self.conn.execute("BEGIN")
        self.conn.execute("SAVEPOINT outer_sp")
        self.conn.execute("INSERT INTO t VALUES (1, 'a')")
        self.conn.execute("SAVEPOINT inner_sp")
        self.conn.execute("INSERT INTO t VALUES (2, 'b')")
        self.conn.execute("ROLLBACK TO outer_sp")
        rows = self.conn.execute("SELECT id FROM t ORDER BY id").fetchall()
        assert rows == []
        self.conn.execute("COMMIT")

    def test_savepoint_without_explicit_begin(self):
        """SAVEPOINT outside an explicit BEGIN still works (implicit txn)."""
        self.conn.execute("INSERT INTO t VALUES (1, 'a')")
        self.conn.execute("SAVEPOINT sp1")
        self.conn.execute("INSERT INTO t VALUES (2, 'b')")
        self.conn.execute("ROLLBACK TO sp1")
        rows = self.conn.execute("SELECT id FROM t ORDER BY id").fetchall()
        assert rows == [(1,)]
        self.conn.commit()

    def test_release_savepoint_keyword(self):
        """RELEASE SAVEPOINT sp1 is accepted."""
        self.conn.execute("BEGIN")
        self.conn.execute("INSERT INTO t VALUES (1, 'a')")
        self.conn.execute("SAVEPOINT sp1")
        self.conn.execute("INSERT INTO t VALUES (2, 'b')")
        self.conn.execute("RELEASE SAVEPOINT sp1")
        rows = self.conn.execute("SELECT id FROM t ORDER BY id").fetchall()
        assert rows == [(1,), (2,)]
        self.conn.execute("COMMIT")

    def test_rollback_to_savepoint_keyword(self):
        """ROLLBACK TO SAVEPOINT sp1 is accepted."""
        self.conn.execute("BEGIN")
        self.conn.execute("INSERT INTO t VALUES (1, 'a')")
        self.conn.execute("SAVEPOINT sp1")
        self.conn.execute("INSERT INTO t VALUES (2, 'b')")
        self.conn.execute("ROLLBACK TO SAVEPOINT sp1")
        rows = self.conn.execute("SELECT id FROM t ORDER BY id").fetchall()
        assert rows == [(1,)]
        self.conn.execute("COMMIT")

    def test_full_rollback_after_savepoint_clears_savepoints(self):
        """After a full ROLLBACK the savepoints list is cleared."""
        self.conn.execute("BEGIN")
        self.conn.execute("SAVEPOINT sp1")
        self.conn.execute("INSERT INTO t VALUES (1, 'a')")
        self.conn.execute("ROLLBACK")
        # _savepoints should be empty now.
        assert self.conn._savepoints == []  # noqa: SLF001

    def test_commit_clears_savepoints_list(self):
        """After COMMIT the savepoints list is cleared."""
        self.conn.execute("BEGIN")
        self.conn.execute("SAVEPOINT sp1")
        self.conn.execute("INSERT INTO t VALUES (1, 'a')")
        self.conn.execute("COMMIT")
        assert self.conn._savepoints == []  # noqa: SLF001

    def test_rollback_to_does_not_clear_savepoint(self):
        """ROLLBACK TO keeps the savepoint alive for re-use."""
        self.conn.execute("BEGIN")
        self.conn.execute("SAVEPOINT sp1")
        self.conn.execute("INSERT INTO t VALUES (1, 'a')")
        self.conn.execute("ROLLBACK TO sp1")
        assert "sp1" in self.conn._savepoints  # noqa: SLF001
        # Can rollback to it again.
        self.conn.execute("INSERT INTO t VALUES (2, 'b')")
        self.conn.execute("ROLLBACK TO sp1")
        rows = self.conn.execute("SELECT id FROM t ORDER BY id").fetchall()
        assert rows == []
        self.conn.execute("COMMIT")


# ===========================================================================
# 4. Error handling.
# ===========================================================================


class TestSavepointErrors:
    def setup_method(self):
        self.conn = mini_sqlite.connect(":memory:", auto_index=False)
        self.conn.execute("CREATE TABLE t (id INTEGER)")

    def teardown_method(self):
        self.conn.close()

    def test_rollback_to_unknown_savepoint_raises(self):
        self.conn.execute("BEGIN")
        with pytest.raises(NotSupportedError):
            self.conn.execute("ROLLBACK TO no_such_sp")

    def test_release_unknown_savepoint_raises(self):
        self.conn.execute("BEGIN")
        with pytest.raises(NotSupportedError):
            self.conn.execute("RELEASE no_such_sp")

    def test_plain_rollback_not_intercepted_as_savepoint(self):
        """Plain ROLLBACK is still handled by the cursor TCL fast-path."""
        self.conn.execute("BEGIN")
        self.conn.execute("INSERT INTO t VALUES (1)")
        self.conn.execute("ROLLBACK")
        rows = self.conn.execute("SELECT id FROM t").fetchall()
        assert rows == []
