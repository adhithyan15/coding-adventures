"""Oracle tests for named WINDOW clause (WINDOW w AS (...) / OVER w).

SQLite allows a SELECT to define one or more window specifications by name
in a trailing WINDOW clause and then reference those names in OVER clauses:

    SELECT col, ROW_NUMBER() OVER w
    FROM   tbl
    WINDOW w AS (PARTITION BY p ORDER BY o)
    ORDER BY col;

Every test here runs the same query against both mini_sqlite and the stdlib
``sqlite3`` module and asserts byte-for-byte identical results (the oracle
pattern used throughout the tier-3 suite).
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _oracle(sql: str, setup: list[str] | None = None) -> tuple[list, list]:
    """Run *sql* in both engines and return (mini_sqlite_rows, sqlite3_rows)."""
    ref = sqlite3.connect(":memory:")
    ours = mini_sqlite.connect(":memory:")
    for s in setup or []:
        ref.execute(s)
        ours.execute(s)
    ref_rows = ref.execute(sql).fetchall()
    our_rows = ours.execute(sql).fetchall()
    return our_rows, ref_rows


SETUP = [
    "CREATE TABLE t (id INTEGER, grp TEXT, val INTEGER)",
    "INSERT INTO t VALUES (1,'a',10),(2,'a',20),(3,'b',30),(4,'b',40),(5,'c',50)",
]

SETUP_SMALL = [
    "CREATE TABLE s (n INTEGER)",
    "INSERT INTO s VALUES (3),(1),(2)",
]


# ---------------------------------------------------------------------------
# Basic named window — single window, no partition
# ---------------------------------------------------------------------------


def test_row_number_named_window():
    """ROW_NUMBER() OVER w where w is ORDER BY id."""
    sql = (
        "SELECT id, ROW_NUMBER() OVER w FROM t "
        "WINDOW w AS (ORDER BY id) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref
    assert ours == [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]


def test_rank_named_window():
    """RANK() OVER w with ORDER BY val."""
    sql = (
        "SELECT id, RANK() OVER w FROM t "
        "WINDOW w AS (ORDER BY val) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


def test_dense_rank_named_window():
    """DENSE_RANK() OVER w."""
    sql = (
        "SELECT id, DENSE_RANK() OVER w FROM t "
        "WINDOW w AS (ORDER BY val) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


# ---------------------------------------------------------------------------
# Named window with PARTITION BY
# ---------------------------------------------------------------------------


def test_row_number_with_partition():
    """ROW_NUMBER() resets per partition when PARTITION BY is in the named window."""
    sql = (
        "SELECT id, grp, ROW_NUMBER() OVER w FROM t "
        "WINDOW w AS (PARTITION BY grp ORDER BY id) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref
    # rows 1,2 → grp='a' rank 1,2; rows 3,4 → 'b' rank 1,2; row 5 → 'c' rank 1
    assert [r[2] for r in ours] == [1, 2, 1, 2, 1]


def test_sum_with_partition():
    """Running SUM over named window partitioned by grp."""
    sql = (
        "SELECT id, SUM(val) OVER w FROM t "
        "WINDOW w AS (PARTITION BY grp ORDER BY id) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


def test_count_star_with_partition():
    """COUNT(*) OVER w partitioned by grp."""
    sql = (
        "SELECT id, COUNT(*) OVER w FROM t "
        "WINDOW w AS (PARTITION BY grp ORDER BY id) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


# ---------------------------------------------------------------------------
# Multiple named windows in one query
# ---------------------------------------------------------------------------


def test_two_named_windows():
    """Two separate named windows referenced by two columns."""
    sql = (
        "SELECT id, ROW_NUMBER() OVER w1, SUM(val) OVER w2 FROM t "
        "WINDOW w1 AS (ORDER BY id), w2 AS (ORDER BY id) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


def test_two_named_windows_different_partitions():
    """w1 has no partition, w2 partitions by grp."""
    sql = (
        "SELECT id, ROW_NUMBER() OVER w1, RANK() OVER w2 FROM t "
        "WINDOW w1 AS (ORDER BY id), w2 AS (PARTITION BY grp ORDER BY val) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


# ---------------------------------------------------------------------------
# Named window mixed with inline OVER
# ---------------------------------------------------------------------------


def test_named_and_inline_mixed():
    """One column uses a named window; another uses inline OVER."""
    sql = (
        "SELECT id, ROW_NUMBER() OVER w, RANK() OVER (ORDER BY val) FROM t "
        "WINDOW w AS (ORDER BY id) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


# ---------------------------------------------------------------------------
# Aggregate window functions via named window
# ---------------------------------------------------------------------------


def test_avg_named_window():
    """AVG(val) OVER w (running average)."""
    sql = (
        "SELECT id, AVG(val) OVER w FROM t "
        "WINDOW w AS (ORDER BY id) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


def test_min_max_named_window():
    """MIN and MAX over the same named window."""
    sql = (
        "SELECT id, MIN(val) OVER w, MAX(val) OVER w FROM t "
        "WINDOW w AS (ORDER BY id) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


# ---------------------------------------------------------------------------
# LAG / LEAD via named window
# ---------------------------------------------------------------------------


def test_lag_named_window():
    """LAG(val) OVER w with default offset=1."""
    sql = (
        "SELECT id, LAG(val) OVER w FROM t "
        "WINDOW w AS (ORDER BY id) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


def test_lead_named_window():
    """LEAD(val) OVER w."""
    sql = (
        "SELECT id, LEAD(val) OVER w FROM t "
        "WINDOW w AS (ORDER BY id) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


# ---------------------------------------------------------------------------
# NTILE via named window
# ---------------------------------------------------------------------------


def test_ntile_named_window():
    """NTILE(2) OVER w splits 5 rows into 2 buckets."""
    sql = (
        "SELECT id, NTILE(2) OVER w FROM t "
        "WINDOW w AS (ORDER BY id) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


# ---------------------------------------------------------------------------
# FIRST_VALUE / LAST_VALUE via named window
# ---------------------------------------------------------------------------


def test_first_value_named_window():
    """FIRST_VALUE(val) OVER w (with ROWS BETWEEN to match SQLite default)."""
    sql = (
        "SELECT id, FIRST_VALUE(val) OVER w FROM t "
        "WINDOW w AS (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


def test_last_value_named_window():
    """LAST_VALUE(val) OVER w (with ROWS BETWEEN UNBOUNDED for full window)."""
    sql = (
        "SELECT id, LAST_VALUE(val) OVER w FROM t "
        "WINDOW w AS (ORDER BY id "
        "ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) ORDER BY id"
    )
    ours, ref = _oracle(sql, SETUP)
    assert ours == ref


# ---------------------------------------------------------------------------
# Error: undefined window name
# ---------------------------------------------------------------------------


def test_undefined_window_name_raises():
    """OVER <undefined_name> should raise an error (not crash silently)."""
    with pytest.raises(mini_sqlite.OperationalError):
        mini_sqlite.connect(":memory:").execute(
            "SELECT ROW_NUMBER() OVER nosuchwindow FROM (SELECT 1 AS x)"
        )


# ---------------------------------------------------------------------------
# Edge: named window on a trivial subquery (no FROM table)
# ---------------------------------------------------------------------------


def test_named_window_small_table():
    """Named window on a small 3-row table sorted in non-natural order."""
    sql = (
        "SELECT n, ROW_NUMBER() OVER w FROM s "
        "WINDOW w AS (ORDER BY n) ORDER BY n"
    )
    ours, ref = _oracle(sql, SETUP_SMALL)
    assert ours == ref
    assert ours == [(1, 1), (2, 2), (3, 3)]
