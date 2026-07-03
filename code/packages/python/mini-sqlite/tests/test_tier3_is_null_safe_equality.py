"""Tests for SQLite's NULL-safe equality operator ``IS`` / ``IS NOT``.

SQLite extends the standard ISO SQL ``IS NULL`` / ``IS NOT NULL``
predicates with a general NULL-safe equality form::

    x IS y          ⟶  true iff x and y are equal OR both are NULL
                        (equivalent to ``x IS NOT DISTINCT FROM y``)
    x IS NOT y      ⟶  the negation
                        (equivalent to ``x IS DISTINCT FROM y``)

This is identical to PostgreSQL's ``IS NOT DISTINCT FROM`` /
``IS DISTINCT FROM`` operators in semantics but uses SQLite's compact
``IS`` spelling.  Mini-sqlite previously parse-errored on this form
because the grammar only accepted ``IS`` followed by ``NULL``,
``NOT NULL``, ``DISTINCT FROM …``, or ``NOT DISTINCT FROM …``.

The fix adds two grammar alternatives — ``"IS" collated`` and
``"IS" "NOT" collated`` — at the end of the IS family (after the
more specific NULL / DISTINCT forms so PEG ordering still works) and
routes them through the existing IS_[NOT_]DISTINCT_FROM planner/codegen
paths in the adapter.
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


class TestNullSafeEquality:
    def test_int_is_int_equal(self) -> None:
        _both_match(query="SELECT 1 IS 1")

    def test_int_is_int_unequal(self) -> None:
        _both_match(query="SELECT 1 IS 0")

    def test_null_is_null(self) -> None:
        _both_match(query="SELECT NULL IS NULL")

    def test_null_is_int(self) -> None:
        _both_match(query="SELECT NULL IS 1")

    def test_int_is_null(self) -> None:
        _both_match(query="SELECT 1 IS NULL")

    def test_string_is_string(self) -> None:
        _both_match(query="SELECT 'a' IS 'a'")

    def test_float_is_float(self) -> None:
        _both_match(query="SELECT 1.5 IS 1.5")

    def test_float_is_int_equal(self) -> None:
        # SQLite: 1 IS 1.0 → 1 (NULL-safe equality coerces)
        _both_match(query="SELECT 1 IS 1.0")


class TestNullSafeInequality:
    def test_int_is_not_int(self) -> None:
        _both_match(query="SELECT 1 IS NOT 2")

    def test_string_is_not_string(self) -> None:
        _both_match(query="SELECT 'a' IS NOT 'b'")

    def test_int_is_not_null(self) -> None:
        # Pre-existing "IS NOT NULL" predicate path — must still work.
        _both_match(query="SELECT 1 IS NOT NULL")

    def test_null_is_not_null(self) -> None:
        _both_match(query="SELECT NULL IS NOT NULL")

    def test_null_is_not_int(self) -> None:
        _both_match(query="SELECT NULL IS NOT 1")


class TestExistingFormsStillWork:
    """Regression: the original IS NULL / IS DISTINCT FROM paths must keep working."""

    def test_is_null(self) -> None:
        _both_match(query="SELECT 1 IS NULL")

    def test_is_not_null(self) -> None:
        _both_match(query="SELECT 1 IS NOT NULL")

    def test_is_distinct_from(self) -> None:
        _both_match(query="SELECT 1 IS DISTINCT FROM 2")

    def test_is_not_distinct_from(self) -> None:
        _both_match(query="SELECT 1 IS NOT DISTINCT FROM 1")

    def test_is_not_distinct_from_null(self) -> None:
        _both_match(query="SELECT NULL IS NOT DISTINCT FROM NULL")


class TestColumnContext:
    def test_is_against_column(self) -> None:
        _both_match(
            "CREATE TABLE t (a INT, b INT)",
            "INSERT INTO t VALUES (1, 1), (1, 2), (NULL, NULL), (NULL, 1), (1, NULL)",
            query="SELECT a, b FROM t WHERE a IS b ORDER BY rowid",
        )

    def test_is_not_against_column(self) -> None:
        _both_match(
            "CREATE TABLE t (a INT, b INT)",
            "INSERT INTO t VALUES (1, 1), (1, 2), (NULL, NULL), (NULL, 1)",
            query="SELECT a, b FROM t WHERE a IS NOT b ORDER BY rowid",
        )

    def test_is_in_where_with_null_match(self) -> None:
        # NULL-safe: WHERE a IS NULL is *equivalent* to WHERE a IS (literal NULL)
        _both_match(
            "CREATE TABLE t (a INT)",
            "INSERT INTO t VALUES (1), (NULL), (2)",
            query="SELECT a FROM t WHERE a IS NULL",
        )


class TestExpressionContext:
    def test_is_in_case_branch(self) -> None:
        _both_match(query="SELECT CASE WHEN NULL IS NULL THEN 'yes' END")

    def test_is_inside_not(self) -> None:
        _both_match(query="SELECT NOT (1 IS 2)")

    def test_chained_is_and_and(self) -> None:
        _both_match(query="SELECT (1 IS 1) AND (2 IS 2)")
