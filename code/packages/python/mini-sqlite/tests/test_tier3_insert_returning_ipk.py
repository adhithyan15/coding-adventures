"""Tests for ``INSERT ... RETURNING`` reflecting auto-assigned IPK.

Mini-sqlite 2.0 added ``RETURNING *`` shorthand but inherited a
pre-existing limitation: ``INSERT(v) VALUES (...) RETURNING *`` on a
table with an auto-assigned INTEGER PRIMARY KEY column reported the
``id`` as NULL.  The VM's ``LoadLastInsertedColumn`` path read from
``st.last_inserted_row``, which was the dict the user passed in —
missing the IPK because the auto-assign happened inside the backend.

Mini-sqlite 2.1 (this PR) fixes the contract: the backend's
``insert()`` mutates the caller's input dict to reflect auto-
assigned and default values, so ``last_inserted_row`` carries the
post-auto-assign state.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both_match(*stmts: str, query: str) -> None:
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        for s in stmts:
            c.execute(s)
    assert mini.execute(query).fetchall() == ref.execute(query).fetchall()


class TestAutoAssignedIPK:
    """RETURNING * surfaces the auto-assigned id."""

    def test_returning_star_after_partial_insert(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            query="INSERT INTO t(v) VALUES ('a') RETURNING *",
        )

    def test_returning_id_alone(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            query="INSERT INTO t(v) VALUES ('a') RETURNING id",
        )

    def test_returning_explicit_id_v(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            query="INSERT INTO t(v) VALUES ('a') RETURNING id, v",
        )

    def test_returning_sequential_inserts(self) -> None:
        # The auto-assigned id increments for each successive INSERT.
        mini = mini_sqlite.connect(":memory:")
        ref = sqlite3.connect(":memory:")
        for c in (mini, ref):
            c.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        for v in ("a", "b", "c"):
            q = f"INSERT INTO t(v) VALUES ('{v}') RETURNING id"
            assert mini.execute(q).fetchall() == ref.execute(q).fetchall()


class TestExplicitIPKStillWorks:
    """When the user supplies the id explicitly, RETURNING returns that value."""

    def test_explicit_id_returned_verbatim(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            query="INSERT INTO t VALUES (42, 'a') RETURNING *",
        )


class TestNonIPKDefault:
    """Default values on non-IPK columns also propagate into RETURNING *."""

    def test_text_default_reflected(self) -> None:
        # When the user omits a column with a DEFAULT, RETURNING *
        # surfaces the default value (matches SQLite).
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT DEFAULT 'hi')",
            query="INSERT INTO t(id) VALUES (1) RETURNING *",
        )


class TestLastInsertRowidStillCorrect:
    """The fix doesn't regress ``last_insert_rowid()``."""

    def test_last_insert_rowid_after_partial_insert(self) -> None:
        _both_match(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t(v) VALUES ('a')",
            query="SELECT last_insert_rowid()",
        )
