"""Oracle tests for two window function correctness fixes.

Fix 1 — hidden-column injection for PlanWindowAgg
--------------------------------------------------
When ORDER BY references a column absent from output_cols (because
ComputeWindowFunctions has projected it away), SortResult previously
raised ValueError.  The codegen now extends output_cols with the missing
columns as hidden trailing entries and strips them after sorting, exactly
mirroring the existing hidden-column injection for plain Project nodes.

Fix 2 — RANGE mode peer-group expansion
----------------------------------------
SQLite's default frame when ORDER BY is present is
``RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW``.  In RANGE mode
``CURRENT ROW`` means the entire *peer group* — all rows with the same
ORDER BY key values as the current row.  The VM previously applied ROWS
semantics (physical row position), returning wrong cumulative totals
whenever tied ORDER BY values appear.

Every test here uses the oracle pattern: run the same SQL against both
``mini_sqlite`` and the stdlib ``sqlite3`` module and assert identical
results.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _oracle(sql: str, setup: list[str] | None = None) -> tuple[list, list]:
    """Run *sql* in both engines; return (mini_sqlite_rows, sqlite3_rows)."""
    ref = sqlite3.connect(":memory:")
    ours = mini_sqlite.connect(":memory:")
    for s in setup or []:
        ref.execute(s)
        ours.execute(s)
    ref_rows = ref.execute(sql).fetchall()
    our_rows = ours.execute(sql).fetchall()
    return our_rows, ref_rows


SETUP_PART = [
    "CREATE TABLE s (grp TEXT, val INTEGER)",
    "INSERT INTO s VALUES ('a',10),('a',20),('b',5),('b',30)",
]

SETUP_TIED = [
    "CREATE TABLE t (a INTEGER)",
    "INSERT INTO t VALUES (1),(2),(2),(3),(3),(3)",
]


# ===========================================================================
# Fix 1: hidden-column injection for PlanWindowAgg
# ---------------------------------------------------------------------------
# These queries crash before the fix because ORDER BY references a column
# (``val``, ``grp``) that ComputeWindowFunctions has projected away.
# ===========================================================================


class TestPartitionOrderCrash:
    """SUM/COUNT over PARTITION BY + external ORDER BY column."""

    def test_sum_partition_order_by_val(self) -> None:
        """ORDER BY val is outside output_cols when PARTITION BY removes it."""
        sql = (
            "SELECT grp, SUM(val) OVER (PARTITION BY grp) "
            "FROM s ORDER BY grp, val"
        )
        ours, ref = _oracle(sql, SETUP_PART)
        assert ours == ref
        # Rows are ordered by (grp, val): a/10 a/20 b/5 b/30
        assert ours == [("a", 30), ("a", 30), ("b", 35), ("b", 35)]

    def test_count_star_partition_order_by_val(self) -> None:
        """COUNT(*) OVER (PARTITION BY grp) ORDER BY grp, val."""
        sql = (
            "SELECT grp, COUNT(*) OVER (PARTITION BY grp) "
            "FROM s ORDER BY grp, val"
        )
        ours, ref = _oracle(sql, SETUP_PART)
        assert ours == ref

    def test_avg_partition_order_by_val(self) -> None:
        """AVG OVER (PARTITION BY grp) ORDER BY val descending."""
        sql = (
            "SELECT grp, AVG(val) OVER (PARTITION BY grp) "
            "FROM s ORDER BY val DESC"
        )
        ours, ref = _oracle(sql, SETUP_PART)
        assert ours == ref

    def test_min_max_partition_order_by_two_cols(self) -> None:
        """Two window functions, ORDER BY references both grp and val."""
        sql = (
            "SELECT grp, MIN(val) OVER (PARTITION BY grp), "
            "MAX(val) OVER (PARTITION BY grp) "
            "FROM s ORDER BY grp, val"
        )
        ours, ref = _oracle(sql, SETUP_PART)
        assert ours == ref

    def test_row_number_partition_order_by_external(self) -> None:
        """ROW_NUMBER() OVER (PARTITION BY grp ORDER BY val) + outer ORDER BY."""
        sql = (
            "SELECT grp, ROW_NUMBER() OVER (PARTITION BY grp ORDER BY val) "
            "FROM s ORDER BY grp, val"
        )
        ours, ref = _oracle(sql, SETUP_PART)
        assert ours == ref

    def test_no_crash_when_order_col_already_in_output(self) -> None:
        """Sanity-check: grp is already in output_cols, no injection needed."""
        sql = (
            "SELECT grp, SUM(val) OVER (PARTITION BY grp) "
            "FROM s ORDER BY grp"
        )
        ours, ref = _oracle(sql, SETUP_PART)
        assert ours == ref


# ===========================================================================
# Fix 2: RANGE mode peer-group expansion
# ---------------------------------------------------------------------------
# These queries return wrong results before the fix because ties in the
# ORDER BY column cause CURRENT ROW to be interpreted as the physical row
# position instead of the end of the peer group.
# ===========================================================================


class TestRangePeerGroup:
    """Default RANGE frame with tied ORDER BY values."""

    def test_count_star_tied_values(self) -> None:
        """COUNT(*) OVER (ORDER BY a) with 2+3 ties.

        Truth table for data (1,2,2,3,3,3):
          a=1: frame=[1]       → count=1
          a=2: frame=[1,2,2]   → count=3  (peer group includes both a=2 rows)
          a=2: frame=[1,2,2]   → count=3
          a=3: frame=[1..6]    → count=6  (peer group includes all a=3 rows)
          a=3: frame=[1..6]    → count=6
          a=3: frame=[1..6]    → count=6
        """
        sql = "SELECT a, COUNT(*) OVER (ORDER BY a) FROM t ORDER BY rowid"
        ours, ref = _oracle(sql, SETUP_TIED)
        assert ours == ref
        assert ours == [(1, 1), (2, 3), (2, 3), (3, 6), (3, 6), (3, 6)]

    def test_sum_tied_values(self) -> None:
        """SUM(a) OVER (ORDER BY a) with tied values."""
        sql = "SELECT a, SUM(a) OVER (ORDER BY a) FROM t ORDER BY rowid"
        ours, ref = _oracle(sql, SETUP_TIED)
        assert ours == ref
        # a=1 cumsum=1; a=2 peer group (values 1,2,2) cumsum=5; a=3 cumsum=14
        assert ours == [(1, 1), (2, 5), (2, 5), (3, 14), (3, 14), (3, 14)]

    def test_avg_tied_values(self) -> None:
        """AVG(a) OVER (ORDER BY a) with tied values."""
        sql = "SELECT a, AVG(a) OVER (ORDER BY a) FROM t ORDER BY rowid"
        ours, ref = _oracle(sql, SETUP_TIED)
        assert ours == ref

    def test_max_tied_values(self) -> None:
        """MAX(a) OVER (ORDER BY a) running maximum."""
        sql = "SELECT a, MAX(a) OVER (ORDER BY a) FROM t ORDER BY rowid"
        ours, ref = _oracle(sql, SETUP_TIED)
        assert ours == ref

    def test_range_peer_group_no_ties(self) -> None:
        """No ties → RANGE and ROWS semantics are identical; still passes."""
        sql = "SELECT a, COUNT(*) OVER (ORDER BY a) FROM t WHERE a != 2 ORDER BY rowid"
        ours, ref = _oracle(sql, SETUP_TIED)
        assert ours == ref

    def test_explicit_range_current_row(self) -> None:
        """Explicit RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW."""
        sql = (
            "SELECT a, COUNT(*) OVER "
            "(ORDER BY a RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) "
            "FROM t ORDER BY rowid"
        )
        ours, ref = _oracle(sql, SETUP_TIED)
        assert ours == ref

    def test_explicit_rows_current_row_unchanged(self) -> None:
        """Explicit ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW.

        ROWS mode uses physical position, so ties are NOT expanded.
        Results differ from RANGE when ties are present.
        """
        sql = (
            "SELECT a, COUNT(*) OVER "
            "(ORDER BY a ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) "
            "FROM t ORDER BY rowid"
        )
        ours, ref = _oracle(sql, SETUP_TIED)
        assert ours == ref
        # Under ROWS semantics: row 0→1, row 1→2, row 2→3, row 3→4, …
        assert ours == [(1, 1), (2, 2), (2, 3), (3, 4), (3, 5), (3, 6)]

    def test_range_peer_group_with_partition(self) -> None:
        """RANGE mode inside a PARTITION BY."""
        setup = [
            "CREATE TABLE p (grp TEXT, a INTEGER)",
            "INSERT INTO p VALUES ('x',1),('x',2),('x',2),('y',1),('y',1)",
        ]
        sql = (
            "SELECT grp, a, COUNT(*) OVER (PARTITION BY grp ORDER BY a) "
            "FROM p ORDER BY grp, rowid"
        )
        ours, ref = _oracle(sql, setup)
        assert ours == ref

    def test_no_order_by_full_partition_unchanged(self) -> None:
        """Without ORDER BY the full partition frame is unaffected by the fix."""
        sql = "SELECT a, COUNT(*) OVER () FROM t ORDER BY rowid"
        ours, ref = _oracle(sql, SETUP_TIED)
        assert ours == ref
        # All rows, full partition → count=6 for every row
        assert ours == [(1, 6), (2, 6), (2, 6), (3, 6), (3, 6), (3, 6)]
