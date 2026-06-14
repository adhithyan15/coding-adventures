"""Oracle tests for ``REPLACE(x, "", y)`` and ``printf("%#o", …)``.

Companion to ``test_replace_empty_and_octal.py`` in ``sql-vm``.  Goes
through the full mini-sqlite stack and asserts byte-for-byte equality
against the reference ``sqlite3`` module.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


class TestReplaceEmptyOracle:
    def test_empty_needle_no_op(self) -> None:
        # Was 'XhXeXlXlXoX' in mini-sqlite; SQLite returns 'hello'.
        _check("SELECT replace('hello', '', 'X')")

    def test_empty_needle_empty_haystack(self) -> None:
        _check("SELECT replace('', '', 'X')")

    def test_normal_replace_regression(self) -> None:
        _check("SELECT replace('hello', 'l', 'L')")


class TestPrintfHashOctalOracle:
    def test_zero(self) -> None:
        _check("SELECT printf('%#o', 0)")

    def test_eight(self) -> None:
        _check("SELECT printf('%#o', 8)")

    def test_sixty_four(self) -> None:
        _check("SELECT printf('%#o', 64)")

    def test_width_padded(self) -> None:
        _check("SELECT printf('%#5o', 8)")

    def test_zero_padded(self) -> None:
        _check("SELECT printf('%#05o', 8)")

    def test_no_hash_regression(self) -> None:
        # Without '#', SQLite emits the plain octal — must still work.
        _check("SELECT printf('%o', 8), printf('%05o', 8)")
