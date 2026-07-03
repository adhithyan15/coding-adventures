"""Tests for the unary ``+`` prefix operator.

SQLite accepts ``+`` as a unary prefix that is a documented no-op
identity::

    SELECT +5         ⟶  5
    SELECT +5.5       ⟶  5.5
    SELECT +(-3)      ⟶  -3
    SELECT 1 + +2     ⟶  3
    SELECT ++5        ⟶  5
    SELECT -+5        ⟶  -5

Mini-sqlite previously parse-errored on every occurrence of unary
``+`` because the grammar's ``unary`` rule only accepted ``-`` and
``~`` as prefixes.  Now ``+`` parses at the same precedence level
and the adapter unwraps it directly (no IR node is emitted because
the operand value is unchanged — that would only add a useless layer
for the planner / codegen to peel).
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


class TestBasic:
    def test_plus_int_literal(self) -> None:
        _both_match(query="SELECT +5")

    def test_plus_float_literal(self) -> None:
        _both_match(query="SELECT +5.5")

    def test_plus_zero(self) -> None:
        _both_match(query="SELECT +0")

    def test_plus_large_int(self) -> None:
        _both_match(query="SELECT +9223372036854775807")


class TestNestedAndChained:
    def test_plus_minus(self) -> None:
        _both_match(query="SELECT -+5")

    def test_double_plus(self) -> None:
        _both_match(query="SELECT ++5")

    def test_plus_paren_negative(self) -> None:
        _both_match(query="SELECT +(-3)")

    def test_plus_inside_addition(self) -> None:
        _both_match(query="SELECT 1 + +2")

    def test_minus_plus_minus(self) -> None:
        _both_match(query="SELECT -+-5")

    def test_plus_bitnot(self) -> None:
        _both_match(query="SELECT +~5")


class TestOnColumns:
    def test_plus_column_in_select(self) -> None:
        _both_match(
            "CREATE TABLE t (a INT)",
            "INSERT INTO t VALUES (1), (2), (3)",
            query="SELECT +a FROM t",
        )

    def test_plus_column_in_where(self) -> None:
        _both_match(
            "CREATE TABLE t (a INT)",
            "INSERT INTO t VALUES (1), (2), (3)",
            query="SELECT a FROM t WHERE a = +2",
        )

    def test_plus_column_in_order_by(self) -> None:
        _both_match(
            "CREATE TABLE t (a INT)",
            "INSERT INTO t VALUES (3), (1), (2)",
            query="SELECT a FROM t ORDER BY +a",
        )


class TestInExpressionContexts:
    def test_plus_in_arithmetic(self) -> None:
        _both_match(query="SELECT 10 - +3")

    def test_plus_in_case_branch(self) -> None:
        _both_match(query="SELECT CASE WHEN +1 = 1 THEN 'yes' END")

    def test_plus_in_func_arg(self) -> None:
        _both_match(query="SELECT ABS(+(-7))")
