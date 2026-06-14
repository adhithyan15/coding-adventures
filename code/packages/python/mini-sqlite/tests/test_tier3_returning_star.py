"""Tests for ``RETURNING *`` shorthand in INSERT / UPDATE / DELETE.

SQLite's RETURNING grammar accepts ``*`` as a shorthand for every
column of the target table, in declaration order.  Mini-sqlite
previously parse-errored on the literal ``*`` token after RETURNING;
this PR adds:

* Grammar: ``returning_item = "*" | expr``.
* Adapter: ``*`` → :class:`Wildcard` sentinel in the returning list.
* Planner: expands the Wildcard into one ``Column(table, col_name)``
  per table column at resolution time (same approach as ``SELECT *``).

These tests pin oracle compatibility with sqlite3 for the most common
RETURNING * patterns.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both(*stmts: str, query: str) -> tuple:
    """Run *stmts* + *query* on both engines, return (mini, ref) row lists."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        for s in stmts:
            c.execute(s)
    return (
        mini.execute(query).fetchall(),
        ref.execute(query).fetchall(),
    )


class TestInsertReturningStar:
    """``INSERT ... RETURNING *`` returns every column of the inserted row."""

    def test_insert_explicit_id(self) -> None:
        # Use explicit id to sidestep mini-sqlite's pre-existing
        # IPK-auto-assign-RETURNING gap (independent of this PR).
        m, r = _both(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            query="INSERT INTO t VALUES (5, 'a') RETURNING *",
        )
        assert m == r == [(5, "a")]

    def test_three_column_table(self) -> None:
        m, r = _both(
            "CREATE TABLE t (a INTEGER PRIMARY KEY, b TEXT, c REAL)",
            query="INSERT INTO t VALUES (1, 'x', 2.5) RETURNING *",
        )
        assert m == r == [(1, "x", 2.5)]


class TestUpdateReturningStar:
    """``UPDATE ... RETURNING *`` returns one row per updated row."""

    def test_update_all_returning_star(self) -> None:
        m, r = _both(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t VALUES (1, 'a'), (2, 'b')",
            query="UPDATE t SET v = 'X' RETURNING *",
        )
        assert m == r == [(1, "X"), (2, "X")]

    def test_update_with_where(self) -> None:
        m, r = _both(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t VALUES (1, 'a'), (2, 'b')",
            query="UPDATE t SET v = 'X' WHERE id = 2 RETURNING *",
        )
        assert m == r == [(2, "X")]


class TestDeleteReturningStar:
    """``DELETE ... RETURNING *`` returns the deleted rows."""

    def test_delete_returning_star(self) -> None:
        m, r = _both(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t VALUES (1, 'a'), (2, 'b')",
            query="DELETE FROM t WHERE id = 1 RETURNING *",
        )
        assert m == r == [(1, "a")]

    def test_delete_all_returning_star(self) -> None:
        m, r = _both(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')",
            query="DELETE FROM t RETURNING *",
        )
        assert m == r
        assert len(m) == 3


class TestExplicitColumnsStillWork:
    """Existing RETURNING with explicit column lists still works."""

    def test_returning_id_v(self) -> None:
        m, r = _both(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t VALUES (1, 'a'), (2, 'b')",
            query="UPDATE t SET v = 'X' WHERE id = 1 RETURNING id, v",
        )
        assert m == r == [(1, "X")]

    def test_returning_single_column(self) -> None:
        m, r = _both(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t VALUES (1, 'a')",
            query="DELETE FROM t WHERE id = 1 RETURNING v",
        )
        assert m == r == [("a",)]


class TestColumnOrder:
    """RETURNING * follows table declaration order, not row insertion."""

    def test_column_order_matches_table_declaration(self) -> None:
        # The columns come back in declaration order regardless of
        # which order the user supplied values in.
        m, r = _both(
            "CREATE TABLE t (a INT, b INT, c INT)",
            query="INSERT INTO t (c, a, b) VALUES (3, 1, 2) RETURNING *",
        )
        # (a, b, c) order matches table declaration.
        assert m == r == [(1, 2, 3)]
