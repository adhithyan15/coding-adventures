"""Hex integer literals — ``0x1F`` / ``0X1f`` — end-to-end oracle tests.

SQLite accepts ``0x`` / ``0X`` followed by one or more hex digits
(``[0-9A-Fa-f]+``) as an integer literal.  These are *integers only* —
SQLite does not parse a ``0x1.8p3`` IEEE-754 hex-float form.  Some
language details we exercise below:

* Both ``0x`` and ``0X`` prefixes accepted.
* Hex digits are case-insensitive (``0xff`` == ``0xFF`` == ``0xfF``).
* Result fits in a 64-bit signed integer (Python ints have no upper
  bound, but SQLite stores them as INTEGER which is 64-bit).
* Leading zeros after the prefix are fine: ``0x00ff`` == 255.
* ``-0x10`` parses as the unary-minus of ``0x10`` (a normal operator,
  not part of the literal syntax).
* Plays nicely with the bitwise operators added in mini-sqlite 1.77.

The lexer maps ``HEX_INT`` to the ``NUMBER`` token type so the grammar
keeps using a single literal-integer terminal everywhere — LIMIT,
OFFSET, frame offsets, expressions, etc.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = list(mini_sqlite.connect(":memory:").execute(query))
    r = list(sqlite3.connect(":memory:").execute(query))
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


def _check_with_table(setup: list[str], query: str) -> None:
    mc = mini_sqlite.connect(":memory:")
    rc = sqlite3.connect(":memory:")
    for s in setup:
        mc.execute(s)
        rc.execute(s)
    m = list(mc.execute(query))
    r = list(rc.execute(query))
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Basic literal forms.
# ---------------------------------------------------------------------------


class TestBasicHex:
    def test_zero(self) -> None:
        _check("SELECT 0x0")

    def test_small(self) -> None:
        _check("SELECT 0x1")

    def test_byte(self) -> None:
        _check("SELECT 0xff")

    def test_uppercase_x(self) -> None:
        _check("SELECT 0X10")

    def test_uppercase_digits(self) -> None:
        _check("SELECT 0xFF")

    def test_mixed_case_digits(self) -> None:
        _check("SELECT 0xfF")

    def test_word(self) -> None:
        _check("SELECT 0xDEADBEEF")

    def test_leading_zeros(self) -> None:
        # 0x00ff and 0xff should both yield 255.
        _check("SELECT 0x00ff")

    def test_multiple_columns(self) -> None:
        _check("SELECT 0x01, 0x02, 0x04, 0x08")


# ---------------------------------------------------------------------------
# In arithmetic expressions.
# ---------------------------------------------------------------------------


class TestHexInArithmetic:
    def test_plus(self) -> None:
        _check("SELECT 0x100 + 1")

    def test_minus(self) -> None:
        _check("SELECT 0xff - 0x0f")

    def test_mul(self) -> None:
        _check("SELECT 0x10 * 0x10")

    def test_div(self) -> None:
        _check("SELECT 0x100 / 0x10")

    def test_mod(self) -> None:
        _check("SELECT 0xff % 0x10")

    def test_unary_minus(self) -> None:
        # ``-`` is a unary operator, not part of the literal.
        _check("SELECT -0x10")


# ---------------------------------------------------------------------------
# With bitwise operators (the original motivating use case).
# ---------------------------------------------------------------------------


class TestHexWithBitwise:
    def test_and(self) -> None:
        _check("SELECT 0xff & 0x0f")

    def test_or(self) -> None:
        _check("SELECT 0xf0 | 0x0f")

    def test_xor_via_not(self) -> None:
        # SQLite has no native XOR; emulate with (a | b) & ~(a & b).
        _check("SELECT (0xff | 0x0f) & ~(0xff & 0x0f)")

    def test_shl(self) -> None:
        _check("SELECT 0x01 << 8")

    def test_shr(self) -> None:
        _check("SELECT 0xff00 >> 8")

    def test_not(self) -> None:
        _check("SELECT ~0xff")


# ---------------------------------------------------------------------------
# In WHERE clauses and against table data.
# ---------------------------------------------------------------------------


class TestHexAgainstTables:
    setup = [
        "CREATE TABLE flags(id INTEGER, mask INTEGER)",
        "INSERT INTO flags VALUES (1, 0x01), (2, 0x02), (3, 0x04), (4, 0x07)",
    ]

    def test_where_eq(self) -> None:
        _check_with_table(self.setup, "SELECT id FROM flags WHERE mask = 0x02")

    def test_where_bitwise(self) -> None:
        _check_with_table(
            self.setup, "SELECT id FROM flags WHERE mask & 0x01 ORDER BY id"
        )

    def test_select_bitmask(self) -> None:
        _check_with_table(
            self.setup,
            "SELECT id, mask & 0x03 FROM flags ORDER BY id",
        )


# ---------------------------------------------------------------------------
# LIMIT / OFFSET accept hex too (the offset code path goes through a
# separate _parse_number call, so it needs its own coverage).
# ---------------------------------------------------------------------------


class TestHexInLimit:
    setup = [
        "CREATE TABLE t(id INTEGER)",
        "INSERT INTO t VALUES (1), (2), (3), (4), (5), (6), (7), (8)",
    ]

    def test_limit_hex(self) -> None:
        _check_with_table(self.setup, "SELECT id FROM t ORDER BY id LIMIT 0x03")

    def test_limit_offset_hex(self) -> None:
        _check_with_table(
            self.setup, "SELECT id FROM t ORDER BY id LIMIT 0x04 OFFSET 0x02"
        )


# ---------------------------------------------------------------------------
# INSERT — hex literals in VALUES.
# ---------------------------------------------------------------------------


class TestHexEdgeCases:
    """SQLite-faithful edge cases that matter for byte-compat."""

    def test_max_64bit_unsigned_wraps_to_neg_one(self) -> None:
        # 0xFFFFFFFFFFFFFFFF = 2^64 - 1, but stored as 64-bit signed → -1.
        _check("SELECT 0xFFFFFFFFFFFFFFFF")

    def test_high_bit_wraps_to_min_int64(self) -> None:
        # 0x8000000000000000 = 2^63 → -2^63 in 64-bit signed.
        _check("SELECT 0x8000000000000000")

    def test_seventeen_digits_raises(self) -> None:
        # > 16 hex digits exceeds 64 bits → SQLite raises "hex literal too big".
        import pytest

        from mini_sqlite.errors import OperationalError

        with pytest.raises(OperationalError, match="hex literal too big"):
            mini_sqlite.connect(":memory:").execute("SELECT 0x10000000000000000")

    def test_huge_hex_rejected_quickly(self) -> None:
        # Defense in depth: a megabyte-sized hex literal must NOT trigger
        # the O(N²) int(s, 16) path.  We reject at the length-check step.
        import pytest

        from mini_sqlite.errors import OperationalError

        big = "SELECT 0x" + "F" * 1_000_000
        with pytest.raises(OperationalError, match="hex literal too big"):
            mini_sqlite.connect(":memory:").execute(big)


class TestHexInInsert:
    def test_insert_hex_values(self) -> None:
        # End-to-end through INSERT, storage, and SELECT.
        mc = mini_sqlite.connect(":memory:")
        rc = sqlite3.connect(":memory:")
        for db in (mc, rc):
            db.execute("CREATE TABLE t(x INTEGER)")
            db.execute("INSERT INTO t VALUES (0x10), (0xff), (0x100)")
        m = list(mc.execute("SELECT x FROM t ORDER BY x"))
        r = list(rc.execute("SELECT x FROM t ORDER BY x"))
        assert m == r, f"mini: {m}, ref: {r}"
