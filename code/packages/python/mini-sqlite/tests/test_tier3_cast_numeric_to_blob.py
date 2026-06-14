"""Tests for ``CAST(<numeric> AS BLOB)`` matching SQLite's text-first rule.

SQLite's numeric→BLOB cast goes through the TEXT representation
first: ``CAST(1 AS BLOB)`` yields the UTF-8 encoding of the
integer's decimal string (``b'1'``, one byte), not an 8-byte
big-endian packed integer.  The same rule applies to floats
(``CAST(1.5 AS BLOB)`` → ``b'1.5'``) and to booleans (``CAST(TRUE
AS BLOB)`` → ``b'1'`` because TRUE = 1).

Mini-sqlite's CAST handler previously called ``struct.pack(">q", x)``
for integers and ``struct.pack(">d", x)`` for floats — packing into
8-byte binary blobs that don't match SQLite's wire format and
silently corrupt round-trip ``CAST(n AS BLOB) AS TEXT`` patterns
that callers rely on for type-erased serialization.

See sql-vm 1.58.0 for the scalar-function fix.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both_match(query: str) -> None:
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    assert mini.execute(query).fetchall() == ref.execute(query).fetchall()


class TestIntToBlob:
    def test_zero(self) -> None:
        _both_match("SELECT CAST(0 AS BLOB)")

    def test_one(self) -> None:
        _both_match("SELECT CAST(1 AS BLOB)")

    def test_multi_digit(self) -> None:
        _both_match("SELECT CAST(42 AS BLOB)")

    def test_negative(self) -> None:
        _both_match("SELECT CAST(-7 AS BLOB)")

    def test_large(self) -> None:
        _both_match("SELECT CAST(9223372036854775807 AS BLOB)")


class TestBoolToBlob:
    """TRUE / FALSE are SQLite-level aliases for 1 / 0; the BLOB cast
    must follow the integer path, not Python's ``str(bool)``."""

    def test_true(self) -> None:
        _both_match("SELECT CAST(TRUE AS BLOB)")

    def test_false(self) -> None:
        _both_match("SELECT CAST(FALSE AS BLOB)")


class TestFloatToBlob:
    def test_one_point_five(self) -> None:
        _both_match("SELECT CAST(1.5 AS BLOB)")

    def test_negative_float(self) -> None:
        _both_match("SELECT CAST(-3.14 AS BLOB)")

    def test_zero_point(self) -> None:
        _both_match("SELECT CAST(0.0 AS BLOB)")


class TestPassthrough:
    """Regression: existing string and NULL paths must keep matching."""

    def test_string_passthrough(self) -> None:
        _both_match("SELECT CAST('hello' AS BLOB)")

    def test_empty_string(self) -> None:
        _both_match("SELECT CAST('' AS BLOB)")

    def test_null_blob(self) -> None:
        _both_match("SELECT CAST(NULL AS BLOB)")


class TestRoundTripIdentity:
    """``CAST(CAST(x AS BLOB) AS TEXT)`` recovers the original textual
    form for numeric ``x`` — now that the BLOB→TEXT direction
    UTF-8-decodes (mini-sqlite 2.15+ / sql-vm 1.59+).  Replaces the
    earlier ``TestKnownLimitationBlobToText`` pin which documented the
    hex-encoding divergence."""

    def test_int_roundtrip(self) -> None:
        _both_match("SELECT CAST(CAST(42 AS BLOB) AS TEXT)")

    def test_bool_roundtrip(self) -> None:
        _both_match("SELECT CAST(CAST(TRUE AS BLOB) AS TEXT)")

    def test_float_roundtrip(self) -> None:
        _both_match("SELECT CAST(CAST(1.5 AS BLOB) AS TEXT)")

    def test_negative_int_roundtrip(self) -> None:
        _both_match("SELECT CAST(CAST(-7 AS BLOB) AS TEXT)")
