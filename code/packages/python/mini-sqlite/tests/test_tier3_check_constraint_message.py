"""Tests for SQLite-compatible CHECK constraint error messages.

SQLite renders a failed CHECK predicate as
``CHECK constraint failed: <expr_text>`` — the original predicate
source text, not a column reference.  Mini-sqlite previously emitted
``CHECK constraint failed: <table>.<col>``, which doesn't tell the
user *why* the check failed and breaks tests that pin error
messages.

The fix plumbs the parsed expression source through
BackendColumnDef → IR ColumnDef → VM check_registry, then uses the
text in the ConstraintViolation message.  See sql-backend 0.21,
sql-codegen 1.41, sql-vm 1.56, and mini-sqlite 2.10 for the layered
implementation.
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite
from mini_sqlite import errors as mini_errors


def _expect_check_error(ddl: str, insert: str, expected_expr: str) -> None:
    """Assert that inserting *insert* into the *ddl* table raises a
    CHECK violation whose message contains ``CHECK constraint failed:
    <expected_expr>`` — exactly matching sqlite3's wording."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        c.execute(ddl)
    expected_msg = f"CHECK constraint failed: {expected_expr}"
    with pytest.raises(mini_errors.IntegrityError) as mini_exc:
        mini.execute(insert)
    with pytest.raises(sqlite3.IntegrityError) as ref_exc:
        ref.execute(insert)
    assert str(mini_exc.value) == expected_msg, (
        f"mini message {str(mini_exc.value)!r} does not match expected"
    )
    assert str(ref_exc.value) == expected_msg, (
        f"sqlite3 message {str(ref_exc.value)!r} drifted from expected"
    )


class TestSimpleComparison:
    def test_greater_than(self) -> None:
        _expect_check_error(
            "CREATE TABLE t (a INT CHECK (a > 0))",
            "INSERT INTO t VALUES (-1)",
            expected_expr="a > 0",
        )

    def test_less_equal(self) -> None:
        _expect_check_error(
            "CREATE TABLE t (a INT CHECK (a <= 100))",
            "INSERT INTO t VALUES (101)",
            expected_expr="a <= 100",
        )

    def test_not_equals(self) -> None:
        _expect_check_error(
            "CREATE TABLE t (name TEXT CHECK (name <> 'bad'))",
            "INSERT INTO t VALUES ('bad')",
            expected_expr="name <> 'bad'",
        )


class TestCompoundPredicate:
    def test_and_predicate(self) -> None:
        _expect_check_error(
            "CREATE TABLE t (a INT CHECK (a >= 0 AND a <= 100))",
            "INSERT INTO t VALUES (200)",
            expected_expr="a >= 0 AND a <= 100",
        )

    def test_or_predicate(self) -> None:
        _expect_check_error(
            "CREATE TABLE t (a INT CHECK (a = 1 OR a = 2))",
            "INSERT INTO t VALUES (3)",
            expected_expr="a = 1 OR a = 2",
        )


class TestFunctionCall:
    def test_length_function(self) -> None:
        _expect_check_error(
            "CREATE TABLE t (name TEXT CHECK (LENGTH(name) > 0))",
            "INSERT INTO t VALUES ('')",
            expected_expr="LENGTH(name) > 0",
        )

    def test_abs_function(self) -> None:
        _expect_check_error(
            "CREATE TABLE t (a INT CHECK (ABS(a) < 10))",
            "INSERT INTO t VALUES (15)",
            expected_expr="ABS(a) < 10",
        )


class TestInList:
    def test_in_list(self) -> None:
        _expect_check_error(
            "CREATE TABLE t (a INT CHECK (a IN (1, 2, 3)))",
            "INSERT INTO t VALUES (5)",
            expected_expr="a IN (1, 2, 3)",
        )
