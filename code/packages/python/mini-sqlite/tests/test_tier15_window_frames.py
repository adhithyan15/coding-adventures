"""Tier 15 — Window function running aggregates and frame clause tests.

Covers three correctness gaps fixed in this PR:

1. **Running / cumulative aggregates**: When ``ORDER BY`` appears in a window
   spec, SQL requires the *default* frame to be
   ``RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW`` (cumulative), not
   the full-partition frame.  Affected functions: SUM, COUNT, COUNT(*), AVG,
   MIN, MAX, NTH_VALUE, LAST_VALUE.

2. **Explicit ROWS BETWEEN frame clause**: The parser now accepts
   ``ROWS BETWEEN frame_bound AND frame_bound`` inside ``OVER (…)`` so that
   users can explicitly control the frame extent.  Supported bounds:
   ``UNBOUNDED PRECEDING``, ``CURRENT ROW``, ``N PRECEDING``, ``N FOLLOWING``,
   ``UNBOUNDED FOLLOWING``.

3. **RANGE BETWEEN … (grammar)**: The same grammar extension also accepts
   ``RANGE BETWEEN …``; at the VM level RANGE is approximated as ROWS
   (correct when ORDER BY keys are distinct, which is the typical case).

All assertions are oracle-verified against real ``sqlite3``.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def _both(sql: str, *, setup: list[str] | None = None) -> tuple[list, list]:
    """Run *sql* against both real sqlite3 and mini-sqlite; return (ref, got)."""
    ref_con = sqlite3.connect(":memory:")
    got_con = mini_sqlite.connect(":memory:")
    for s in (setup or []):
        ref_con.execute(s)
        got_con.execute(s)
    ref_rows = ref_con.execute(sql).fetchall()
    got_rows = got_con.execute(sql).fetchall()
    return ref_rows, got_rows


_INT_TABLE = [
    "CREATE TABLE t (x INTEGER)",
    "INSERT INTO t VALUES (1)",
    "INSERT INTO t VALUES (2)",
    "INSERT INTO t VALUES (3)",
]

_EMP_TABLE = [
    "CREATE TABLE emp (name TEXT, dept TEXT, salary INTEGER)",
    "INSERT INTO emp VALUES ('Alice', 'Eng', 90000)",
    "INSERT INTO emp VALUES ('Bob',   'Eng', 70000)",
    "INSERT INTO emp VALUES ('Carol', 'HR',  80000)",
    "INSERT INTO emp VALUES ('Dave',  'HR',  60000)",
]


# ---------------------------------------------------------------------------
# 1. Default cumulative frame — ORDER BY changes SUM/COUNT/AVG/MIN/MAX
# ---------------------------------------------------------------------------


class TestRunningSum:
    """SUM OVER (ORDER BY) produces a cumulative (running) sum."""

    def test_running_sum_basic(self) -> None:
        ref, got = _both(
            "SELECT x, SUM(x) OVER (ORDER BY x) AS rs FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1), (2, 3), (3, 6)]

    def test_running_sum_partition(self) -> None:
        """Running SUM resets at each PARTITION BY group."""
        ref, got = _both(
            "SELECT dept, salary, SUM(salary) OVER (PARTITION BY dept ORDER BY salary) AS rs"
            " FROM emp ORDER BY dept, salary",
            setup=_EMP_TABLE,
        )
        assert got == ref

    def test_global_sum_no_order_by(self) -> None:
        """SUM OVER () with no ORDER BY uses the full-partition frame."""
        ref, got = _both(
            "SELECT x, SUM(x) OVER () AS total FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 6), (2, 6), (3, 6)]

    def test_running_sum_desc(self) -> None:
        """Running SUM ORDER BY DESC produces a reverse-cumulative sum."""
        ref, got = _both(
            "SELECT x, SUM(x) OVER (ORDER BY x DESC) AS rs FROM t ORDER BY x DESC",
            setup=_INT_TABLE,
        )
        assert got == ref


class TestRunningCount:
    """COUNT OVER (ORDER BY) counts cumulatively."""

    def test_running_count_col(self) -> None:
        ref, got = _both(
            "SELECT x, COUNT(x) OVER (ORDER BY x) AS rc FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1), (2, 2), (3, 3)]

    def test_running_count_star(self) -> None:
        ref, got = _both(
            "SELECT x, COUNT(*) OVER (ORDER BY x) AS rc FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1), (2, 2), (3, 3)]

    def test_global_count_star_no_order(self) -> None:
        """COUNT(*) OVER () → every row gets the total row count."""
        ref, got = _both(
            "SELECT x, COUNT(*) OVER () AS cnt FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 3), (2, 3), (3, 3)]


class TestRunningAvg:
    """AVG OVER (ORDER BY) computes a cumulative average."""

    def test_running_avg(self) -> None:
        ref, got = _both(
            "SELECT x, AVG(x) OVER (ORDER BY x) AS ra FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1.0), (2, 1.5), (3, 2.0)]


class TestRunningMinMax:
    """MIN / MAX OVER (ORDER BY) is cumulative."""

    def test_running_max(self) -> None:
        """Running MAX: grows monotonically when ORDER BY ASC."""
        ref, got = _both(
            "SELECT x, MAX(x) OVER (ORDER BY x) AS rm FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1), (2, 2), (3, 3)]

    def test_running_min(self) -> None:
        """Running MIN is the smallest value seen so far (constant for ASC data)."""
        ref, got = _both(
            "SELECT x, MIN(x) OVER (ORDER BY x) AS rm FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1), (2, 1), (3, 1)]

    def test_running_min_desc(self) -> None:
        """Running MIN ORDER BY DESC behaves like a reverse cumulative min."""
        ref, got = _both(
            "SELECT x, MIN(x) OVER (ORDER BY x DESC) AS rm FROM t ORDER BY x DESC",
            setup=_INT_TABLE,
        )
        assert got == ref


# ---------------------------------------------------------------------------
# 2. NTH_VALUE and LAST_VALUE respect the default cumulative frame
# ---------------------------------------------------------------------------


class TestNthValueFrame:
    """NTH_VALUE(col, n) returns NULL until the n-th row enters the frame."""

    def test_nth_value_per_row(self) -> None:
        ref, got = _both(
            "SELECT x, NTH_VALUE(x, 2) OVER (ORDER BY x) AS nv FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        # Row 1 frame=[1]  → no 2nd element → NULL
        # Row 2 frame=[1,2] → 2nd element = 2
        # Row 3 frame=[1,2,3] → 2nd element still 2
        assert got == ref == [(1, None), (2, 2), (3, 2)]

    def test_nth_value_n_too_large(self) -> None:
        """NTH_VALUE(x, 99) → always NULL (frame never reaches 99 rows)."""
        ref, got = _both(
            "SELECT x, NTH_VALUE(x, 99) OVER (ORDER BY x) AS nv FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, None), (2, None), (3, None)]

    def test_nth_value_no_order_by(self) -> None:
        """Without ORDER BY the frame is the full partition — NTH_VALUE is stable."""
        ref, got = _both(
            "SELECT x, NTH_VALUE(x, 2) OVER () AS nv FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref


class TestLastValueFrame:
    """LAST_VALUE respects the default cumulative frame when ORDER BY is present."""

    def test_last_value_default_frame(self) -> None:
        """Default frame UNBOUNDED PRECEDING → CURRENT ROW: last = current row."""
        ref, got = _both(
            "SELECT x, LAST_VALUE(x) OVER (ORDER BY x) AS lv FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1), (2, 2), (3, 3)]

    def test_last_value_full_partition(self) -> None:
        """Explicit full-partition frame: last value = last in partition (3)."""
        ref, got = _both(
            "SELECT x, LAST_VALUE(x) OVER"
            " (ORDER BY x ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING)"
            " AS lv FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 3), (2, 3), (3, 3)]


# ---------------------------------------------------------------------------
# 3. Explicit ROWS BETWEEN frame clause
# ---------------------------------------------------------------------------


class TestRowsBetweenUnboundedCurrentRow:
    """ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW is the explicit cumulative."""

    def test_sum_rows_unbounded_current(self) -> None:
        ref, got = _both(
            "SELECT x, SUM(x) OVER"
            " (ORDER BY x ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW)"
            " AS rs FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1), (2, 3), (3, 6)]


class TestRowsBetweenUnboundedFollowing:
    """ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING = full partition."""

    def test_sum_full_partition(self) -> None:
        ref, got = _both(
            "SELECT x, SUM(x) OVER"
            " (ORDER BY x ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING)"
            " AS rs FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 6), (2, 6), (3, 6)]

    def test_max_full_partition(self) -> None:
        ref, got = _both(
            "SELECT x, MAX(x) OVER"
            " (ORDER BY x ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING)"
            " AS mx FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 3), (2, 3), (3, 3)]


class TestRowsBetweenNPreceding:
    """ROWS BETWEEN N PRECEDING AND CURRENT ROW — sliding window of N+1 rows."""

    def test_sum_1_preceding(self) -> None:
        """2-row sliding sum: (1), (1+2), (2+3)."""
        ref, got = _both(
            "SELECT x, SUM(x) OVER"
            " (ORDER BY x ROWS BETWEEN 1 PRECEDING AND CURRENT ROW)"
            " AS rs FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1), (2, 3), (3, 5)]

    def test_sum_2_preceding(self) -> None:
        """3-row sliding sum over 5-row table."""
        setup = [
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (1)",
            "INSERT INTO t VALUES (2)",
            "INSERT INTO t VALUES (3)",
            "INSERT INTO t VALUES (4)",
            "INSERT INTO t VALUES (5)",
        ]
        ref, got = _both(
            "SELECT x, SUM(x) OVER"
            " (ORDER BY x ROWS BETWEEN 2 PRECEDING AND CURRENT ROW)"
            " AS rs FROM t ORDER BY x",
            setup=setup,
        )
        # Row 0: [1] → 1
        # Row 1: [1,2] → 3
        # Row 2: [1,2,3] → 6
        # Row 3: [2,3,4] → 9
        # Row 4: [3,4,5] → 12
        assert got == ref == [(1, 1), (2, 3), (3, 6), (4, 9), (5, 12)]

    def test_count_1_preceding(self) -> None:
        """Running COUNT with 1-preceding window."""
        ref, got = _both(
            "SELECT x, COUNT(x) OVER"
            " (ORDER BY x ROWS BETWEEN 1 PRECEDING AND CURRENT ROW)"
            " AS rc FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1), (2, 2), (3, 2)]

    def test_avg_1_preceding(self) -> None:
        """Moving average (2-row window)."""
        ref, got = _both(
            "SELECT x, AVG(x) OVER"
            " (ORDER BY x ROWS BETWEEN 1 PRECEDING AND CURRENT ROW)"
            " AS ra FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1.0), (2, 1.5), (3, 2.5)]


class TestRangeBetween:
    """RANGE BETWEEN … (approximated as ROWS when keys are distinct)."""

    def test_sum_range_unbounded_current(self) -> None:
        ref, got = _both(
            "SELECT x, SUM(x) OVER"
            " (ORDER BY x RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW)"
            " AS rs FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1), (2, 3), (3, 6)]

    def test_sum_range_full(self) -> None:
        ref, got = _both(
            "SELECT x, SUM(x) OVER"
            " (ORDER BY x RANGE BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING)"
            " AS rs FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 6), (2, 6), (3, 6)]


# ---------------------------------------------------------------------------
# 4. Non-aggregate window functions are unaffected by these changes
# ---------------------------------------------------------------------------


class TestRankingFunctionsUnchanged:
    """Ranking functions (ROW_NUMBER, RANK, DENSE_RANK, NTILE, PERCENT_RANK,
    CUME_DIST, LAG, LEAD) must continue to work correctly."""

    def test_row_number(self) -> None:
        ref, got = _both(
            "SELECT x, ROW_NUMBER() OVER (ORDER BY x) AS rn FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1), (2, 2), (3, 3)]

    def test_rank(self) -> None:
        setup = [
            "CREATE TABLE t (x INTEGER)",
            "INSERT INTO t VALUES (1)",
            "INSERT INTO t VALUES (1)",
            "INSERT INTO t VALUES (3)",
        ]
        ref, got = _both(
            "SELECT x, RANK() OVER (ORDER BY x) AS r FROM t ORDER BY x",
            setup=setup,
        )
        assert got == ref == [(1, 1), (1, 1), (3, 3)]

    def test_lag(self) -> None:
        ref, got = _both(
            "SELECT x, LAG(x) OVER (ORDER BY x) AS l FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, None), (2, 1), (3, 2)]

    def test_lead(self) -> None:
        ref, got = _both(
            "SELECT x, LEAD(x) OVER (ORDER BY x) AS l FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 2), (2, 3), (3, None)]

    def test_ntile(self) -> None:
        ref, got = _both(
            "SELECT x, NTILE(2) OVER (ORDER BY x) AS t FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref

    def test_percent_rank(self) -> None:
        ref, got = _both(
            "SELECT x, PERCENT_RANK() OVER (ORDER BY x) AS pr FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref

    def test_cume_dist(self) -> None:
        ref, got = _both(
            "SELECT x, CUME_DIST() OVER (ORDER BY x) AS cd FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref

    def test_first_value_unchanged(self) -> None:
        """FIRST_VALUE still returns the first value in the frame (partition start)."""
        ref, got = _both(
            "SELECT x, FIRST_VALUE(x) OVER (ORDER BY x) AS fv FROM t ORDER BY x",
            setup=_INT_TABLE,
        )
        assert got == ref == [(1, 1), (2, 1), (3, 1)]
