"""Error translation — pipeline exceptions → PEP 249 classes."""

import pytest

import mini_sqlite
from mini_sqlite.errors import translate


def test_syntax_error_becomes_programming_error():
    conn = mini_sqlite.connect(":memory:")
    with pytest.raises(mini_sqlite.ProgrammingError):
        conn.execute("gibberish ???")


def test_unknown_table_becomes_operational_error():
    conn = mini_sqlite.connect(":memory:")
    with pytest.raises(mini_sqlite.OperationalError):
        conn.execute("SELECT * FROM nope")


def test_unknown_column_becomes_operational_error():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (a INTEGER)")
    with pytest.raises(mini_sqlite.OperationalError):
        conn.execute("SELECT missing FROM t")


def test_integrity_not_null():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (a INTEGER, b TEXT NOT NULL)")
    with pytest.raises(mini_sqlite.IntegrityError):
        conn.execute("INSERT INTO t (a) VALUES (1)")


def test_duplicate_create_without_if_not_exists():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (a INTEGER)")
    with pytest.raises(mini_sqlite.OperationalError):
        conn.execute("CREATE TABLE t (a INTEGER)")


def test_create_if_not_exists_is_fine():
    conn = mini_sqlite.connect(":memory:")
    conn.execute("CREATE TABLE t (a INTEGER)")
    conn.execute("CREATE TABLE IF NOT EXISTS t (a INTEGER)")


def test_translate_unknown_exception_is_internal():
    class Weird(Exception):
        pass

    err = translate(Weird("odd"))
    assert isinstance(err, mini_sqlite.InternalError)


def test_translate_already_pep249_error_passes_through():
    err = mini_sqlite.IntegrityError("boom")
    assert translate(err) is err
