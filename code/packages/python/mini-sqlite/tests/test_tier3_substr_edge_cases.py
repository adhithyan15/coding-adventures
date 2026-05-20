"""Oracle tests for ``SUBSTR(x, y[, z])`` — match SQLite byte-for-byte.

Companion to ``test_substr_edge_cases.py`` in ``sql-vm``.  These tests
go through the full mini-sqlite stack (lexer → parser → planner →
codegen → VM → scalar registry) and assert that the answer matches
the reference ``sqlite3`` module on the same query.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Standard usage — sanity baseline, no edge cases triggered
# ---------------------------------------------------------------------------


class TestSubstrBaseline:
    def test_basic_three_arg(self) -> None:
        _check("SELECT substr('hello', 1, 3)")

    def test_two_arg_to_end(self) -> None:
        _check("SELECT substr('hello', 2)")

    def test_zero_length(self) -> None:
        _check("SELECT substr('hello', 2, 0)")


# ---------------------------------------------------------------------------
# y = 0 — one position before the string
# ---------------------------------------------------------------------------


class TestSubstrZeroStart:
    def test_zero_with_length_3(self) -> None:
        # Was 'hel' (wrong); SQLite returns 'he'.
        _check("SELECT substr('hello', 0, 3)")

    def test_zero_with_length_1(self) -> None:
        _check("SELECT substr('hello', 0, 1)")

    def test_zero_with_length_5(self) -> None:
        _check("SELECT substr('hello', 0, 5)")

    def test_zero_with_zero_length(self) -> None:
        _check("SELECT substr('hello', 0, 0)")


# ---------------------------------------------------------------------------
# Negative y — count from end
# ---------------------------------------------------------------------------


class TestSubstrNegativeStart:
    def test_minus_one(self) -> None:
        _check("SELECT substr('hello', -1)")

    def test_minus_three(self) -> None:
        _check("SELECT substr('hello', -3)")

    def test_minus_n_whole_string(self) -> None:
        _check("SELECT substr('hello', -5)")

    def test_far_negative_no_length(self) -> None:
        # Was 'hello' (correct by accident); regression guard.
        _check("SELECT substr('hello', -100)")


# ---------------------------------------------------------------------------
# Negative z — characters preceding y
# ---------------------------------------------------------------------------


class TestSubstrNegativeLength:
    def test_minus_one(self) -> None:
        _check("SELECT substr('hello', 2, -1)")

    def test_minus_three_clipped(self) -> None:
        _check("SELECT substr('hello', 2, -3)")

    def test_minus_two_at_position_three(self) -> None:
        _check("SELECT substr('hello', 3, -2)")

    def test_minus_two_at_end(self) -> None:
        _check("SELECT substr('hello', 5, -2)")


# ---------------------------------------------------------------------------
# Out-of-range — far negative starts
# ---------------------------------------------------------------------------


class TestSubstrOutOfRange:
    def test_far_negative_short_length(self) -> None:
        # Was 'hello' (wrong); SQLite returns ''.
        _check("SELECT substr('hello', -100, 5)")

    def test_far_negative_long_length(self) -> None:
        # Was 'he' (wrong); SQLite returns 'hello'.
        _check("SELECT substr('hello', -100, 102)")

    def test_start_past_end(self) -> None:
        _check("SELECT substr('hello', 100)")


# ---------------------------------------------------------------------------
# Empty input and NULL
# ---------------------------------------------------------------------------


class TestSubstrEmptyAndNull:
    def test_empty_string(self) -> None:
        _check("SELECT substr('', 1)")

    def test_empty_with_length(self) -> None:
        _check("SELECT substr('', 1, 5)")

    def test_null_input(self) -> None:
        _check("SELECT substr(NULL, 1)")
        _check("SELECT substr(NULL, 1, 3)")
