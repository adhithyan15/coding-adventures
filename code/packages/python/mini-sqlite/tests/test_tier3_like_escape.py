"""
``LIKE … ESCAPE 'c'`` clause — escape character disables wildcard meaning.

SQLite syntax::

    expr LIKE pattern ESCAPE escape_char

The escape character (a single-character string literal) disables the special
meaning of the very next character in the pattern.  Most commonly used to
match literal ``%`` and ``_`` characters:

  • ``'50%' LIKE '50\\%' ESCAPE '\\'`` → 1   (matches literal ``%``)
  • ``'500' LIKE '50\\%' ESCAPE '\\'`` → 0   (no literal ``%`` in '500')
  • ``'a_b' LIKE 'a\\_b' ESCAPE '\\'`` → 1   (matches literal ``_``)
  • ``'aXb' LIKE 'a\\_b' ESCAPE '\\'`` → 0   (literal ``_``, not wildcard)

Every test in this file is an oracle comparison against the real ``sqlite3``
module.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(sql: str) -> None:
    mini = mini_sqlite.connect(":memory:").execute(sql).fetchone()
    ref = sqlite3.connect(":memory:").execute(sql).fetchone()
    assert mini == ref, f"SQL: {sql!r}\n  mini: {mini}\n  ref:  {ref}"


# ---------------------------------------------------------------------------
# Backslash as escape character
# ---------------------------------------------------------------------------


def test_escape_underscore_match():
    _check(r"SELECT 'a_b' LIKE 'a\_b' ESCAPE '\'")


def test_escape_underscore_no_match():
    _check(r"SELECT 'aXb' LIKE 'a\_b' ESCAPE '\'")


def test_escape_percent_match():
    _check(r"SELECT '50%' LIKE '50\%' ESCAPE '\'")


def test_escape_percent_no_match():
    _check(r"SELECT '500' LIKE '50\%' ESCAPE '\'")


def test_escape_percent_with_suffix():
    _check(r"SELECT '50%abc' LIKE '50\%abc' ESCAPE '\'")


# ---------------------------------------------------------------------------
# Dollar sign as escape character
# ---------------------------------------------------------------------------


def test_dollar_escape_underscore():
    _check("SELECT 'a_b' LIKE 'a$_b' ESCAPE '$'")


def test_dollar_escape_percent():
    _check("SELECT '50%' LIKE '50$%' ESCAPE '$'")


def test_dollar_escape_negative():
    _check("SELECT 'aXb' LIKE 'a$_b' ESCAPE '$'")


# ---------------------------------------------------------------------------
# Mixed escaped + unescaped wildcards
# ---------------------------------------------------------------------------


def test_mixed_escape_and_wildcard_percent():
    # Pattern: 'a$%%' ESCAPE '$' → literal 'a%' followed by % wildcard
    _check("SELECT 'a%xyz' LIKE 'a$%%' ESCAPE '$'")


def test_mixed_escape_and_wildcard_underscore():
    # Pattern: 'a$__' ESCAPE '$' → literal 'a_' followed by _ wildcard
    _check("SELECT 'a_Z' LIKE 'a$__' ESCAPE '$'")


# ---------------------------------------------------------------------------
# NOT LIKE … ESCAPE
# ---------------------------------------------------------------------------


def test_not_like_escape_matched():
    # 'a_b' does match the escaped pattern, so NOT LIKE → 0
    _check(r"SELECT 'a_b' NOT LIKE 'a\_b' ESCAPE '\'")


def test_not_like_escape_unmatched():
    # 'aXb' doesn't match the escaped pattern, so NOT LIKE → 1
    _check(r"SELECT 'aXb' NOT LIKE 'a\_b' ESCAPE '\'")


# ---------------------------------------------------------------------------
# WHERE clause with ESCAPE
# ---------------------------------------------------------------------------


def test_where_like_escape():
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    mini.execute("CREATE TABLE t (name TEXT)")
    ref.execute("CREATE TABLE t (name TEXT)")
    rows = [("user_1",), ("user%2",), ("user3",), ("admin_1",)]
    mini.executemany("INSERT INTO t VALUES (?)", rows)
    ref.executemany("INSERT INTO t VALUES (?)", rows)
    sql = r"SELECT name FROM t WHERE name LIKE 'user\_%' ESCAPE '\' ORDER BY name"
    got = mini.execute(sql).fetchall()
    exp = ref.execute(sql).fetchall()
    assert got == exp, f"got {got}\n  exp {exp}"


# ---------------------------------------------------------------------------
# Edge cases
# ---------------------------------------------------------------------------


def test_escape_with_no_special_chars():
    """Pattern with no wildcards behaves as exact match regardless of escape."""
    _check("SELECT 'hello' LIKE 'hello' ESCAPE '$'")


def test_escape_self_escape():
    """Escaping the escape character itself yields a literal escape char."""
    _check("SELECT 'a$b' LIKE 'a$$b' ESCAPE '$'")


def test_like_without_escape_still_works():
    """Plain LIKE (no ESCAPE clause) must still treat _ and % as wildcards."""
    _check("SELECT 'abc' LIKE 'a_c'")
    _check("SELECT 'abcdef' LIKE 'a%f'")
