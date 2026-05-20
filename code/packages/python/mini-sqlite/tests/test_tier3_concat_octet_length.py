"""Oracle tests for ``concat``, ``concat_ws``, and ``octet_length``.

Three SQLite 3.44+ string-family scalar functions that mini-sqlite was
missing.  Application SQL that depends on them previously raised::

    InternalError: unknown scalar function: 'concat'

All assertions compare against real ``sqlite3`` byte-for-byte.

NULL semantics differences pinned by tests:
- ``concat(NULL, x, NULL)`` → ``concat(x)`` (NULLs treated as '')
- ``concat_ws(NULL, ...)`` → NULL (NULL separator propagates)
- ``concat_ws(sep, x, NULL, y)`` → ``x`` ``sep`` ``y`` (NULL value skipped)
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(sql: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(sql).fetchone()
    r = sqlite3.connect(":memory:").execute(sql).fetchone()
    assert m == r, f"SQL: {sql!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# concat(...)
# ---------------------------------------------------------------------------


class TestConcat:
    def test_two_strings(self) -> None:
        _check("SELECT concat('a', 'b')")

    def test_three_strings(self) -> None:
        _check("SELECT concat('a', 'b', 'c')")

    def test_null_arg_treated_as_empty(self) -> None:
        _check("SELECT concat('a', NULL, 'c')")

    def test_all_nulls(self) -> None:
        _check("SELECT concat(NULL, NULL)")

    def test_numeric_coerced(self) -> None:
        _check("SELECT concat('id=', 42)")

    def test_float_coerced(self) -> None:
        _check("SELECT concat('value=', 3.14)")

    def test_single_arg_string(self) -> None:
        _check("SELECT concat('hello')")

    def test_in_where_clause(self) -> None:
        # Common usage: build composite key in WHERE / GROUP BY.
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        for s in [
            "CREATE TABLE t (a TEXT, b TEXT)",
            "INSERT INTO t VALUES ('hello', 'world'), ('foo', 'bar')",
        ]:
            conn_m.execute(s)
            conn_r.execute(s)
        sql = "SELECT * FROM t WHERE concat(a, ' ', b) = 'hello world'"
        assert conn_m.execute(sql).fetchall() == conn_r.execute(sql).fetchall()


# ---------------------------------------------------------------------------
# concat_ws(sep, ...)
# ---------------------------------------------------------------------------


class TestConcatWs:
    def test_basic_separator(self) -> None:
        _check("SELECT concat_ws('-', 'a', 'b', 'c')")

    def test_null_arg_skipped(self) -> None:
        # NULL skipped — separator NOT doubled around it.
        _check("SELECT concat_ws('-', 'a', NULL, 'c')")

    def test_null_separator_returns_null(self) -> None:
        _check("SELECT concat_ws(NULL, 'a', 'b')")

    def test_multi_char_separator(self) -> None:
        _check("SELECT concat_ws(' | ', 'a', 'b', 'c')")

    def test_empty_separator(self) -> None:
        _check("SELECT concat_ws('', 'a', 'b', 'c')")

    def test_numeric_args(self) -> None:
        _check("SELECT concat_ws(',', 1, 2, 3)")

    def test_all_nulls_returns_empty(self) -> None:
        # When all values (after sep) are NULL, the join joins zero pieces.
        _check("SELECT concat_ws('-', NULL, NULL)")


# ---------------------------------------------------------------------------
# octet_length(x)
# ---------------------------------------------------------------------------


class TestOctetLength:
    def test_ascii(self) -> None:
        _check("SELECT octet_length('hello')")

    def test_empty(self) -> None:
        _check("SELECT octet_length('')")

    def test_non_ascii(self) -> None:
        # 'café' — 4 chars but 5 bytes (é = 2 bytes in UTF-8).
        _check("SELECT octet_length('café')")

    def test_null(self) -> None:
        _check("SELECT octet_length(NULL)")

    def test_integer(self) -> None:
        # 3 chars = '1','2','3'
        _check("SELECT octet_length(123)")

    def test_negative_integer(self) -> None:
        # 3 chars = '-','4','2'
        _check("SELECT octet_length(-42)")

    def test_distinct_from_length_for_unicode(self) -> None:
        # length and octet_length disagree on non-ASCII text.
        # mini-sqlite vs sqlite3 should both report the same disagreement.
        m_l = mini_sqlite.connect(":memory:").execute(
            "SELECT length('café')"
        ).fetchone()[0]
        m_o = mini_sqlite.connect(":memory:").execute(
            "SELECT octet_length('café')"
        ).fetchone()[0]
        r_l = sqlite3.connect(":memory:").execute(
            "SELECT length('café')"
        ).fetchone()[0]
        r_o = sqlite3.connect(":memory:").execute(
            "SELECT octet_length('café')"
        ).fetchone()[0]
        assert m_l == r_l == 4
        assert m_o == r_o == 5
        assert m_l < m_o  # length < octet_length when non-ASCII present
