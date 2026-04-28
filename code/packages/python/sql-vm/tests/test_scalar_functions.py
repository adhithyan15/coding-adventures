"""
Tests for sql_vm.scalar_functions and CallScalar VM dispatch.
=============================================================

Coverage strategy
-----------------

Each test class maps to one logical category of built-in functions.  The
class names mirror the SQL category headings in ``scalar_functions.py``:

- ``TestRegistry``       — dispatch mechanics, UnsupportedFunction
- ``TestNullHandling``   — COALESCE, IFNULL, NULLIF, IIF
- ``TestTypeof``         — TYPEOF for every SqlValue kind
- ``TestCast``           — CAST to every supported affinity target
- ``TestNumeric``        — ABS, ROUND, CEIL, FLOOR, SIGN, MOD
- ``TestMathFunctions``  — SQRT, POW, LOG, LOG2, LOG10, EXP, PI, trig
- ``TestStringFunctions``— UPPER, LOWER, LENGTH, TRIM, SUBSTR, REPLACE, INSTR
- ``TestHexBlob``        — HEX, UNHEX, ZEROBLOB, RANDOMBLOB
- ``TestQuoteChar``      — QUOTE, CHAR, UNICODE
- ``TestSoundex``        — SOUNDEX
- ``TestPrintf``         — PRINTF / FORMAT
- ``TestRandom``         — RANDOM, RANDOMBLOB, LAST_INSERT_ROWID
- ``TestVmCallScalar``   — end-to-end execution through the VM dispatch loop

Each test verifies at minimum:
  1. The "happy path" return value.
  2. NULL propagation (where applicable).
  3. Edge cases (empty strings, zero, negative, out-of-domain).
"""

from __future__ import annotations

import math

import pytest

from sql_vm.errors import UnsupportedFunction, WrongNumberOfArguments
from sql_vm.scalar_functions import call

# ---------------------------------------------------------------------------
# Helper: call single function with positional args
# ---------------------------------------------------------------------------


def fn(name: str, *args: object) -> object:
    """Thin wrapper to call a registered scalar function by name."""
    return call(name, list(args))  # type: ignore[arg-type]


# ===========================================================================
# Registry mechanics
# ===========================================================================


class TestRegistry:
    def test_unknown_function_raises(self) -> None:
        with pytest.raises(UnsupportedFunction) as exc:
            fn("no_such_function", 1)
        assert exc.value.name == "no_such_function"
        assert "no_such_function" in str(exc.value)

    def test_unknown_function_error_str(self) -> None:
        err = UnsupportedFunction(name="my_fn")
        assert str(err) == "unknown scalar function: 'my_fn'"

    def test_wrong_arity_error_str(self) -> None:
        err = WrongNumberOfArguments(name="round", expected="1 or 2", got=3)
        assert "round" in str(err)
        assert "1 or 2" in str(err)
        assert "3" in str(err)

    def test_function_names_are_case_insensitive_at_call_level(self) -> None:
        # The registry stores lower-cased names; callers that lower-case before
        # dispatch get the right function.
        assert call("abs", [-3]) == 3  # lower
        # Upper-case lookup does NOT match — callers must lower-case.
        with pytest.raises(UnsupportedFunction):
            call("ABS", [-3])

    def test_round_wrong_arity(self) -> None:
        with pytest.raises(WrongNumberOfArguments):
            fn("round", 3.14, 2, "extra")

    def test_trim_wrong_arity(self) -> None:
        with pytest.raises(WrongNumberOfArguments):
            fn("trim")

    def test_log_wrong_arity(self) -> None:
        with pytest.raises(WrongNumberOfArguments):
            fn("log")

    def test_printf_no_args_raises(self) -> None:
        with pytest.raises(WrongNumberOfArguments):
            fn("printf")


# ===========================================================================
# NULL-handling
# ===========================================================================


class TestNullHandling:
    # COALESCE -----------------------------------------------------------
    def test_coalesce_first_non_null(self) -> None:
        assert fn("coalesce", None, 2, 3) == 2

    def test_coalesce_all_null(self) -> None:
        assert fn("coalesce", None, None) is None

    def test_coalesce_single_value(self) -> None:
        assert fn("coalesce", 42) == 42

    def test_coalesce_first_arg_not_null(self) -> None:
        assert fn("coalesce", "hello", None) == "hello"

    def test_coalesce_preserves_zero_as_truthy(self) -> None:
        # 0 is NOT NULL, so COALESCE(0, 5) → 0
        assert fn("coalesce", 0, 5) == 0

    # IFNULL ------------------------------------------------------------
    def test_ifnull_returns_x_when_not_null(self) -> None:
        assert fn("ifnull", 7, 99) == 7

    def test_ifnull_returns_y_when_null(self) -> None:
        assert fn("ifnull", None, 99) == 99

    def test_ifnull_both_null(self) -> None:
        assert fn("ifnull", None, None) is None

    # NULLIF ------------------------------------------------------------
    def test_nullif_equal_returns_null(self) -> None:
        assert fn("nullif", 5, 5) is None

    def test_nullif_not_equal_returns_x(self) -> None:
        assert fn("nullif", 5, 6) == 5

    def test_nullif_both_null(self) -> None:
        assert fn("nullif", None, None) is None

    def test_nullif_x_null_y_non_null(self) -> None:
        # x IS NULL; x != y (NULL != 5), so return x = NULL
        assert fn("nullif", None, 5) is None

    # IIF ---------------------------------------------------------------
    def test_iif_true_condition(self) -> None:
        assert fn("iif", True, "yes", "no") == "yes"

    def test_iif_false_condition(self) -> None:
        assert fn("iif", False, "yes", "no") == "no"

    def test_iif_null_condition_is_falsy(self) -> None:
        assert fn("iif", None, "yes", "no") == "no"

    def test_iif_zero_condition_is_falsy(self) -> None:
        assert fn("iif", 0, "yes", "no") == "no"

    def test_iif_nonzero_condition_is_truthy(self) -> None:
        assert fn("iif", 1, "yes", "no") == "yes"


# ===========================================================================
# TYPEOF
# ===========================================================================


class TestTypeof:
    def test_null(self) -> None:
        assert fn("typeof", None) == "null"

    def test_integer(self) -> None:
        assert fn("typeof", 42) == "integer"

    def test_negative_integer(self) -> None:
        assert fn("typeof", -1) == "integer"

    def test_real(self) -> None:
        assert fn("typeof", 3.14) == "real"

    def test_text(self) -> None:
        assert fn("typeof", "hello") == "text"

    def test_blob(self) -> None:
        assert fn("typeof", b"\x00\xFF") == "blob"

    def test_bytearray_is_blob(self) -> None:
        assert fn("typeof", bytearray(b"abc")) == "blob"

    def test_bool_is_integer(self) -> None:
        # SQLite treats TRUE/FALSE as integer 1/0.
        assert fn("typeof", True) == "integer"
        assert fn("typeof", False) == "integer"

    def test_zero_float(self) -> None:
        assert fn("typeof", 0.0) == "real"


# ===========================================================================
# CAST
# ===========================================================================


class TestCast:
    def test_cast_null_returns_null(self) -> None:
        assert fn("cast", None, "integer") is None

    def test_cast_float_to_int(self) -> None:
        assert fn("cast", 3.9, "integer") == 3

    def test_cast_string_to_int(self) -> None:
        assert fn("cast", "42", "integer") == 42

    def test_cast_string_float_to_int(self) -> None:
        assert fn("cast", "3.7", "integer") == 3

    def test_cast_int_to_real(self) -> None:
        result = fn("cast", 5, "real")
        assert result == 5.0
        assert isinstance(result, float)

    def test_cast_string_to_real(self) -> None:
        assert fn("cast", "3.14", "real") == pytest.approx(3.14)

    def test_cast_int_to_text(self) -> None:
        assert fn("cast", 42, "text") == "42"

    def test_cast_float_to_text(self) -> None:
        assert fn("cast", 3.14, "text") == "3.14"

    def test_cast_blob_to_text_is_hex(self) -> None:
        # bytes → hex string per our CAST implementation.
        result = fn("cast", b"\xde\xad", "text")
        assert isinstance(result, str)
        assert result.lower() == "dead"

    def test_cast_string_to_blob(self) -> None:
        result = fn("cast", "hi", "blob")
        assert result == b"hi"

    def test_cast_int_to_blob(self) -> None:
        result = fn("cast", 255, "blob")
        assert isinstance(result, bytes)

    def test_cast_to_boolean(self) -> None:
        assert fn("cast", 1, "boolean") is True
        assert fn("cast", 0, "boolean") is False

    def test_cast_unknown_type_returns_x(self) -> None:
        # Graceful: unknown target → pass-through.
        assert fn("cast", 42, "unknowntype") == 42

    def test_cast_bool_to_int(self) -> None:
        assert fn("cast", True, "integer") == 1
        assert fn("cast", False, "integer") == 0

    def test_cast_varchar_alias(self) -> None:
        assert fn("cast", 10, "varchar") == "10"


# ===========================================================================
# Numeric functions
# ===========================================================================


class TestNumeric:
    # ABS ----------------------------------------------------------------
    def test_abs_positive(self) -> None:
        assert fn("abs", 5) == 5

    def test_abs_negative_int(self) -> None:
        assert fn("abs", -5) == 5

    def test_abs_negative_float(self) -> None:
        assert fn("abs", -3.14) == pytest.approx(3.14)

    def test_abs_zero(self) -> None:
        assert fn("abs", 0) == 0

    def test_abs_null(self) -> None:
        assert fn("abs", None) is None

    def test_abs_non_numeric_passthrough(self) -> None:
        # Non-numeric: pass through unchanged.
        assert fn("abs", "hello") == "hello"

    # ROUND --------------------------------------------------------------
    def test_round_no_precision(self) -> None:
        assert fn("round", 3.5) == 4.0

    def test_round_negative(self) -> None:
        assert fn("round", -3.5) == -4.0

    def test_round_with_precision(self) -> None:
        assert fn("round", 3.14159, 2) == pytest.approx(3.14)

    def test_round_null(self) -> None:
        assert fn("round", None) is None

    def test_round_non_numeric_passthrough(self) -> None:
        assert fn("round", "hi", 2) == "hi"

    # CEIL / CEILING ---------------------------------------------------
    def test_ceil_positive(self) -> None:
        assert fn("ceil", 3.1) == 4.0

    def test_ceil_negative(self) -> None:
        assert fn("ceil", -3.9) == -3.0

    def test_ceiling_alias(self) -> None:
        assert fn("ceiling", 2.5) == 3.0

    def test_ceil_null(self) -> None:
        assert fn("ceil", None) is None

    def test_ceil_integer(self) -> None:
        assert fn("ceil", 5) == 5.0

    # FLOOR --------------------------------------------------------------
    def test_floor_positive(self) -> None:
        assert fn("floor", 3.9) == 3.0

    def test_floor_negative(self) -> None:
        assert fn("floor", -3.1) == -4.0

    def test_floor_null(self) -> None:
        assert fn("floor", None) is None

    # SIGN ---------------------------------------------------------------
    def test_sign_positive(self) -> None:
        assert fn("sign", 10) == 1

    def test_sign_negative(self) -> None:
        assert fn("sign", -5) == -1

    def test_sign_zero(self) -> None:
        assert fn("sign", 0) == 0

    def test_sign_null(self) -> None:
        assert fn("sign", None) is None

    # MOD ----------------------------------------------------------------
    def test_mod_basic(self) -> None:
        assert fn("mod", 10, 3) == 1

    def test_mod_by_zero_returns_null(self) -> None:
        # SQLite: x % 0 → NULL (not an exception).
        assert fn("mod", 10, 0) is None

    def test_mod_null_propagation(self) -> None:
        assert fn("mod", None, 3) is None
        assert fn("mod", 10, None) is None

    def test_mod_float(self) -> None:
        result = fn("mod", 7.5, 2.5)
        assert result == pytest.approx(0.0)

    def test_mod_non_numeric_returns_null(self) -> None:
        assert fn("mod", "a", 3) is None


# ===========================================================================
# Math functions
# ===========================================================================


class TestMathFunctions:
    def test_sqrt_positive(self) -> None:
        assert fn("sqrt", 4) == pytest.approx(2.0)

    def test_sqrt_null(self) -> None:
        assert fn("sqrt", None) is None

    def test_sqrt_negative_returns_null(self) -> None:
        assert fn("sqrt", -1) is None

    def test_sqrt_non_numeric_returns_null(self) -> None:
        assert fn("sqrt", "abc") is None

    # POW / POWER -------------------------------------------------------
    def test_pow_basic(self) -> None:
        assert fn("pow", 2, 10) == pytest.approx(1024.0)

    def test_power_alias(self) -> None:
        assert fn("power", 3, 3) == pytest.approx(27.0)

    def test_pow_null(self) -> None:
        assert fn("pow", None, 2) is None
        assert fn("pow", 2, None) is None

    def test_pow_zero_base(self) -> None:
        assert fn("pow", 0, 5) == pytest.approx(0.0)

    # LOG / LN ----------------------------------------------------------
    def test_log_natural(self) -> None:
        result = fn("log", math.e)
        assert result == pytest.approx(1.0)

    def test_ln_alias(self) -> None:
        result = fn("ln", 1)
        assert result == pytest.approx(0.0)

    def test_log_base_2(self) -> None:
        # LOG(B, x) → log base B of x
        result = fn("log", 2, 8)
        assert result == pytest.approx(3.0)

    def test_log_null(self) -> None:
        assert fn("log", None) is None

    def test_log_non_positive_returns_null(self) -> None:
        assert fn("log", 0) is None
        assert fn("log", -1) is None

    # LOG2 / LOG10 / EXP ------------------------------------------------
    def test_log2(self) -> None:
        assert fn("log2", 8) == pytest.approx(3.0)

    def test_log10(self) -> None:
        assert fn("log10", 1000) == pytest.approx(3.0)

    def test_exp_zero(self) -> None:
        assert fn("exp", 0) == pytest.approx(1.0)

    def test_exp_one(self) -> None:
        assert fn("exp", 1) == pytest.approx(math.e)

    def test_exp_null(self) -> None:
        assert fn("exp", None) is None

    # PI ----------------------------------------------------------------
    def test_pi(self) -> None:
        assert fn("pi") == pytest.approx(math.pi)

    # Trigonometric -----------------------------------------------------
    def test_sin_zero(self) -> None:
        assert fn("sin", 0) == pytest.approx(0.0)

    def test_sin_half_pi(self) -> None:
        assert fn("sin", math.pi / 2) == pytest.approx(1.0)

    def test_cos_zero(self) -> None:
        assert fn("cos", 0) == pytest.approx(1.0)

    def test_tan_zero(self) -> None:
        assert fn("tan", 0) == pytest.approx(0.0)

    def test_asin_one(self) -> None:
        assert fn("asin", 1) == pytest.approx(math.pi / 2)

    def test_asin_out_of_domain(self) -> None:
        assert fn("asin", 2) is None

    def test_acos_one(self) -> None:
        assert fn("acos", 1) == pytest.approx(0.0)

    def test_atan_one(self) -> None:
        assert fn("atan", 1) == pytest.approx(math.pi / 4)

    def test_atan_two_args(self) -> None:
        result = fn("atan", 1, 1)
        assert result == pytest.approx(math.pi / 4)

    def test_atan2(self) -> None:
        assert fn("atan2", 1, 1) == pytest.approx(math.pi / 4)

    def test_degrees(self) -> None:
        assert fn("degrees", math.pi) == pytest.approx(180.0)

    def test_radians(self) -> None:
        assert fn("radians", 180) == pytest.approx(math.pi)

    def test_trig_null_propagation(self) -> None:
        for name in ("sin", "cos", "tan", "asin", "acos", "degrees", "radians",
                     "exp", "sqrt", "log2", "log10"):
            assert fn(name, None) is None, f"{name}(NULL) should be NULL"


# ===========================================================================
# String functions
# ===========================================================================


class TestStringFunctions:
    # UPPER / LOWER -----------------------------------------------------
    def test_upper(self) -> None:
        assert fn("upper", "hello") == "HELLO"

    def test_upper_null(self) -> None:
        assert fn("upper", None) is None

    def test_lower(self) -> None:
        assert fn("lower", "HELLO") == "hello"

    def test_lower_null(self) -> None:
        assert fn("lower", None) is None

    def test_upper_non_string_passthrough(self) -> None:
        assert fn("upper", 42) == 42

    # LENGTH / LEN ------------------------------------------------------
    def test_length_string(self) -> None:
        assert fn("length", "hello") == 5

    def test_length_empty(self) -> None:
        assert fn("length", "") == 0

    def test_length_null(self) -> None:
        assert fn("length", None) is None

    def test_length_blob(self) -> None:
        assert fn("length", b"\x00\xFF\xAB") == 3

    def test_length_integer(self) -> None:
        # LENGTH(42) → 2 (length of "42")
        assert fn("length", 42) == 2

    def test_len_alias(self) -> None:
        assert fn("len", "abc") == 3

    # TRIM / LTRIM / RTRIM ----------------------------------------------
    def test_trim_whitespace(self) -> None:
        assert fn("trim", "  hello  ") == "hello"

    def test_trim_custom_chars(self) -> None:
        assert fn("trim", "xxhelloxx", "x") == "hello"

    def test_trim_null(self) -> None:
        assert fn("trim", None) is None

    def test_ltrim_whitespace(self) -> None:
        assert fn("ltrim", "  hello  ") == "hello  "

    def test_rtrim_whitespace(self) -> None:
        assert fn("rtrim", "  hello  ") == "  hello"

    def test_ltrim_custom(self) -> None:
        assert fn("ltrim", "---abc---", "-") == "abc---"

    def test_rtrim_custom(self) -> None:
        assert fn("rtrim", "---abc---", "-") == "---abc"

    # SUBSTR / SUBSTRING ------------------------------------------------
    def test_substr_basic(self) -> None:
        assert fn("substr", "hello", 2) == "ello"

    def test_substr_with_length(self) -> None:
        assert fn("substr", "hello", 2, 3) == "ell"

    def test_substr_negative_start(self) -> None:
        assert fn("substr", "hello", -3) == "llo"

    def test_substr_zero_length(self) -> None:
        assert fn("substr", "hello", 2, 0) == ""

    def test_substr_null(self) -> None:
        assert fn("substr", None, 1) is None

    def test_substring_alias(self) -> None:
        assert fn("substring", "hello", 1, 3) == "hel"

    # REPLACE -----------------------------------------------------------
    def test_replace_basic(self) -> None:
        assert fn("replace", "hello world", "world", "SQL") == "hello SQL"

    def test_replace_multiple_occurrences(self) -> None:
        assert fn("replace", "aaa", "a", "bb") == "bbbbbb"

    def test_replace_null(self) -> None:
        assert fn("replace", None, "a", "b") is None
        assert fn("replace", "x", None, "b") is None

    # INSTR -------------------------------------------------------------
    def test_instr_found(self) -> None:
        assert fn("instr", "hello", "ll") == 3

    def test_instr_not_found(self) -> None:
        assert fn("instr", "hello", "xyz") == 0

    def test_instr_empty_needle(self) -> None:
        assert fn("instr", "hello", "") == 1

    def test_instr_null(self) -> None:
        assert fn("instr", None, "x") is None
        assert fn("instr", "x", None) is None

    def test_instr_blob(self) -> None:
        assert fn("instr", b"\x01\x02\x03", b"\x02") == 2


# ===========================================================================
# HEX, UNHEX, ZEROBLOB, RANDOMBLOB
# ===========================================================================


class TestHexBlob:
    def test_hex_blob(self) -> None:
        assert fn("hex", b"\xde\xad\xbe\xef") == "DEADBEEF"

    def test_hex_text(self) -> None:
        # "AB" → UTF-8 bytes 0x41 0x42
        assert fn("hex", "AB") == "4142"

    def test_hex_null(self) -> None:
        assert fn("hex", None) is None

    def test_hex_integer(self) -> None:
        result = fn("hex", 255)
        # big-endian 8-byte encoding of 255 = 0x00000000000000FF
        assert isinstance(result, str)
        assert result.endswith("FF")

    def test_unhex_basic(self) -> None:
        assert fn("unhex", "DEADBEEF") == b"\xde\xad\xbe\xef"

    def test_unhex_lowercase(self) -> None:
        assert fn("unhex", "deadbeef") == b"\xde\xad\xbe\xef"

    def test_unhex_with_ignore_chars(self) -> None:
        assert fn("unhex", "DE AD", " ") == b"\xde\xad"

    def test_unhex_null(self) -> None:
        assert fn("unhex", None) is None

    def test_unhex_malformed_returns_null(self) -> None:
        assert fn("unhex", "ZZ") is None

    def test_zeroblob_basic(self) -> None:
        assert fn("zeroblob", 4) == b"\x00\x00\x00\x00"

    def test_zeroblob_zero(self) -> None:
        assert fn("zeroblob", 0) == b""

    def test_zeroblob_null(self) -> None:
        assert fn("zeroblob", None) is None

    def test_randomblob_length(self) -> None:
        result = fn("randomblob", 8)
        assert isinstance(result, bytes)
        assert len(result) == 8

    def test_randomblob_null(self) -> None:
        assert fn("randomblob", None) is None

    def test_randomblob_nonpositive(self) -> None:
        assert fn("randomblob", 0) is None
        assert fn("randomblob", -1) is None


# ===========================================================================
# QUOTE, CHAR, UNICODE
# ===========================================================================


class TestQuoteChar:
    def test_quote_text(self) -> None:
        assert fn("quote", "hello") == "'hello'"

    def test_quote_text_with_single_quote(self) -> None:
        assert fn("quote", "it's") == "'it''s'"

    def test_quote_null(self) -> None:
        assert fn("quote", None) == "NULL"

    def test_quote_integer(self) -> None:
        assert fn("quote", 42) == "42"

    def test_quote_blob(self) -> None:
        result = fn("quote", b"\xde\xad")
        assert result == "X'DEAD'"

    def test_char_basic(self) -> None:
        assert fn("char", 65, 66, 67) == "ABC"

    def test_char_hello(self) -> None:
        assert fn("char", 72, 101, 108, 108, 111) == "Hello"

    def test_char_null_propagation(self) -> None:
        assert fn("char", 65, None, 67) is None

    def test_unicode_first_char(self) -> None:
        assert fn("unicode", "A") == 65

    def test_unicode_multi_char(self) -> None:
        # Returns code point of FIRST character only.
        assert fn("unicode", "hello") == 104

    def test_unicode_empty(self) -> None:
        assert fn("unicode", "") is None

    def test_unicode_null(self) -> None:
        assert fn("unicode", None) is None

    def test_unicode_blob(self) -> None:
        # First byte of blob
        assert fn("unicode", b"\x41\x42") == 0x41


# ===========================================================================
# SOUNDEX
# ===========================================================================


class TestSoundex:
    def test_robert(self) -> None:
        assert fn("soundex", "Robert") == "R163"

    def test_rupert(self) -> None:
        # Robert and Rupert have the same Soundex code.
        assert fn("soundex", "Rupert") == "R163"

    def test_null_returns_placeholder(self) -> None:
        assert fn("soundex", None) == "?000"

    def test_empty_string(self) -> None:
        assert fn("soundex", "") == "?000"

    def test_single_letter(self) -> None:
        result = fn("soundex", "A")
        assert isinstance(result, str)
        assert len(result) == 4

    def test_all_vowels(self) -> None:
        # Names like "AEIO" have no consonants → pad with zeros.
        result = fn("soundex", "AEIO")
        assert isinstance(result, str)
        assert len(result) == 4
        assert result[1:] == "000"

    def test_numbers_stripped(self) -> None:
        # Non-alpha characters are stripped before coding.
        result = fn("soundex", "R123obert")
        assert result == "R163"


# ===========================================================================
# PRINTF / FORMAT
# ===========================================================================


class TestPrintf:
    def test_hello_world(self) -> None:
        assert fn("printf", "Hello %s!", "world") == "Hello world!"

    def test_integer_format(self) -> None:
        assert fn("printf", "%d + %d = %d", 1, 2, 3) == "1 + 2 = 3"

    def test_float_precision(self) -> None:
        assert fn("printf", "%.2f", 3.14159) == "3.14"

    def test_sql_escape_q(self) -> None:
        # %q wraps in single quotes and doubles internal quotes.
        assert fn("printf", "%q", "it's") == "'it''s'"

    def test_sql_escape_Q_null(self) -> None:
        # %Q with NULL → the literal string "NULL".
        assert fn("printf", "%Q", None) == "NULL"

    def test_sql_escape_Q_non_null(self) -> None:
        assert fn("printf", "%Q", "hello") == "'hello'"

    def test_percent_literal(self) -> None:
        assert fn("printf", "100%%") == "100%"

    def test_format_alias(self) -> None:
        assert fn("format", "x=%d", 7) == "x=7"

    def test_null_format_returns_null(self) -> None:
        assert fn("printf", None, 1, 2) is None

    def test_no_format_args(self) -> None:
        assert fn("printf", "literal") == "literal"

    def test_width_padding(self) -> None:
        # Right-pad with spaces for %-10s
        result = fn("printf", "%-10s|", "hi")
        assert result == "hi        |"

    def test_hex_integer_format(self) -> None:
        assert fn("printf", "%x", 255) == "ff"
        assert fn("printf", "%X", 255) == "FF"

    def test_octal_format(self) -> None:
        assert fn("printf", "%o", 8) == "10"

    def test_scientific_notation(self) -> None:
        result = fn("printf", "%.2e", 12345.0)
        assert "e" in result.lower()


# ===========================================================================
# RANDOM, LAST_INSERT_ROWID
# ===========================================================================


class TestRandom:
    def test_random_returns_integer(self) -> None:
        result = fn("random")
        assert isinstance(result, int)

    def test_random_in_64bit_range(self) -> None:
        result = fn("random")
        assert -(2**63) <= result < 2**63

    def test_random_calls_produce_different_values(self) -> None:
        # With overwhelming probability two 64-bit random calls differ.
        values = {fn("random") for _ in range(10)}
        assert len(values) > 1

    def test_last_insert_rowid_returns_null(self) -> None:
        # Placeholder — not yet wired to backend.
        assert fn("last_insert_rowid") is None


# ===========================================================================
# VM integration: CallScalar dispatches correctly end-to-end
# ===========================================================================


class TestVmCallScalar:
    """Execute micro-programs that contain CallScalar instructions."""

    def _run(self, instructions: list) -> object:
        """Run a Program against an empty InMemoryBackend and return the
        first value on the stack (or the first result row)."""
        from sql_backend.in_memory import InMemoryBackend
        from sql_codegen import (
            Program,
        )

        from sql_vm import execute

        program = Program(
            instructions=tuple(instructions),
            labels={},
            result_schema=("result",),
        )
        backend = InMemoryBackend()
        result = execute(program, backend)
        return result.rows[0][0] if result.rows else None

    def _simple(self, instructions: list) -> object:
        """Run instructions that produce a single result row."""
        return self._run(
            [
                *instructions,
                # SetResultSchema THEN BeginRow/EmitColumn/EmitRow is the
                # standard codegen pattern.
            ]
        )

    def test_abs_via_vm(self) -> None:
        from sql_backend.in_memory import InMemoryBackend
        from sql_codegen import (
            BeginRow,
            CallScalar,
            EmitColumn,
            EmitRow,
            Halt,
            LoadConst,
            Program,
            SetResultSchema,
        )

        from sql_vm import execute

        prog = Program(
            instructions=(
                SetResultSchema(columns=("result",)),
                BeginRow(),
                LoadConst(value=-7),
                CallScalar(func="abs", n_args=1),
                EmitColumn(name="result"),
                EmitRow(),
                Halt(),
            ),
            labels={},
            result_schema=("result",),
        )
        result = execute(prog, InMemoryBackend())
        assert result.rows == ((7,),)

    def test_coalesce_via_vm(self) -> None:
        from sql_backend.in_memory import InMemoryBackend
        from sql_codegen import (
            BeginRow,
            CallScalar,
            EmitColumn,
            EmitRow,
            Halt,
            LoadConst,
            Program,
            SetResultSchema,
        )

        from sql_vm import execute

        prog = Program(
            instructions=(
                SetResultSchema(columns=("result",)),
                BeginRow(),
                LoadConst(value=None),
                LoadConst(value=42),
                CallScalar(func="coalesce", n_args=2),
                EmitColumn(name="result"),
                EmitRow(),
                Halt(),
            ),
            labels={},
            result_schema=("result",),
        )
        result = execute(prog, InMemoryBackend())
        assert result.rows == ((42,),)

    def test_upper_via_vm(self) -> None:
        from sql_backend.in_memory import InMemoryBackend
        from sql_codegen import (
            BeginRow,
            CallScalar,
            EmitColumn,
            EmitRow,
            Halt,
            LoadConst,
            Program,
            SetResultSchema,
        )

        from sql_vm import execute

        prog = Program(
            instructions=(
                SetResultSchema(columns=("result",)),
                BeginRow(),
                LoadConst(value="hello"),
                CallScalar(func="upper", n_args=1),
                EmitColumn(name="result"),
                EmitRow(),
                Halt(),
            ),
            labels={},
            result_schema=("result",),
        )
        result = execute(prog, InMemoryBackend())
        assert result.rows == (("HELLO",),)

    def test_unsupported_function_propagates(self) -> None:
        from sql_backend.in_memory import InMemoryBackend
        from sql_codegen import (
            CallScalar,
            Halt,
            LoadConst,
            Program,
        )

        from sql_vm import execute
        from sql_vm.errors import UnsupportedFunction

        prog = Program(
            instructions=(
                LoadConst(value=1),
                CallScalar(func="nonexistent_fn", n_args=1),
                Halt(),
            ),
            labels={},
            result_schema=(),
        )
        with pytest.raises(UnsupportedFunction):
            execute(prog, InMemoryBackend())

    def test_printf_via_vm(self) -> None:
        from sql_backend.in_memory import InMemoryBackend
        from sql_codegen import (
            BeginRow,
            CallScalar,
            EmitColumn,
            EmitRow,
            Halt,
            LoadConst,
            Program,
            SetResultSchema,
        )

        from sql_vm import execute

        prog = Program(
            instructions=(
                SetResultSchema(columns=("result",)),
                BeginRow(),
                LoadConst(value="value=%d"),
                LoadConst(value=99),
                CallScalar(func="printf", n_args=2),
                EmitColumn(name="result"),
                EmitRow(),
                Halt(),
            ),
            labels={},
            result_schema=("result",),
        )
        result = execute(prog, InMemoryBackend())
        assert result.rows == (("value=99",),)

    def test_null_propagation_through_vm(self) -> None:
        """UPPER(NULL) should push NULL, not raise."""
        from sql_backend.in_memory import InMemoryBackend
        from sql_codegen import (
            BeginRow,
            CallScalar,
            EmitColumn,
            EmitRow,
            Halt,
            LoadConst,
            Program,
            SetResultSchema,
        )

        from sql_vm import execute

        prog = Program(
            instructions=(
                SetResultSchema(columns=("result",)),
                BeginRow(),
                LoadConst(value=None),
                CallScalar(func="upper", n_args=1),
                EmitColumn(name="result"),
                EmitRow(),
                Halt(),
            ),
            labels={},
            result_schema=("result",),
        )
        result = execute(prog, InMemoryBackend())
        assert result.rows == ((None,),)


# ===========================================================================
# Scalar MAX / MIN (two-argument forms)
# ===========================================================================


class TestScalarMinMax:
    """Scalar MAX(a, b) and MIN(a, b) — two-argument forms."""

    def test_max_integers(self) -> None:
        assert fn("max", 3, 5) == 5

    def test_max_integers_reversed(self) -> None:
        assert fn("max", 5, 3) == 5

    def test_max_equal(self) -> None:
        assert fn("max", 4, 4) == 4

    def test_min_integers(self) -> None:
        assert fn("min", 3, 5) == 3

    def test_min_equal(self) -> None:
        assert fn("min", 7, 7) == 7

    def test_max_floats(self) -> None:
        assert fn("max", 1.5, 2.5) == 2.5

    def test_min_floats(self) -> None:
        assert fn("min", 1.5, 2.5) == 1.5

    def test_max_strings(self) -> None:
        assert fn("max", "apple", "fig") == "fig"

    def test_min_strings(self) -> None:
        assert fn("min", "apple", "fig") == "apple"

    def test_max_with_null_returns_non_null(self) -> None:
        # NULL is "less than everything" so MAX(x, NULL) → x
        assert fn("max", 1, None) == 1
        assert fn("max", None, 1) == 1

    def test_min_with_null_returns_null(self) -> None:
        # NULL is "less than everything" so MIN(x, NULL) → NULL
        assert fn("min", 1, None) is None
        assert fn("min", None, 1) is None

    def test_max_all_null(self) -> None:
        assert fn("max", None, None) is None

    def test_min_all_null(self) -> None:
        assert fn("min", None, None) is None

    def test_max_negative_numbers(self) -> None:
        assert fn("max", -5, -1) == -1

    def test_min_negative_numbers(self) -> None:
        assert fn("min", -5, -1) == -5

    def test_max_mixed_int_float(self) -> None:
        assert fn("max", 2, 1.5) == 2

    def test_min_mixed_int_float(self) -> None:
        assert fn("min", 2, 1.5) == 1.5


# ===========================================================================
# Date/time functions
# ===========================================================================


class TestDateTimeFunctions:
    """DATE, TIME, DATETIME, JULIANDAY, UNIXEPOCH, STRFTIME."""

    # ------------------------------------------------------------------
    # NULL propagation
    # ------------------------------------------------------------------

    def test_date_null(self) -> None:
        assert fn("date", None) is None

    def test_time_null(self) -> None:
        assert fn("time", None) is None

    def test_datetime_null(self) -> None:
        assert fn("datetime", None) is None

    def test_julianday_null(self) -> None:
        assert fn("julianday", None) is None

    def test_unixepoch_null(self) -> None:
        assert fn("unixepoch", None) is None

    def test_strftime_null_format(self) -> None:
        assert fn("strftime", None, "now") is None

    def test_strftime_null_timevalue(self) -> None:
        assert fn("strftime", "%Y", None) is None

    # ------------------------------------------------------------------
    # 'now' → correct format
    # ------------------------------------------------------------------

    def test_date_now_format(self) -> None:
        import re
        result = fn("date", "now")
        assert isinstance(result, str)
        assert re.match(r"^\d{4}-\d{2}-\d{2}$", result), f"bad format: {result!r}"

    def test_time_now_format(self) -> None:
        import re
        result = fn("time", "now")
        assert isinstance(result, str)
        assert re.match(r"^\d{2}:\d{2}:\d{2}$", result), f"bad format: {result!r}"

    def test_datetime_now_format(self) -> None:
        import re
        result = fn("datetime", "now")
        assert isinstance(result, str)
        assert re.match(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$", result), f"bad format: {result!r}"

    def test_julianday_now_is_float(self) -> None:
        result = fn("julianday", "now")
        assert isinstance(result, float)
        # Julian Day for 2024+ should be > 2451544 (year 2000)
        assert result > 2451544.0

    def test_unixepoch_now_is_positive_int(self) -> None:
        result = fn("unixepoch", "now")
        assert isinstance(result, int)
        assert result > 946684800  # > year 2000

    # ------------------------------------------------------------------
    # Known fixed time values
    # ------------------------------------------------------------------

    def test_date_iso_string(self) -> None:
        assert fn("date", "2024-03-15") == "2024-03-15"

    def test_time_from_datetime_string(self) -> None:
        assert fn("time", "2024-03-15 14:30:45") == "14:30:45"

    def test_datetime_from_string(self) -> None:
        assert fn("datetime", "2024-03-15 14:30:00") == "2024-03-15 14:30:00"

    def test_julianday_known_constant(self) -> None:
        # 2000-01-01 00:00:00 UTC = JD 2451544.5 (well-known constant)
        result = fn("julianday", "2000-01-01")
        assert isinstance(result, float)
        assert abs(result - 2451544.5) < 1e-6

    def test_unixepoch_epoch(self) -> None:
        assert fn("unixepoch", "1970-01-01") == 0

    def test_unixepoch_known_date(self) -> None:
        # 2000-01-01 00:00:00 UTC = 946684800
        assert fn("unixepoch", "2000-01-01") == 946684800

    def test_unixepoch_julian_day_input(self) -> None:
        # Julian Day 2440587.5 = 1970-01-01 UTC
        result = fn("unixepoch", 2440587.5)
        assert result == 0

    def test_date_from_unix_epoch_int(self) -> None:
        # Unix timestamp 0 → 1970-01-01
        assert fn("date", 0) == "1970-01-01"

    def test_date_from_julian_day_float(self) -> None:
        # JD 2451544.5 → 2000-01-01
        assert fn("date", 2451544.5) == "2000-01-01"

    # ------------------------------------------------------------------
    # Modifiers
    # ------------------------------------------------------------------

    def test_date_plus_days(self) -> None:
        assert fn("date", "2024-03-15", "+1 days") == "2024-03-16"

    def test_date_minus_days(self) -> None:
        assert fn("date", "2024-03-15", "-1 days") == "2024-03-14"

    def test_date_plus_months(self) -> None:
        assert fn("date", "2024-02-15", "+1 months") == "2024-03-15"

    def test_date_plus_month_leap_year_clamp(self) -> None:
        # Jan 31 + 1 month → Feb 29 (2024 is a leap year)
        assert fn("date", "2024-01-31", "+1 months") == "2024-02-29"

    def test_date_plus_month_non_leap_clamp(self) -> None:
        # Jan 31 + 1 month → Feb 28 (2023 is NOT a leap year)
        assert fn("date", "2023-01-31", "+1 months") == "2023-02-28"

    def test_date_start_of_month(self) -> None:
        assert fn("date", "2024-03-15", "start of month") == "2024-03-01"

    def test_date_start_of_year(self) -> None:
        assert fn("date", "2024-07-04", "start of year") == "2024-01-01"

    def test_datetime_start_of_day(self) -> None:
        assert fn("datetime", "2024-03-15 14:30:00", "start of day") == "2024-03-15 00:00:00"

    def test_datetime_compound_modifiers(self) -> None:
        # +1 day then -2 hours: net +22 hours from 2024-03-15 12:00:00
        result = fn("datetime", "2024-03-15 12:00:00", "+1 days", "-2 hours")
        assert result == "2024-03-16 10:00:00"

    def test_date_plus_years(self) -> None:
        assert fn("date", "2020-03-15", "+2 years") == "2022-03-15"

    def test_date_minus_months(self) -> None:
        assert fn("date", "2024-03-15", "-2 months") == "2024-01-15"

    def test_time_plus_minutes(self) -> None:
        assert fn("time", "2024-03-15 12:00:00", "+30 minutes") == "12:30:00"

    def test_time_plus_seconds(self) -> None:
        assert fn("time", "2024-03-15 12:00:00", "+90 seconds") == "12:01:30"

    # ------------------------------------------------------------------
    # STRFTIME
    # ------------------------------------------------------------------

    def test_strftime_year_month(self) -> None:
        assert fn("strftime", "%Y-%m", "2024-03-15") == "2024-03"

    def test_strftime_full_date(self) -> None:
        assert fn("strftime", "%Y-%m-%d", "2024-03-15") == "2024-03-15"

    def test_strftime_epoch_of_known_date(self) -> None:
        # 2000-01-01 00:00:00 UTC = 946684800
        assert fn("strftime", "%s", "2000-01-01") == "946684800"

    def test_strftime_day_of_year(self) -> None:
        # 2024-01-01 is day 001 of the year
        assert fn("strftime", "%j", "2024-01-01") == "001"
        # 2024-12-31 is day 366 (2024 is a leap year)
        assert fn("strftime", "%j", "2024-12-31") == "366"

    def test_strftime_time_components(self) -> None:
        assert fn("strftime", "%H:%M:%S", "2024-03-15 14:30:45") == "14:30:45"

    def test_strftime_with_modifier(self) -> None:
        result = fn("strftime", "%Y-%m-%d", "2024-03-15", "-7 days")
        assert result == "2024-03-08"

    def test_strftime_percent_literal(self) -> None:
        assert fn("strftime", "100%%", "2024-03-15") == "100%"

    def test_strftime_fractional_seconds(self) -> None:
        # %f = SS.SSS; no sub-second input → 00.000
        result = fn("strftime", "%f", "2024-03-15 14:30:45")
        assert result == "45.000"
