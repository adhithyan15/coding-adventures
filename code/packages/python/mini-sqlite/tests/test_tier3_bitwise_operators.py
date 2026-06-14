"""Bitwise operators — &, |, <<, >>, ~ — end-to-end oracle tests.

SQLite defines five bitwise operators that all operate on 64-bit signed
integer values:

* ``a & b``   bitwise AND
* ``a | b``   bitwise OR
* ``a << b``  left shift (b ≥ 64 → 0; b < 0 → right shift by |b|)
* ``a >> b``  right shift (arithmetic — sign bit propagates;
              b ≥ 64 → 0 for non-negative a, -1 for negative a;
              b < 0  → left shift by |b|)
* ``~a``      bitwise NOT (one's complement)

Operand coercion rules (matching SQLite):

* booleans → 0 / 1
* integers → as-is
* floats   → truncated toward zero (``5.7 & 3`` ≡ ``5 & 3``)
* strings  → SQLite parses NUMERIC affinity; we conservatively raise
            TypeMismatch.  The oracle tests below avoid this corner.

Result-type rules:

* Result is always an integer.
* 64-bit two's-complement wrap-around: ``1 << 63`` → ``-2**63``,
  not ``+2**63``.  Without this, the VM would happily emit unbounded
  Python ints and silently disagree with SQLite for large shifts.

Precedence (per the SQLite reference):

    unary (-, ~) > * / % > + - || > & | << >> > comparisons

These tests run each expression through both ``mini_sqlite`` and the
stdlib ``sqlite3`` module and require byte-identical results.
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
# Basic literal forms (exercise the constant folder).
# ---------------------------------------------------------------------------


class TestBitAnd:
    def test_basic(self) -> None:
        _check("SELECT 5 & 3")

    def test_zero(self) -> None:
        _check("SELECT 5 & 0")

    def test_self(self) -> None:
        _check("SELECT 12 & 12")

    def test_neg_one(self) -> None:
        # -1 is all-bits-set in two's complement, so x & -1 == x.
        _check("SELECT 42 & -1")

    def test_mask_low_byte(self) -> None:
        # Note: hex literals (0x…) are not supported here yet, so we use
        # decimal forms.  4660 = 0x1234, 255 = 0xff.
        _check("SELECT 4660 & 255")


class TestBitOr:
    def test_basic(self) -> None:
        _check("SELECT 5 | 3")

    def test_zero(self) -> None:
        _check("SELECT 5 | 0")

    def test_neg_one(self) -> None:
        _check("SELECT 42 | -1")

    def test_compose_with_shift(self) -> None:
        _check("SELECT (1 << 3) | (1 << 5)")


class TestShiftLeft:
    def test_basic(self) -> None:
        _check("SELECT 1 << 4")

    def test_zero_shift(self) -> None:
        _check("SELECT 7 << 0")

    def test_high_bit(self) -> None:
        # Wraps to -2**63 — this is the canonical 64-bit overflow case
        # that catches "did you remember to mask to 64 bits?" bugs.
        _check("SELECT 1 << 63")

    def test_overflow_shift(self) -> None:
        _check("SELECT 1 << 64")

    def test_negative_shift(self) -> None:
        # SQLite reinterprets ``a << -b`` as ``a >> b``.
        _check("SELECT 16 << -2")


class TestShiftRight:
    def test_basic(self) -> None:
        _check("SELECT 16 >> 2")

    def test_zero_shift(self) -> None:
        _check("SELECT 7 >> 0")

    def test_arithmetic_negative(self) -> None:
        # SQLite uses arithmetic shift — sign bit propagates.
        _check("SELECT -8 >> 1")

    def test_overflow_positive(self) -> None:
        # Non-negative >> 64 → 0
        _check("SELECT 12345 >> 64")

    def test_overflow_negative(self) -> None:
        # Negative >> 64 → -1 (sign bit fills the whole word).
        _check("SELECT -12345 >> 64")

    def test_negative_shift(self) -> None:
        _check("SELECT 8 >> -1")


class TestBitNot:
    def test_basic(self) -> None:
        _check("SELECT ~0")

    def test_neg_one(self) -> None:
        _check("SELECT ~-1")

    def test_positive(self) -> None:
        _check("SELECT ~5")

    def test_double_not(self) -> None:
        _check("SELECT ~~5")

    def test_with_neg(self) -> None:
        _check("SELECT -~5")


# ---------------------------------------------------------------------------
# Column references (exercise the VM path, bypassing constant folding).
# ---------------------------------------------------------------------------


class TestBitwiseWithColumns:
    setup = [
        "CREATE TABLE t(x INTEGER)",
        "INSERT INTO t VALUES (1), (2), (4), (8), (-1)",
    ]

    def test_and_column(self) -> None:
        _check_with_table(self.setup, "SELECT x, x & 3 FROM t ORDER BY x")

    def test_or_column(self) -> None:
        _check_with_table(self.setup, "SELECT x, x | 16 FROM t ORDER BY x")

    def test_shl_column(self) -> None:
        _check_with_table(self.setup, "SELECT x, x << 2 FROM t ORDER BY x")

    def test_shr_column(self) -> None:
        _check_with_table(self.setup, "SELECT x, x >> 1 FROM t ORDER BY x")

    def test_not_column(self) -> None:
        _check_with_table(self.setup, "SELECT x, ~x FROM t ORDER BY x")

    def test_high_bit_via_column(self) -> None:
        # Verifies the VM (not just the folder) does 64-bit wrap.
        _check_with_table(
            ["CREATE TABLE u(x INTEGER)", "INSERT INTO u VALUES (1)"],
            "SELECT x << 63 FROM u",
        )


# ---------------------------------------------------------------------------
# Precedence — bitwise binds looser than arithmetic, tighter than comparison.
# ---------------------------------------------------------------------------


class TestPrecedence:
    def test_shl_vs_add(self) -> None:
        # ``1 << 2 + 1`` = ``1 << 3`` = 8, not ``(1<<2) + 1`` = 5.
        _check("SELECT 1 << 2 + 1")

    def test_and_vs_or_left_assoc(self) -> None:
        # All four bitwise ops are at one precedence level, left-associative,
        # so ``4 | 2 & 1`` is ``(4|2)&1`` = 0, not ``4 | (2&1)`` = 4.
        _check("SELECT 4 | 2 & 1")

    def test_bitwise_vs_comparison(self) -> None:
        # Comparison binds looser than bitwise → ``5 & 3 = 1`` parses as
        # ``(5 & 3) = 1`` = TRUE.
        _check("SELECT 5 & 3 = 1")

    def test_unary_not_binds_tightest(self) -> None:
        # ``~5 + 1`` parses as ``(~5) + 1`` = -6 + 1 = -5.
        _check("SELECT ~5 + 1")

    def test_double_unary(self) -> None:
        # Unary operators are right-associative — ``-~5`` parses as
        # ``-(~5)`` = -(-6) = 6.
        _check("SELECT -~5")


# ---------------------------------------------------------------------------
# Operand coercion: floats truncate toward zero before the bitwise op.
# ---------------------------------------------------------------------------


class TestFloatTruncation:
    def test_and_float_lhs(self) -> None:
        _check("SELECT 5.7 & 3")

    def test_and_float_rhs(self) -> None:
        _check("SELECT 5 & 3.9")

    def test_shl_float_shift_count(self) -> None:
        # Shift count truncates too: 1 << 4.7 is 1 << 4 = 16.
        _check("SELECT 1 << 4.7")


# ---------------------------------------------------------------------------
# NULL propagation — any NULL operand produces NULL.
# ---------------------------------------------------------------------------


class TestNullPropagation:
    def test_and_null_lhs(self) -> None:
        _check("SELECT NULL & 3")

    def test_or_null_rhs(self) -> None:
        _check("SELECT 3 | NULL")

    def test_shl_null(self) -> None:
        _check("SELECT NULL << 2")

    def test_not_null(self) -> None:
        _check("SELECT ~NULL")


# ---------------------------------------------------------------------------
# Use in WHERE clauses — verifies the boolean coercion path.
# ---------------------------------------------------------------------------


class TestInWhere:
    setup = [
        "CREATE TABLE flags(id INTEGER, mask INTEGER)",
        "INSERT INTO flags VALUES (1, 1), (2, 2), (3, 4), (4, 7)",
    ]

    def test_where_and_mask(self) -> None:
        # All rows where bit 0 is set.
        _check_with_table(self.setup, "SELECT id FROM flags WHERE mask & 1 ORDER BY id")

    def test_where_or_set(self) -> None:
        _check_with_table(
            self.setup,
            "SELECT id, mask | 8 FROM flags WHERE mask | 8 = 9 ORDER BY id",
        )

    def test_where_shift(self) -> None:
        _check_with_table(
            self.setup, "SELECT id FROM flags WHERE mask = 1 << 1 ORDER BY id"
        )
