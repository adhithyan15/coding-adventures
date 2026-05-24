"""Tests for ``CAST(<bool> AS TEXT)`` matching SQLite's "no native bool" rule.

SQLite has no native boolean type — the ``TRUE`` and ``FALSE``
keywords are aliases for the integers ``1`` and ``0``.  Casting them
to TEXT therefore yields ``'1'`` and ``'0'``, not the strings
``'True'`` and ``'False'``.

Mini-sqlite's CAST handler used to call ``str(x)`` on the value
directly.  Python's ``str(True)`` returns ``'True'``, which leaked
the Python-level boolean repr into SQL output and broke any oracle
test that compared against sqlite3's ``'1'`` / ``'0'``.  The fix
detects ``bool`` before the generic str path (mirroring the
INTEGER-affinity path, which already special-cases bool) and
returns ``str(int(x))``.

These tests pin the corrected behaviour so a future refactor of the
CAST handler can't silently regress it.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both_match(query: str) -> None:
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    assert mini.execute(query).fetchall() == ref.execute(query).fetchall()


class TestBoolToText:
    def test_cast_true_as_text(self) -> None:
        _both_match("SELECT CAST(TRUE AS TEXT)")

    def test_cast_false_as_text(self) -> None:
        _both_match("SELECT CAST(FALSE AS TEXT)")

    def test_cast_true_as_varchar(self) -> None:
        _both_match("SELECT CAST(TRUE AS VARCHAR)")

    def test_cast_false_as_char(self) -> None:
        _both_match("SELECT CAST(FALSE AS CHAR)")


class TestRegressionIntegerPaths:
    """The INTEGER affinity path already handled bool — pin it so a
    refactor that touches the TEXT branch doesn't accidentally regress
    the INTEGER one."""

    def test_cast_true_as_integer(self) -> None:
        _both_match("SELECT CAST(TRUE AS INTEGER)")

    def test_cast_false_as_integer(self) -> None:
        _both_match("SELECT CAST(FALSE AS INTEGER)")

    def test_cast_true_as_real(self) -> None:
        _both_match("SELECT CAST(TRUE AS REAL)")


class TestPlainStringConversionStillWorks:
    """Regression: non-bool values keep their str() rendering."""

    def test_cast_int_as_text(self) -> None:
        _both_match("SELECT CAST(42 AS TEXT)")

    def test_cast_float_as_text(self) -> None:
        _both_match("SELECT CAST(3.14 AS TEXT)")

    def test_cast_null_as_text(self) -> None:
        _both_match("SELECT CAST(NULL AS TEXT)")


class TestInExpressionContext:
    def test_bool_text_in_concat(self) -> None:
        # Building a label that includes a boolean flag — common idiom.
        _both_match("SELECT 'is_active=' || CAST(TRUE AS TEXT)")

    def test_bool_text_in_where(self) -> None:
        _both_match(
            "SELECT 1 WHERE CAST(TRUE AS TEXT) = '1'",
        )
