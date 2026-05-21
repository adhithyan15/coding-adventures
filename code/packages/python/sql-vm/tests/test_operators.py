"""Arithmetic, comparison, three-valued logic, LIKE matching."""

from __future__ import annotations

import pytest
from sql_codegen import BinaryOpCode, UnaryOpCode

from sql_vm.errors import TypeMismatch
from sql_vm.operators import apply_binary, apply_unary, like_match


class TestArithmetic:
    def test_add_ints(self) -> None:
        assert apply_binary(BinaryOpCode.ADD, 2, 3) == 5

    def test_add_mixed_numeric(self) -> None:
        assert apply_binary(BinaryOpCode.ADD, 2, 1.5) == 3.5

    def test_sub_mul_mod(self) -> None:
        assert apply_binary(BinaryOpCode.SUB, 10, 4) == 6
        assert apply_binary(BinaryOpCode.MUL, 3, 4) == 12
        assert apply_binary(BinaryOpCode.MOD, 10, 3) == 1

    def test_int_div_truncates_toward_zero(self) -> None:
        assert apply_binary(BinaryOpCode.DIV, 7, 2) == 3
        assert apply_binary(BinaryOpCode.DIV, -7, 2) == -3  # truncate, not floor
        assert apply_binary(BinaryOpCode.DIV, 7, -2) == -3

    def test_float_div(self) -> None:
        result = apply_binary(BinaryOpCode.DIV, 5.0, 2)
        assert result == 2.5

    def test_div_by_zero_returns_null(self) -> None:
        # SQLite returns NULL for x / 0 rather than raising.  Mini-sqlite
        # used to raise DivisionByZero (surfaced as OperationalError), so
        # any application that expected SQLite's NULL-on-error policy was
        # crashing.  Now both DIV and MOD by zero produce NULL.
        assert apply_binary(BinaryOpCode.DIV, 5, 0) is None
        assert apply_binary(BinaryOpCode.DIV, 5.0, 0) is None
        assert apply_binary(BinaryOpCode.DIV, 5, 0.0) is None

    def test_mod_by_zero(self) -> None:
        # Both DIV and MOD return NULL for division by zero.
        assert apply_binary(BinaryOpCode.MOD, 5, 0) is None

    def test_mod_sign_follows_dividend(self) -> None:
        # SQLite's % is C-style fmod: result sign matches the *dividend*.
        # Python's % follows the divisor, which gives the wrong answer
        # for negative operands (e.g. ``-7 % 3 == 2`` in Python but SQLite
        # produces -1).  Pin the corrected C-style behaviour.
        assert apply_binary(BinaryOpCode.MOD, -7, 3) == -1
        assert apply_binary(BinaryOpCode.MOD, 7, -3) == 1
        assert apply_binary(BinaryOpCode.MOD, -7, -3) == -1
        assert apply_binary(BinaryOpCode.MOD, 7, 3) == 1

    def test_arithmetic_with_non_numeric_raises(self) -> None:
        with pytest.raises(TypeMismatch):
            apply_binary(BinaryOpCode.ADD, "a", 1)

    def test_null_propagates_through_arithmetic(self) -> None:
        assert apply_binary(BinaryOpCode.ADD, None, 5) is None
        assert apply_binary(BinaryOpCode.MUL, 3, None) is None


class TestComparison:
    def test_eq_ne(self) -> None:
        assert apply_binary(BinaryOpCode.EQ, 1, 1) is True
        assert apply_binary(BinaryOpCode.NEQ, 1, 2) is True

    def test_lt_gt(self) -> None:
        assert apply_binary(BinaryOpCode.LT, 1, 2) is True
        assert apply_binary(BinaryOpCode.GT, 2, 1) is True
        assert apply_binary(BinaryOpCode.LTE, 2, 2) is True
        assert apply_binary(BinaryOpCode.GTE, 2, 2) is True

    def test_string_comparison_lex(self) -> None:
        assert apply_binary(BinaryOpCode.LT, "apple", "banana") is True

    def test_null_propagates_through_comparison(self) -> None:
        assert apply_binary(BinaryOpCode.EQ, None, 1) is None
        assert apply_binary(BinaryOpCode.LT, 1, None) is None

    def test_bool_vs_int_raises(self) -> None:
        # SQL BOOLEAN is not comparable to INTEGER in our model.
        with pytest.raises(TypeMismatch):
            apply_binary(BinaryOpCode.EQ, True, 1)

    def test_string_vs_int_raises(self) -> None:
        with pytest.raises(TypeMismatch):
            apply_binary(BinaryOpCode.LT, "a", 1)


class TestThreeValuedLogic:
    def test_and_truth_table(self) -> None:
        assert apply_binary(BinaryOpCode.AND, True, True) is True
        assert apply_binary(BinaryOpCode.AND, True, False) is False
        assert apply_binary(BinaryOpCode.AND, False, False) is False
        assert apply_binary(BinaryOpCode.AND, None, True) is None
        assert apply_binary(BinaryOpCode.AND, None, False) is False
        assert apply_binary(BinaryOpCode.AND, False, None) is False
        assert apply_binary(BinaryOpCode.AND, None, None) is None

    def test_or_truth_table(self) -> None:
        assert apply_binary(BinaryOpCode.OR, True, True) is True
        assert apply_binary(BinaryOpCode.OR, True, False) is True
        assert apply_binary(BinaryOpCode.OR, False, False) is False
        assert apply_binary(BinaryOpCode.OR, None, True) is True
        assert apply_binary(BinaryOpCode.OR, None, False) is None
        assert apply_binary(BinaryOpCode.OR, True, None) is True
        assert apply_binary(BinaryOpCode.OR, None, None) is None

    def test_and_coerces_integer_to_truth(self) -> None:
        # SQLite's AND coerces numeric values to truth: 1 is TRUE, 0 is
        # FALSE.  The previous behaviour rejected ``apply_binary(AND, 1,
        # True)`` with TypeMismatch — that was the bug that allowed
        # ``SELECT 1 AND 0`` to fold to NULL in the optimizer; this test
        # now pins the correct semantics.
        assert apply_binary(BinaryOpCode.AND, 1, True) is True
        assert apply_binary(BinaryOpCode.AND, 1, 0) is False
        assert apply_binary(BinaryOpCode.AND, 1, 1) is True
        assert apply_binary(BinaryOpCode.AND, 0, 1) is False

    def test_or_coerces_integer_to_truth(self) -> None:
        # See ``test_and_coerces_integer_to_truth`` for the rationale.
        assert apply_binary(BinaryOpCode.OR, 1, False) is True
        assert apply_binary(BinaryOpCode.OR, 0, 0) is False
        assert apply_binary(BinaryOpCode.OR, 0, 1) is True

    def test_and_or_reject_strings(self) -> None:
        # Strings have no defined SQL truth value here — they should still
        # raise TypeMismatch.  Only numeric/boolean operands are coerced.
        with pytest.raises(TypeMismatch):
            apply_binary(BinaryOpCode.AND, "abc", True)
        with pytest.raises(TypeMismatch):
            apply_binary(BinaryOpCode.OR, "abc", False)


class TestConcat:
    def test_concat_text(self) -> None:
        assert apply_binary(BinaryOpCode.CONCAT, "hello ", "world") == "hello world"

    def test_concat_null(self) -> None:
        assert apply_binary(BinaryOpCode.CONCAT, None, "x") is None

    def test_concat_non_text_raises(self) -> None:
        with pytest.raises(TypeMismatch):
            apply_binary(BinaryOpCode.CONCAT, 1, 2)


class TestUnary:
    def test_neg_int(self) -> None:
        assert apply_unary(UnaryOpCode.NEG, 5) == -5

    def test_neg_float(self) -> None:
        assert apply_unary(UnaryOpCode.NEG, 1.5) == -1.5

    def test_neg_null(self) -> None:
        assert apply_unary(UnaryOpCode.NEG, None) is None

    def test_neg_text_raises(self) -> None:
        with pytest.raises(TypeMismatch):
            apply_unary(UnaryOpCode.NEG, "abc")

    def test_not_bool(self) -> None:
        assert apply_unary(UnaryOpCode.NOT, True) is False
        assert apply_unary(UnaryOpCode.NOT, False) is True

    def test_not_null(self) -> None:
        assert apply_unary(UnaryOpCode.NOT, None) is None

    def test_not_int_coerces_to_truth(self) -> None:
        # SQLite has no separate BOOLEAN class; ``NOT 0`` is ``1`` and
        # ``NOT 5`` is ``0``.  The previous behaviour raised
        # TypeMismatch for any non-bool input — same family of bug as the
        # AND/OR integer-truthiness fix.
        assert apply_unary(UnaryOpCode.NOT, 1) is False
        assert apply_unary(UnaryOpCode.NOT, 0) is True
        assert apply_unary(UnaryOpCode.NOT, 5) is False
        assert apply_unary(UnaryOpCode.NOT, -1) is False
        assert apply_unary(UnaryOpCode.NOT, 1.5) is False
        assert apply_unary(UnaryOpCode.NOT, 0.0) is True

    def test_not_string_still_raises(self) -> None:
        # Strings have no defined truth value here — keep the TypeMismatch.
        with pytest.raises(TypeMismatch):
            apply_unary(UnaryOpCode.NOT, "abc")


class TestLike:
    def test_literal_match(self) -> None:
        assert like_match("hello", "hello") is True
        assert like_match("hello", "world") is False

    def test_percent_any(self) -> None:
        assert like_match("hello", "h%") is True
        assert like_match("hello", "%llo") is True
        assert like_match("hello", "%ll%") is True

    def test_underscore_single(self) -> None:
        assert like_match("cat", "c_t") is True
        assert like_match("cast", "c_t") is False

    def test_empty_patterns(self) -> None:
        assert like_match("", "") is True
        assert like_match("", "%") is True
        assert like_match("a", "") is False

    def test_case_insensitive(self) -> None:
        # SQL standard (and SQLite default): LIKE is case-insensitive for ASCII.
        assert like_match("Abc", "abc") is True
        assert like_match("ABC", "abc") is True
        assert like_match("abc", "ABC") is True
        assert like_match("Hello", "HELLO%") is True


# ---------------------------------------------------------------------------
# Bitwise operators — &, |, <<, >>, ~ with 64-bit two's-complement wrap.
# ---------------------------------------------------------------------------


class TestBitwise:
    """SQLite's bitwise ops on 64-bit signed integers.

    The interesting invariants beyond the obvious truth tables:
    * Floats truncate toward zero before the op runs.
    * Results wrap to 64-bit signed two's complement (``1 << 63`` is
      ``-2**63``, not ``+2**63``).
    * Shifts ≥ 64 bits saturate: SHL/SHR by 64+ of a non-negative value
      gives 0; SHR by 64+ of a negative value gives -1 (sign extension).
    * Negative shift counts flip direction: ``a << -k`` ≡ ``a >> k``.
    """

    def test_and_basic(self) -> None:
        assert apply_binary(BinaryOpCode.BIT_AND, 5, 3) == 1

    def test_or_basic(self) -> None:
        assert apply_binary(BinaryOpCode.BIT_OR, 5, 3) == 7

    def test_and_neg_one_is_identity(self) -> None:
        # -1 is all-bits-set in two's complement.
        assert apply_binary(BinaryOpCode.BIT_AND, 42, -1) == 42

    def test_or_neg_one_is_neg_one(self) -> None:
        assert apply_binary(BinaryOpCode.BIT_OR, 42, -1) == -1

    def test_shl_basic(self) -> None:
        assert apply_binary(BinaryOpCode.BIT_SHL, 1, 4) == 16

    def test_shl_wraps_to_negative(self) -> None:
        # 1 << 63 overflows the positive range; reinterpret as signed.
        assert apply_binary(BinaryOpCode.BIT_SHL, 1, 63) == -(2**63)

    def test_shl_saturates_at_64(self) -> None:
        assert apply_binary(BinaryOpCode.BIT_SHL, 12345, 64) == 0
        assert apply_binary(BinaryOpCode.BIT_SHL, 12345, 100) == 0

    def test_shl_negative_count_flips_to_shr(self) -> None:
        assert apply_binary(BinaryOpCode.BIT_SHL, 16, -2) == 4
        # Far-negative count: same saturation as forward shifts.
        assert apply_binary(BinaryOpCode.BIT_SHL, 16, -100) == 0
        assert apply_binary(BinaryOpCode.BIT_SHL, -16, -100) == -1

    def test_shr_basic(self) -> None:
        assert apply_binary(BinaryOpCode.BIT_SHR, 16, 2) == 4

    def test_shr_sign_extends(self) -> None:
        # Arithmetic shift: sign bit propagates.
        assert apply_binary(BinaryOpCode.BIT_SHR, -8, 1) == -4

    def test_shr_saturates_positive(self) -> None:
        assert apply_binary(BinaryOpCode.BIT_SHR, 12345, 64) == 0
        assert apply_binary(BinaryOpCode.BIT_SHR, 12345, 200) == 0

    def test_shr_saturates_negative(self) -> None:
        # Sign extension makes a 64-bit -1 fill the entire word.
        assert apply_binary(BinaryOpCode.BIT_SHR, -1, 64) == -1
        assert apply_binary(BinaryOpCode.BIT_SHR, -42, 200) == -1

    def test_shr_negative_count_flips_to_shl(self) -> None:
        assert apply_binary(BinaryOpCode.BIT_SHR, 4, -2) == 16
        assert apply_binary(BinaryOpCode.BIT_SHR, 4, -100) == 0

    def test_float_lhs_truncates(self) -> None:
        assert apply_binary(BinaryOpCode.BIT_AND, 5.9, 3) == 1

    def test_float_rhs_truncates(self) -> None:
        assert apply_binary(BinaryOpCode.BIT_AND, 5, 3.9) == 1

    def test_bool_coerces_to_int(self) -> None:
        assert apply_binary(BinaryOpCode.BIT_AND, True, 1) == 1
        assert apply_binary(BinaryOpCode.BIT_OR, False, 1) == 1

    def test_string_operand_raises(self) -> None:
        with pytest.raises(TypeMismatch):
            apply_binary(BinaryOpCode.BIT_AND, "foo", 1)
        with pytest.raises(TypeMismatch):
            apply_binary(BinaryOpCode.BIT_OR, 1, "foo")

    def test_not_basic(self) -> None:
        assert apply_unary(UnaryOpCode.BIT_NOT, 0) == -1
        assert apply_unary(UnaryOpCode.BIT_NOT, 5) == -6
        assert apply_unary(UnaryOpCode.BIT_NOT, -1) == 0

    def test_not_float_truncates(self) -> None:
        assert apply_unary(UnaryOpCode.BIT_NOT, 5.9) == -6

    def test_not_null_propagates(self) -> None:
        assert apply_unary(UnaryOpCode.BIT_NOT, None) is None

    def test_not_string_raises(self) -> None:
        with pytest.raises(TypeMismatch):
            apply_unary(UnaryOpCode.BIT_NOT, "foo")
