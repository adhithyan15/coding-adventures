"""Oracle tests: INTERSECT ALL and EXCEPT ALL raise OperationalError.

SQLite does not support bag semantics for INTERSECT or EXCEPT.  Both
``SELECT … INTERSECT ALL SELECT …`` and ``SELECT … EXCEPT ALL SELECT …``
produce ``OperationalError: near "ALL": syntax error`` in the real engine.
Mini-sqlite must match this behaviour byte-for-byte.

The grammar parses ``[ "ALL" ]`` for all three set operators so the adapter
can surface a clean, SQLite-compatible error rather than a confusing
token-mismatch from the PEG engine.  The adapter's ``_set_op_clause``
function detects the illegal combination and raises ``OperationalError``.
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite


def _ref_raises(sql: str) -> str:
    """Assert sqlite3 raises OperationalError and return the message."""
    with pytest.raises(sqlite3.OperationalError) as exc_info:
        sqlite3.connect(":memory:").execute(sql)
    return str(exc_info.value)


class TestIntersectAllRejected:
    """INTERSECT ALL is rejected with the same error as real SQLite."""

    def test_intersect_all_raises_operational_error(self) -> None:
        sql = "SELECT 1 INTERSECT ALL SELECT 1"
        ref_msg = _ref_raises(sql)
        with pytest.raises(mini_sqlite.OperationalError) as exc_info:
            mini_sqlite.connect(":memory:").execute(sql)
        assert str(exc_info.value) == ref_msg

    def test_intersect_all_message_contains_near_all(self) -> None:
        sql = "SELECT 1 INTERSECT ALL SELECT 1"
        with pytest.raises(mini_sqlite.OperationalError) as exc_info:
            mini_sqlite.connect(":memory:").execute(sql)
        assert 'near "ALL": syntax error' in str(exc_info.value)

    def test_intersect_all_with_strings(self) -> None:
        sql = "SELECT 'a' INTERSECT ALL SELECT 'a'"
        with pytest.raises(mini_sqlite.OperationalError):
            mini_sqlite.connect(":memory:").execute(sql)

    def test_intersect_all_with_table(self) -> None:
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t(x INT)")
        con.execute("INSERT INTO t VALUES (1)")
        with pytest.raises(mini_sqlite.OperationalError) as exc_info:
            con.execute("SELECT x FROM t INTERSECT ALL SELECT x FROM t")
        assert 'near "ALL": syntax error' in str(exc_info.value)

    def test_intersect_without_all_still_works(self) -> None:
        """Plain INTERSECT (no ALL) must continue to work."""
        ref = sqlite3.connect(":memory:").execute("SELECT 1 INTERSECT SELECT 1").fetchall()
        our = mini_sqlite.connect(":memory:").execute("SELECT 1 INTERSECT SELECT 1").fetchall()
        assert our == ref

    def test_intersect_deduplicates(self) -> None:
        """INTERSECT without ALL gives set (deduplicated) semantics."""
        sql = "SELECT 1 UNION ALL SELECT 1 UNION ALL SELECT 2 INTERSECT SELECT 1"
        ref = sqlite3.connect(":memory:").execute(sql).fetchall()
        our = mini_sqlite.connect(":memory:").execute(sql).fetchall()
        assert our == ref


class TestExceptAllRejected:
    """EXCEPT ALL is rejected with the same error as real SQLite."""

    def test_except_all_raises_operational_error(self) -> None:
        sql = "SELECT 1 EXCEPT ALL SELECT 2"
        ref_msg = _ref_raises(sql)
        with pytest.raises(mini_sqlite.OperationalError) as exc_info:
            mini_sqlite.connect(":memory:").execute(sql)
        assert str(exc_info.value) == ref_msg

    def test_except_all_message_contains_near_all(self) -> None:
        sql = "SELECT 1 EXCEPT ALL SELECT 2"
        with pytest.raises(mini_sqlite.OperationalError) as exc_info:
            mini_sqlite.connect(":memory:").execute(sql)
        assert 'near "ALL": syntax error' in str(exc_info.value)

    def test_except_all_with_strings(self) -> None:
        sql = "SELECT 'x' EXCEPT ALL SELECT 'y'"
        with pytest.raises(mini_sqlite.OperationalError):
            mini_sqlite.connect(":memory:").execute(sql)

    def test_except_all_with_table(self) -> None:
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t(x INT)")
        con.execute("INSERT INTO t VALUES (1), (2)")
        with pytest.raises(mini_sqlite.OperationalError) as exc_info:
            con.execute("SELECT x FROM t EXCEPT ALL SELECT 1")
        assert 'near "ALL": syntax error' in str(exc_info.value)

    def test_except_without_all_still_works(self) -> None:
        """Plain EXCEPT (no ALL) must continue to work."""
        ref = sqlite3.connect(":memory:").execute("SELECT 1 EXCEPT SELECT 2").fetchall()
        our = mini_sqlite.connect(":memory:").execute("SELECT 1 EXCEPT SELECT 2").fetchall()
        assert our == ref

    def test_except_removes_intersection(self) -> None:
        """EXCEPT without ALL gives set-difference semantics."""
        sql = "SELECT 1 UNION ALL SELECT 2 EXCEPT SELECT 2"
        ref = sqlite3.connect(":memory:").execute(sql).fetchall()
        our = mini_sqlite.connect(":memory:").execute(sql).fetchall()
        assert our == ref


class TestUnionAllStillWorks:
    """UNION ALL must be unaffected by the INTERSECT/EXCEPT guard."""

    def test_union_all_basic(self) -> None:
        ref = sqlite3.connect(":memory:").execute("SELECT 1 UNION ALL SELECT 1").fetchall()
        our = mini_sqlite.connect(":memory:").execute("SELECT 1 UNION ALL SELECT 1").fetchall()
        assert our == ref  # [(1,), (1,)] — duplicates preserved

    def test_union_all_with_strings(self) -> None:
        ref = sqlite3.connect(":memory:").execute("SELECT 'a' UNION ALL SELECT 'a'").fetchall()
        our = mini_sqlite.connect(":memory:").execute("SELECT 'a' UNION ALL SELECT 'a'").fetchall()
        assert our == ref

    def test_union_all_three_way(self) -> None:
        sql = "SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3"
        ref = sqlite3.connect(":memory:").execute(sql).fetchall()
        our = mini_sqlite.connect(":memory:").execute(sql).fetchall()
        assert our == ref

    def test_union_without_all_deduplicates(self) -> None:
        sql = "SELECT 1 UNION SELECT 1"
        ref = sqlite3.connect(":memory:").execute(sql).fetchall()
        our = mini_sqlite.connect(":memory:").execute(sql).fetchall()
        assert our == ref  # [(1,)] — deduplicated
