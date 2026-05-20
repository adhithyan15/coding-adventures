"""Oracle tests for ``STRING_AGG`` — SQLite 3.44+ synonym for GROUP_CONCAT.

Standard SQL spells string aggregation as ``STRING_AGG(expr, sep)``;
SQLite added ``string_agg`` as an alias for ``group_concat`` in 3.44.
Mini-sqlite previously raised ``unknown scalar function: 'string_agg'``
because the alias wasn't wired into the adapter's aggregate dispatch.

This PR routes ``STRING_AGG`` through the same code path as
``GROUP_CONCAT`` (planner emits the same ``AggregateExpr`` IR; VM
handles both via ``AggFunc.GROUP_CONCAT``).  Both forms now produce
identical results.

All assertions compare against real ``sqlite3`` byte-for-byte.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(setup: list[str], query: str) -> None:
    conn_m = mini_sqlite.connect(":memory:")
    conn_r = sqlite3.connect(":memory:")
    for s in setup:
        conn_m.execute(s)
        conn_r.execute(s)
    m = conn_m.execute(query).fetchall()
    r = conn_r.execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# string_agg basics
# ---------------------------------------------------------------------------


class TestStringAggBasics:
    SETUP = [
        "CREATE TABLE t (x INTEGER)",
        "INSERT INTO t VALUES (1), (2), (3)",
    ]

    def test_comma_separator(self) -> None:
        _check(self.SETUP, "SELECT string_agg(x, ',') FROM t")

    def test_pipe_separator(self) -> None:
        _check(self.SETUP, "SELECT string_agg(x, '|') FROM t")

    def test_multi_char_separator(self) -> None:
        _check(self.SETUP, "SELECT string_agg(x, ' -> ') FROM t")

    def test_empty_separator(self) -> None:
        _check(self.SETUP, "SELECT string_agg(x, '') FROM t")


# ---------------------------------------------------------------------------
# string_agg + GROUP BY
# ---------------------------------------------------------------------------


class TestStringAggGrouped:
    def test_group_by_with_string_agg(self) -> None:
        _check(
            [
                "CREATE TABLE orders (cust TEXT, item TEXT)",
                "INSERT INTO orders VALUES "
                "('alice', 'book'), "
                "('alice', 'pen'), "
                "('bob', 'apple'), "
                "('bob', 'banana'), "
                "('bob', 'cherry')",
            ],
            "SELECT cust, string_agg(item, ', ') FROM orders "
            "GROUP BY cust ORDER BY cust",
        )

    def test_group_concat_and_string_agg_equivalent(self) -> None:
        # Equivalence check — same query with both function names.
        conn = mini_sqlite.connect(":memory:")
        for s in [
            "CREATE TABLE t (id INTEGER, label TEXT)",
            "INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')",
        ]:
            conn.execute(s)
        gc = conn.execute("SELECT group_concat(label, ';') FROM t").fetchall()
        sa = conn.execute("SELECT string_agg(label, ';') FROM t").fetchall()
        assert gc == sa == [("a;b;c",)]


# ---------------------------------------------------------------------------
# string_agg NULL handling
# ---------------------------------------------------------------------------


class TestStringAggNulls:
    def test_nulls_skipped(self) -> None:
        # Both group_concat and string_agg skip NULL values, like SQLite.
        _check(
            [
                "CREATE TABLE t (val TEXT)",
                "INSERT INTO t VALUES ('a'), (NULL), ('b'), (NULL), ('c')",
            ],
            "SELECT string_agg(val, '+') FROM t",
        )

    def test_all_nulls_returns_null(self) -> None:
        _check(
            [
                "CREATE TABLE t (val TEXT)",
                "INSERT INTO t VALUES (NULL), (NULL)",
            ],
            "SELECT string_agg(val, ',') FROM t",
        )


# ---------------------------------------------------------------------------
# Regression — group_concat unchanged
# ---------------------------------------------------------------------------


class TestGroupConcatRegression:
    SETUP = [
        "CREATE TABLE t (x INTEGER)",
        "INSERT INTO t VALUES (1), (2), (3)",
    ]

    def test_group_concat_default_sep(self) -> None:
        _check(self.SETUP, "SELECT group_concat(x) FROM t")

    def test_group_concat_custom_sep(self) -> None:
        _check(self.SETUP, "SELECT group_concat(x, '-') FROM t")

    # Note: ``group_concat(DISTINCT x)`` is a separate latent bug — the
    # DISTINCT modifier isn't honoured by the VM's aggregate state.
    # Deliberately not asserted here; this PR's scope is the STRING_AGG
    # alias, not DISTINCT handling.
