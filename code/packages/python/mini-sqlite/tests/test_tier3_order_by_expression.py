"""Tests for ``ORDER BY <expr>`` with arbitrary expressions.

Previously, mini-sqlite raised
``InternalError: unexpected error: ValueError: tuple.index(x): x not in tuple``
when ``ORDER BY`` referenced an expression whose natural display name was
``"?"`` (the fallback the codegen used for un-named expressions like
``a+b``, ``UPPER(name)``, ``CASE WHEN … END``, …).

The fix extends sql-codegen's hidden-column injection to recognise the
``"?"`` case: each expression sort key is projected as a hidden trailing
column under a synthetic per-position name (``__sortkey_0``, …), the
SortKey IR is rewritten to look up that name, and StripTrailingColumns
removes the extras after the sort runs.  The result still matches
SQLite's ``ORDER BY`` semantics row-for-row.
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


_NUMS = (
    "CREATE TABLE t (a INT, b INT)",
    "INSERT INTO t VALUES (1, 5), (2, 3), (3, 4), (2, 2), (NULL, 1)",
)

_NAMES = (
    "CREATE TABLE u (name TEXT, age INT)",
    "INSERT INTO u VALUES ('bob', 30), ('Alice', 25), ('Carol', 28), ('alice', 25)",
)


class TestArithmeticExpression:
    def test_order_by_a_plus_b(self) -> None:
        _both_match(*_NUMS, query="SELECT a, b FROM t ORDER BY a+b")

    def test_order_by_a_times_2_desc(self) -> None:
        _both_match(*_NUMS, query="SELECT a FROM t ORDER BY (a*2) DESC")

    def test_order_by_a_minus_b(self) -> None:
        _both_match(*_NUMS, query="SELECT a, b FROM t ORDER BY a-b")


class TestFunctionCall:
    def test_order_by_upper(self) -> None:
        _both_match(*_NAMES, query="SELECT name FROM u ORDER BY UPPER(name)")

    def test_order_by_lower_desc(self) -> None:
        _both_match(*_NAMES, query="SELECT name FROM u ORDER BY LOWER(name) DESC")

    def test_order_by_length(self) -> None:
        _both_match(*_NAMES, query="SELECT name FROM u ORDER BY LENGTH(name), name")


class TestCaseExpression:
    def test_order_by_case_when(self) -> None:
        _both_match(
            *_NUMS,
            query=(
                "SELECT a FROM t ORDER BY "
                "CASE WHEN a < 2 THEN 0 WHEN a < 3 THEN 1 ELSE 2 END"
            ),
        )


class TestMultipleExpressionKeys:
    def test_two_expression_keys(self) -> None:
        _both_match(*_NUMS, query="SELECT a FROM t ORDER BY a+1, b-1")

    def test_expression_then_column(self) -> None:
        _both_match(*_NUMS, query="SELECT a FROM t ORDER BY a+b, a")

    def test_column_then_expression(self) -> None:
        _both_match(*_NUMS, query="SELECT a FROM t ORDER BY a, b*10")


class TestExpressionWithLimit:
    """ORDER BY <expr> LIMIT N must still strip hidden columns correctly."""

    def test_order_by_expression_with_limit(self) -> None:
        _both_match(*_NUMS, query="SELECT a, b FROM t ORDER BY a+b LIMIT 2")

    def test_order_by_expression_with_limit_offset(self) -> None:
        _both_match(
            *_NUMS,
            query="SELECT a, b FROM t ORDER BY a+b LIMIT 2 OFFSET 1",
        )


class TestExpressionWithDistinct:
    def test_distinct_order_by_expression(self) -> None:
        _both_match(*_NUMS, query="SELECT DISTINCT a FROM t ORDER BY a*10")
