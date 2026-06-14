"""Oracle tests for SQLite's ``%q``, ``%Q``, ``%w`` printf conversions.

These pair each interesting input against the reference ``sqlite3``
module and assert byte-for-byte equality.  See the sql-vm package for
finer-grained unit tests against the formatter directly.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


class TestPercentQOracle:
    def test_no_quotes(self) -> None:
        _check("SELECT printf('%q', 'hello')")

    def test_single_quote_doubled(self) -> None:
        _check("SELECT printf('%q', 'it''s')")

    def test_multiple_quotes(self) -> None:
        _check("SELECT printf('%q', 'a''b''c')")

    def test_null(self) -> None:
        _check("SELECT printf('%q', NULL)")


class TestPercentBigQOracle:
    def test_wrapping(self) -> None:
        _check("SELECT printf('%Q', 'hello')")

    def test_quote_in_string(self) -> None:
        _check("SELECT printf('%Q', 'it''s')")

    def test_null_becomes_literal(self) -> None:
        _check("SELECT printf('%Q', NULL)")


class TestPercentWOracle:
    def test_plain_identifier(self) -> None:
        _check("SELECT printf('%w', 'hello')")

    def test_double_quote_doubled(self) -> None:
        # SQL string literal with a single embedded double quote.
        _check("SELECT printf('%w', 'a\"b')")

    def test_multiple_double_quotes(self) -> None:
        _check("SELECT printf('%w', 'col\"name\"x')")

    def test_single_quotes_left_alone(self) -> None:
        _check("SELECT printf('%w', 'it''s')")

    def test_null(self) -> None:
        _check("SELECT printf('%w', NULL)")


class TestComposition:
    def test_build_values_clause(self) -> None:
        _check("SELECT printf('VALUES(%Q)', 'O''Brien')")

    def test_build_quoted_identifier(self) -> None:
        _check("SELECT printf('SELECT \"%w\" FROM t', 'odd\"col')")

    def test_mix_s_and_q(self) -> None:
        _check("SELECT printf('%s = %q', 'col', 'O''Brien')")
