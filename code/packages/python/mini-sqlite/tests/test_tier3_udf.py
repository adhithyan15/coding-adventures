"""Tests for user-defined functions (UDFs) via conn.create_function()."""

import math

import pytest

import mini_sqlite


@pytest.fixture()
def conn():
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE nums (val REAL)")
    c.execute("INSERT INTO nums VALUES (4.0)")
    c.execute("INSERT INTO nums VALUES (9.0)")
    c.execute("INSERT INTO nums VALUES (16.0)")
    return c


def test_udf_basic(conn):
    """A simple UDF can be called from SQL."""
    conn.create_function("double", 1, lambda x: x * 2)
    # Insert values in ascending order so results are deterministic without ORDER BY.
    rows = conn.execute("SELECT double(val) FROM nums").fetchall()
    assert set(rows) == {(8.0,), (18.0,), (32.0,)}


def test_udf_math(conn):
    """UDF wrapping a stdlib function."""
    conn.create_function("sqrt", 1, math.sqrt)
    rows = conn.execute("SELECT sqrt(val) FROM nums").fetchall()
    assert set(rows) == {(2.0,), (3.0,), (4.0,)}


def test_udf_string_transform():
    """UDF on string data."""
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE t (s TEXT)")
    c.execute("INSERT INTO t VALUES ('hello')")
    c.execute("INSERT INTO t VALUES ('world')")
    c.create_function("shout", 1, lambda s: s.upper() + "!")
    rows = c.execute("SELECT shout(s) FROM t").fetchall()
    assert set(rows) == {("HELLO!",), ("WORLD!",)}


def test_udf_zero_args():
    """UDF with zero arguments acts as a constant generator."""
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE one (x INTEGER)")
    c.execute("INSERT INTO one VALUES (0)")
    c.create_function("forty_two", 0, lambda: 42)
    rows = c.execute("SELECT forty_two() FROM one").fetchall()
    assert rows == [(42,)]


def test_udf_variadic():
    """UDF registered with nargs=-1 accepts any number of arguments."""
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE one (x INTEGER)")
    c.execute("INSERT INTO one VALUES (0)")
    c.create_function("add_all", -1, lambda *args: sum(args))
    rows = c.execute("SELECT add_all(1, 2, 3, 4) FROM one").fetchall()
    assert rows == [(10,)]


def test_udf_shadows_builtin():
    """User-defined function takes precedence over a built-in of the same name."""
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE one (x INTEGER)")
    c.execute("INSERT INTO one VALUES (-3)")
    c.create_function("abs", 1, lambda x: x * 100)
    rows = c.execute("SELECT abs(x) FROM one").fetchall()
    assert rows == [(-300,)]


def test_udf_wrong_arg_count():
    """Calling a UDF with the wrong number of arguments raises an error."""
    c = mini_sqlite.connect(":memory:")
    c.create_function("double", 1, lambda x: x * 2)
    c.execute("CREATE TABLE one (x INTEGER)")
    c.execute("INSERT INTO one VALUES (1)")
    with pytest.raises(mini_sqlite.InternalError):
        c.execute("SELECT double(1, 2) FROM one").fetchall()


def test_udf_returns_none():
    """UDF returning None produces NULL in SQL."""
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE one (x INTEGER)")
    c.execute("INSERT INTO one VALUES (0)")
    c.create_function("null_fn", 0, lambda: None)
    rows = c.execute("SELECT null_fn() FROM one").fetchall()
    assert rows == [(None,)]


def test_udf_in_where():
    """UDF usable in WHERE clause."""
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE t (x INTEGER)")
    c.execute("INSERT INTO t VALUES (1)")
    c.execute("INSERT INTO t VALUES (2)")
    c.execute("INSERT INTO t VALUES (3)")
    c.create_function("is_even", 1, lambda x: 1 if x % 2 == 0 else 0)
    rows = c.execute("SELECT x FROM t WHERE is_even(x) = 1").fetchall()
    assert rows == [(2,)]


def test_udf_multiple_functions():
    """Multiple UDFs can be registered on the same connection."""
    c = mini_sqlite.connect(":memory:")
    c.execute("CREATE TABLE one (x INTEGER)")
    c.execute("INSERT INTO one VALUES (3)")
    c.create_function("add1", 1, lambda x: x + 1)
    c.create_function("mul2", 1, lambda x: x * 2)
    rows = c.execute("SELECT add1(x), mul2(x) FROM one").fetchall()
    assert rows == [(4, 6)]
