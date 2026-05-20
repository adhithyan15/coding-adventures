"""Reject ``DISTINCT`` on multi-argument aggregates.

SQLite enforces a parse-time rule: a ``DISTINCT`` aggregate must take
exactly one argument.  Calling ``group_concat(DISTINCT col, sep)`` or
``json_group_object(DISTINCT key, val)`` raises::

    OperationalError: DISTINCT aggregates must have exactly one argument

This mirrors a fundamental ambiguity — for a single column the notion
of "distinct values" is unambiguous, but for multiple arguments the
engine would have to define distinctness over tuples (and SQLite does
not).

Before this PR, mini-sqlite silently accepted both forms.  The fix
adds the same arity check in the adapter's ``GROUP_CONCAT`` /
``STRING_AGG`` / ``JSON_GROUP_OBJECT`` branches and surfaces the
identical diagnostic text so callers can rely on string-match tests
against either engine.

These tests verify:
  * the rejected forms raise with the exact SQLite message
  * the legal forms (single-arg DISTINCT; multi-arg without DISTINCT)
    still work
"""

from __future__ import annotations

import pytest

import mini_sqlite
from mini_sqlite import ProgrammingError

_DISTINCT_MSG = "DISTINCT aggregates must have exactly one argument"


# ---------------------------------------------------------------------------
# Rejected forms — DISTINCT + multi-arg aggregate
# ---------------------------------------------------------------------------


class TestDistinctMultiArgRejected:
    def _setup(self) -> mini_sqlite.Connection:
        conn = mini_sqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (k TEXT, v INTEGER)")
        conn.execute("INSERT INTO t VALUES ('a', 1), ('b', 2), ('a', 1)")
        return conn

    def test_group_concat_distinct_with_separator(self) -> None:
        conn = self._setup()
        with pytest.raises(ProgrammingError, match=_DISTINCT_MSG):
            conn.execute("SELECT group_concat(DISTINCT v, ',') FROM t")

    def test_group_concat_distinct_with_pipe_separator(self) -> None:
        conn = self._setup()
        with pytest.raises(ProgrammingError, match=_DISTINCT_MSG):
            conn.execute("SELECT group_concat(DISTINCT v, '|') FROM t")

    def test_string_agg_distinct_with_separator(self) -> None:
        # STRING_AGG shares the GROUP_CONCAT code path so the rule applies
        # identically to the SQLite 3.44+ alias.
        conn = self._setup()
        with pytest.raises(ProgrammingError, match=_DISTINCT_MSG):
            conn.execute("SELECT string_agg(DISTINCT v, ',') FROM t")

    def test_json_group_object_distinct(self) -> None:
        conn = self._setup()
        with pytest.raises(ProgrammingError, match=_DISTINCT_MSG):
            conn.execute("SELECT json_group_object(DISTINCT k, v) FROM t")

    def test_error_message_exact_match(self) -> None:
        # Pin the exact diagnostic text so callers can string-match against
        # either mini-sqlite or the reference sqlite3 module.
        conn = self._setup()
        try:
            conn.execute("SELECT group_concat(DISTINCT v, ',') FROM t")
        except ProgrammingError as e:
            assert str(e) == _DISTINCT_MSG
        else:
            raise AssertionError("expected ProgrammingError")


# ---------------------------------------------------------------------------
# Legal forms — must still work
# ---------------------------------------------------------------------------


class TestLegalFormsStillWork:
    def _setup(self) -> mini_sqlite.Connection:
        conn = mini_sqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (k TEXT, v INTEGER)")
        conn.execute("INSERT INTO t VALUES ('a', 1), ('b', 2), ('a', 1)")
        return conn

    def test_group_concat_distinct_default_sep(self) -> None:
        # DISTINCT + single argument: legal.  Default ',' separator.
        conn = self._setup()
        rows = conn.execute("SELECT group_concat(DISTINCT v) FROM t").fetchall()
        assert rows == [("1,2",)]

    def test_group_concat_with_separator_no_distinct(self) -> None:
        # Multi-arg without DISTINCT: legal.
        conn = self._setup()
        rows = conn.execute("SELECT group_concat(v, '|') FROM t").fetchall()
        assert rows == [("1|2|1",)]

    def test_string_agg_no_distinct(self) -> None:
        conn = self._setup()
        rows = conn.execute("SELECT string_agg(v, ',') FROM t").fetchall()
        assert rows == [("1,2,1",)]

    def test_count_distinct_single_arg(self) -> None:
        # Confirms the new check didn't leak into the COUNT/SUM/AVG branch,
        # which already enforced its own arity rule.
        conn = self._setup()
        rows = conn.execute("SELECT COUNT(DISTINCT v) FROM t").fetchall()
        assert rows == [(2,)]

    def test_sum_distinct_single_arg(self) -> None:
        conn = self._setup()
        rows = conn.execute("SELECT SUM(DISTINCT v) FROM t").fetchall()
        assert rows == [(3,)]  # 1 + 2 (duplicate 1 collapsed)
