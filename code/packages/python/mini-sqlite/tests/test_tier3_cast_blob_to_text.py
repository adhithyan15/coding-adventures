"""Tests for ``CAST(<blob> AS TEXT)`` UTF-8 decoding (matches SQLite).

SQLite's BLOB→TEXT cast treats the BLOB bytes as the encoded text
representation and UTF-8-decodes them.  So::

    CAST(x'48656c6c6f' AS TEXT)  ⟶  'Hello'   (not '48656c6c6f')
    CAST(x'31' AS TEXT)          ⟶  '1'       (not '31')
    CAST(x'3432' AS TEXT)        ⟶  '42'      (not '3432')

Mini-sqlite previously hex-encoded the bytes, breaking the
documented identity::

    CAST(CAST(n AS BLOB) AS TEXT) == CAST(n AS TEXT)

The fix (sql-vm 1.59.0) changes the BLOB → TEXT path in
``_cast_fn`` from ``x.hex()`` to ``x.decode("utf-8",
errors="replace")``.  Invalid UTF-8 bytes are mapped to U+FFFD
rather than raising — matches SQLite's "decode lazily, never error
mid-query" stance and keeps the cast total.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both_match(query: str) -> None:
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    assert mini.execute(query).fetchall() == ref.execute(query).fetchall()


class TestAsciiBlob:
    def test_hello(self) -> None:
        _both_match("SELECT CAST(x'48656c6c6f' AS TEXT)")

    def test_single_digit(self) -> None:
        _both_match("SELECT CAST(x'31' AS TEXT)")

    def test_multi_digit(self) -> None:
        _both_match("SELECT CAST(x'3432' AS TEXT)")

    def test_empty_blob(self) -> None:
        _both_match("SELECT CAST(x'' AS TEXT)")


class TestNumericRoundTrip:
    """``CAST(CAST(n AS BLOB) AS TEXT)`` recovers the textual form
    of ``n`` — pinning the documented SQLite identity."""

    def test_int_one(self) -> None:
        _both_match("SELECT CAST(CAST(1 AS BLOB) AS TEXT)")

    def test_int_forty_two(self) -> None:
        _both_match("SELECT CAST(CAST(42 AS BLOB) AS TEXT)")

    def test_int_negative(self) -> None:
        _both_match("SELECT CAST(CAST(-7 AS BLOB) AS TEXT)")

    def test_float_one_point_five(self) -> None:
        _both_match("SELECT CAST(CAST(1.5 AS BLOB) AS TEXT)")

    def test_bool_true(self) -> None:
        _both_match("SELECT CAST(CAST(TRUE AS BLOB) AS TEXT)")

    def test_bool_false(self) -> None:
        _both_match("SELECT CAST(CAST(FALSE AS BLOB) AS TEXT)")


class TestUtf8MultiByte:
    """Multi-byte UTF-8 sequences round-trip cleanly."""

    def test_e_acute(self) -> None:
        _both_match("SELECT CAST(x'c3a9' AS TEXT)")


class TestInvalidUtf8:
    """Invalid UTF-8 → U+FFFD replacement (slight divergence from
    sqlite3's text_factory which raises, but matches the SQLite
    engine's lenient decoding stance).  Pin mini's lenient
    behaviour so a future strict-mode addition is detected."""

    def test_invalid_byte_becomes_replacement_char(self) -> None:
        m = mini_sqlite.connect(":memory:")
        (val,) = m.execute("SELECT CAST(x'ff' AS TEXT)").fetchall()[0]
        assert val == "�"
