"""Tests for ``EXPLAIN QUERY PLAN <stmt>``.

SQLite's EXPLAIN QUERY PLAN walks a parsed/optimised query plan and
emits a four-column row set (``id``, ``parent``, ``notused``,
``detail``) describing the algorithmic choices it made — which table
got a sequential scan, which got an index search, whether a temp
b-tree was used for sorting / grouping, etc.

Mini-sqlite implements this by walking its own LogicalPlan tree and
mapping each "interesting" node to a SQLite-compatible detail string.
Pure transforms (Filter, Project, Limit, Having, Join) are elided so
their children re-parent to the elided node's parent — matching
SQLite's output topology.

Test strategy:

* Verify the four-column shape (id, parent, notused, detail).
* Pin the detail strings produced for the common plan shapes:
  scan, index search, sort, group by, distinct, join, subquery.
* Verify ``EXPLAIN`` without ``QUERY PLAN`` still returns an empty
  result (mini-sqlite doesn't emit VDBE bytecode).
* Verify EXPLAIN QUERY PLAN does NOT execute the inner statement
  (no side effects on the backend).
"""

from __future__ import annotations

import mini_sqlite


class TestSchema:
    """The four-column output shape matches SQLite."""

    def test_column_order(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("CREATE TABLE t (x INTEGER)")
        cur = c.execute("EXPLAIN QUERY PLAN SELECT * FROM t")
        # Description is sqlite3-style: tuple-of-tuples per column.
        # mini-sqlite mirrors that — but we mainly care about the
        # column names being present in order.
        assert cur.description is not None
        names = [d[0] for d in cur.description]
        assert names == ["id", "parent", "notused", "detail"]

    def test_notused_is_always_zero(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("CREATE TABLE t (x INTEGER)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT * FROM t WHERE x > 1 ORDER BY x"
        ).fetchall()
        assert rows
        for r in rows:
            assert r[2] == 0  # notused column


class TestSimpleScan:
    """Plain ``SELECT * FROM t`` emits one ``SCAN t`` row."""

    def test_select_star_emits_scan(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("CREATE TABLE t (x INTEGER)")
        rows = c.execute("EXPLAIN QUERY PLAN SELECT * FROM t").fetchall()
        assert rows == [(1, 0, 0, "SCAN t")]

    def test_aliased_table_includes_alias(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("CREATE TABLE t (x INTEGER)")
        rows = c.execute("EXPLAIN QUERY PLAN SELECT * FROM t AS u").fetchall()
        assert rows == [(1, 0, 0, "SCAN t AS u")]


class TestIndexSearch:
    """Predicates matched by an index produce ``SEARCH ... USING INDEX (<col>...)``."""

    def test_index_search_on_equality_predicate(self) -> None:
        # auto_index=False so the advisor doesn't add its own index;
        # we want to verify the plan we explicitly built.
        c = mini_sqlite.connect(":memory:", auto_index=False)
        c.execute("CREATE TABLE t (x INTEGER, y INTEGER)")
        c.execute("CREATE INDEX ix_x ON t (x)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT * FROM t WHERE x = 5"
        ).fetchall()
        # One row; detail names the index and the equality bound, matching
        # real SQLite's ``(x=?)`` format.
        assert rows == [(1, 0, 0, "SEARCH t USING INDEX ix_x (x=?)")]

    def test_index_search_with_gt_bound(self) -> None:
        c = mini_sqlite.connect(":memory:", auto_index=False)
        c.execute("CREATE TABLE t (x INTEGER)")
        c.execute("CREATE INDEX ix_x ON t (x)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT * FROM t WHERE x > 5"
        ).fetchall()
        assert rows == [(1, 0, 0, "SEARCH t USING INDEX ix_x (x>?)")]

    def test_index_search_with_lt_bound(self) -> None:
        c = mini_sqlite.connect(":memory:", auto_index=False)
        c.execute("CREATE TABLE t (x INTEGER)")
        c.execute("CREATE INDEX ix_x ON t (x)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT * FROM t WHERE x < 5"
        ).fetchall()
        assert rows == [(1, 0, 0, "SEARCH t USING INDEX ix_x (x<?)")]

    def test_index_search_between_bounds(self) -> None:
        # BETWEEN is inclusive on both ends in SQL, but SQLite's EXPLAIN
        # text uses ``>`` / ``<`` (no inclusivity markers).
        c = mini_sqlite.connect(":memory:", auto_index=False)
        c.execute("CREATE TABLE t (x INTEGER)")
        c.execute("CREATE INDEX ix_x ON t (x)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT * FROM t WHERE x BETWEEN 1 AND 5"
        ).fetchall()
        assert rows == [(1, 0, 0, "SEARCH t USING INDEX ix_x (x>? AND x<?)")]

    def test_composite_index_two_equality_bounds(self) -> None:
        c = mini_sqlite.connect(":memory:", auto_index=False)
        c.execute("CREATE TABLE t (x INTEGER, y INTEGER)")
        c.execute("CREATE INDEX ix_xy ON t (x, y)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT * FROM t WHERE x = 1 AND y = 2"
        ).fetchall()
        # Two-column composite equality.
        assert rows == [(1, 0, 0, "SEARCH t USING INDEX ix_xy (x=? AND y=?)")]

    def test_composite_index_mixed_eq_and_range(self) -> None:
        c = mini_sqlite.connect(":memory:", auto_index=False)
        c.execute("CREATE TABLE t (x INTEGER, y INTEGER)")
        c.execute("CREATE INDEX ix_xy ON t (x, y)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT * FROM t WHERE x = 1 AND y > 2"
        ).fetchall()
        assert rows == [(1, 0, 0, "SEARCH t USING INDEX ix_xy (x=? AND y>?)")]


class TestTempBTreeNodes:
    """Sort / group by / distinct each emit a 'USE TEMP B-TREE' row."""

    def test_order_by_emits_temp_btree(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("CREATE TABLE t (x INTEGER)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT * FROM t ORDER BY x"
        ).fetchall()
        # Sort wraps the scan: id=1 is the sort, id=2 is the child scan.
        assert rows == [
            (1, 0, 0, "USE TEMP B-TREE FOR ORDER BY"),
            (2, 1, 0, "SCAN t"),
        ]

    def test_group_by_emits_temp_btree(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("CREATE TABLE t (x INTEGER)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT x, COUNT(*) FROM t GROUP BY x"
        ).fetchall()
        # Aggregate wraps the scan.
        assert (1, 0, 0, "USE TEMP B-TREE FOR GROUP BY") in rows

    def test_distinct_emits_temp_btree(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("CREATE TABLE t (x INTEGER)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT DISTINCT x FROM t"
        ).fetchall()
        assert (1, 0, 0, "USE TEMP B-TREE FOR DISTINCT") in rows


class TestJoinShape:
    """A two-table join emits two sibling rows under the same parent."""

    def test_join_emits_two_scans(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("CREATE TABLE a (x INTEGER)")
        c.execute("CREATE TABLE b (x INTEGER)")
        rows = c.execute(
            "EXPLAIN QUERY PLAN SELECT * FROM a JOIN b ON a.x = b.x"
        ).fetchall()
        # The Join plan node is elided, so both scans are siblings
        # under the implicit root (parent=0).
        assert rows == [
            (1, 0, 0, "SCAN a"),
            (2, 0, 0, "SCAN b"),
        ]


class TestSideEffects:
    """EXPLAIN QUERY PLAN must NOT execute the inner statement."""

    def test_explain_query_plan_does_not_insert(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("CREATE TABLE t (x INTEGER)")
        # If EXPLAIN ran the INSERT we'd see a row in t afterwards.
        c.execute("EXPLAIN QUERY PLAN INSERT INTO t VALUES (1)")
        assert c.execute("SELECT COUNT(*) FROM t").fetchone() == (0,)


class TestBareExplain:
    """Bare EXPLAIN (no QUERY PLAN) returns no rows — mini-sqlite has no VDBE."""

    def test_bare_explain_returns_empty(self) -> None:
        c = mini_sqlite.connect(":memory:")
        c.execute("CREATE TABLE t (x INTEGER)")
        # We don't oracle-compare against sqlite3 here — bare EXPLAIN
        # produces VDBE bytecode in real SQLite, but mini-sqlite uses a
        # different IR.  Returning empty is the documented behaviour.
        result = c.execute("EXPLAIN SELECT * FROM t").fetchall()
        assert result == []
