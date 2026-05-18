"""
JSON path-shortcut operators ``->`` and ``->>`` (SQLite 3.38+).

SQLite added two operators in 2022 as shortcuts for ``json_extract``:

    j -> path     — JSON-typed extraction (result is JSON text)
    j ->> path    — SQL-typed extraction (result is the unwrapped scalar)

Where ``path`` is one of:

    * an integer N           →  ``$[N]``  (array index)
    * a bare string ``"a"``  →  ``$.a``  (object key)
    * a string starting with ``$`` → used verbatim

Truth table (matches the real ``sqlite3`` module):

+---------------------------------+--------------------+
| Expression                      | Result             |
+=================================+====================+
| ``'[1,2,3]' -> 0``              | ``'1'`` (text)    |
| ``'[1,2,3]' ->> 0``             | ``1`` (integer)    |
| ``'{"a":1}' -> 'a'``            | ``'1'`` (text)    |
| ``'{"a":1}' ->> 'a'``           | ``1`` (integer)    |
| ``'{"a":{"b":3}}' -> 'a' -> 'b'``  | ``'3'``          |
| ``'{"a":{"b":3}}' -> 'a' ->> 'b'`` | ``3`` (integer)  |
+---------------------------------+--------------------+

Every test oracle-compares against the real ``sqlite3`` module.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(sql: str) -> None:
    mini = mini_sqlite.connect(":memory:").execute(sql).fetchone()
    ref = sqlite3.connect(":memory:").execute(sql).fetchone()
    assert mini == ref, f"SQL: {sql!r}\n  mini: {mini}\n  ref:  {ref}"


# ---------------------------------------------------------------------------
# Array index access
# ---------------------------------------------------------------------------


def test_arrow_array_index_first():
    _check("SELECT '[1,2,3]' -> 0")


def test_arrow_array_index_middle():
    _check("SELECT '[1,2,3]' -> 1")


def test_arrow_array_index_last():
    _check("SELECT '[10,20,30,40]' -> 3")


def test_arrow_text_array_index():
    _check("SELECT '[1,2,3]' ->> 0")


def test_arrow_text_array_string_value():
    _check("SELECT '[\"a\",\"b\",\"c\"]' ->> 1")


def test_arrow_array_out_of_bounds_returns_null():
    _check("SELECT '[1,2,3]' -> 99")


# ---------------------------------------------------------------------------
# Object key access
# ---------------------------------------------------------------------------


def test_arrow_object_key():
    _check("SELECT '{\"a\":1,\"b\":2}' -> 'a'")


def test_arrow_text_object_key():
    _check("SELECT '{\"a\":1,\"b\":2}' ->> 'a'")


def test_arrow_object_missing_key_is_null():
    _check("SELECT '{\"a\":1}' -> 'missing'")


def test_arrow_text_object_string_value():
    _check("SELECT '{\"name\":\"alice\"}' ->> 'name'")


# ---------------------------------------------------------------------------
# Chained operators
# ---------------------------------------------------------------------------


def test_arrow_chain_two_objects():
    _check("SELECT '{\"a\":{\"b\":42}}' -> 'a' -> 'b'")


def test_arrow_arrow_text_chain():
    _check("SELECT '{\"a\":{\"b\":42}}' -> 'a' ->> 'b'")


def test_arrow_chain_object_array_object():
    _check("SELECT '{\"users\":[{\"name\":\"alice\"}]}' -> 'users' -> 0 -> 'name'")


def test_arrow_text_chain_object_array_object():
    _check("SELECT '{\"users\":[{\"name\":\"bob\"}]}' -> 'users' -> 0 ->> 'name'")


# ---------------------------------------------------------------------------
# Explicit JSON path strings (start with $)
# ---------------------------------------------------------------------------


def test_arrow_with_explicit_dollar_path():
    _check("SELECT '{\"a\":{\"b\":42}}' ->> '$.a.b'")


def test_arrow_with_explicit_array_path():
    _check("SELECT '[10,20,30]' -> '$[2]'")


# ---------------------------------------------------------------------------
# NULL propagation
# ---------------------------------------------------------------------------


def test_arrow_null_left():
    _check("SELECT NULL -> 0")


def test_arrow_text_null_left():
    _check("SELECT NULL ->> 'a'")


def test_arrow_null_right():
    _check("SELECT '[1]' -> NULL")


# ---------------------------------------------------------------------------
# Object / array results stay as JSON text
# ---------------------------------------------------------------------------


def test_arrow_text_object_value_stays_json():
    """``->>`` on an object-shaped result returns the JSON text, not unwrapped."""
    _check("SELECT '{\"a\":{\"b\":1}}' ->> 'a'")


def test_arrow_array_value_returns_json():
    _check("SELECT '{\"items\":[1,2,3]}' -> 'items'")


# ---------------------------------------------------------------------------
# Combined with other SQL features
# ---------------------------------------------------------------------------


def test_arrow_in_where_clause():
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for con in (mini, ref):
        con.execute("CREATE TABLE t (id INTEGER, payload TEXT)")
        con.execute("INSERT INTO t VALUES (1, '{\"role\":\"admin\"}')")
        con.execute("INSERT INTO t VALUES (2, '{\"role\":\"user\"}')")
        con.execute("INSERT INTO t VALUES (3, '{\"role\":\"admin\"}')")
    sql = "SELECT id FROM t WHERE payload ->> 'role' = 'admin' ORDER BY id"
    got = mini.execute(sql).fetchall()
    exp = ref.execute(sql).fetchall()
    assert got == exp


def test_arrow_in_select_list():
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for con in (mini, ref):
        con.execute("CREATE TABLE t (id INTEGER, payload TEXT)")
        con.execute("INSERT INTO t VALUES (1, '{\"name\":\"alice\",\"age\":30}')")
        con.execute("INSERT INTO t VALUES (2, '{\"name\":\"bob\",\"age\":25}')")
    sql = "SELECT id, payload ->> 'name' AS name, payload ->> 'age' AS age FROM t ORDER BY id"
    got = mini.execute(sql).fetchall()
    exp = ref.execute(sql).fetchall()
    assert got == exp
