"""Arithmetic, comparison, three-valued logic, LIKE matching."""

from __future__ import annotations

import pytest
from sql_codegen import BinaryOpCode, UnaryOpCode

from sql_vm.errors import DivisionByZero, TypeMismatch
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

    def test_div_by_zero(self) -> None:
        with pytest.raises(DivisionByZero):
            apply_binary(BinaryOpCode.DIV, 5, 0)

    def test_mod_by_zero(self) -> None:
        with pytest.raises(DivisionByZero):
            apply_binary(BinaryOpCode.MOD, 5, 0)

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

    def test_and_rejects_non_boolean(self) -> None:
        # With no FALSE short-circuit, AND must validate the input types.
        with pytest.raises(TypeMismatch):
            apply_binary(BinaryOpCode.AND, 1, True)

    def test_or_rejects_non_boolean(self) -> None:
        # With no TRUE short-circuit, OR must validate the input types.
        with pytest.raises(TypeMismatch):
            apply_binary(BinaryOpCode.OR, 1, False)


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

    def test_not_int_raises(self) -> None:
        with pytest.raises(TypeMismatch):
            apply_unary(UnaryOpCode.NOT, 1)


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
