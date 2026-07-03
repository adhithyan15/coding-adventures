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

    def test_cast_blob_to_text_utf8_decodes(self) -> None:
        # bytes → UTF-8-decoded text (matches SQLite, replaces the
        # earlier hex-encoding behaviour as of sql-vm 1.59.0).
        assert fn("cast", b"Hello", "text") == "Hello"
        assert fn("cast", b"42", "text") == "42"
        # Empty BLOB round-trips to empty string.
        assert fn("cast", b"", "text") == ""
        # Invalid UTF-8 bytes are replaced with U+FFFD rather than
        # raising — keeps the cast total and matches SQLite's
        # "decode lazily, never error mid-query" stance.
        assert fn("cast", b"\xff", "text") == "�"

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
        # Non-numeric strings: SQLite returns 0.0 (not the original string).
        # SQLite's ABS() coerces the argument to a number — strings that
        # contain no numeric prefix coerce to 0.
        assert fn("abs", "hello") == 0.0

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
    def test_log_base_10(self) -> None:
        # SQLite: LOG(x) is base-10 logarithm.  LOG(100) → 2.0
        result = fn("log", 100)
        assert result == pytest.approx(2.0)

    def test_ln_natural(self) -> None:
        # SQLite: LN(x) is the natural logarithm.  LN(e) → 1.0
        result = fn("ln", math.e)
        assert result == pytest.approx(1.0)

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
        # SQLite returns empty string for HEX(NULL), not NULL.
        assert fn("hex", None) == ""

    def test_hex_integer(self) -> None:
        # SQLite: HEX(N) operates on the decimal string representation of N.
        # HEX(255) → HEX("255") → ASCII bytes 0x32 0x35 0x35 → "323535"
        result = fn("hex", 255)
        assert result == "323535"

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
        # %q doubles internal single quotes and emits NO surrounding
        # quotes (the caller wraps).  This matches SQLite's reference
        # behaviour; the previous assertion was wrong and matched what
        # %Q does instead.  See test_printf_q_w_correct.py for the full
        # %q / %Q / %w grid.
        assert fn("printf", "%q", "it's") == "it''s"

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

    def test_last_insert_rowid_default_zero(self) -> None:
        # With connection-state plumbing the VM-level default is 0;
        # engine-level integration tests cover the real values.
        from sql_vm.scalar_functions import set_connection_state
        set_connection_state(last_insert_rowid=0, changes=0, total_changes=0)
        assert fn("last_insert_rowid") == 0

    def test_changes_default_zero(self) -> None:
        from sql_vm.scalar_functions import set_connection_state
        set_connection_state(changes=0)
        assert fn("changes") == 0

    def test_total_changes_default_zero(self) -> None:
        from sql_vm.scalar_functions import set_connection_state
        set_connection_state(total_changes=0)
        assert fn("total_changes") == 0

    def test_set_connection_state_updates_globals(self) -> None:
        from sql_vm.scalar_functions import set_connection_state
        set_connection_state(last_insert_rowid=42, changes=7, total_changes=99)
        assert fn("last_insert_rowid") == 42
        assert fn("changes") == 7
        assert fn("total_changes") == 99

    def test_sqlite_version_returns_string(self) -> None:
        v = fn("sqlite_version")
        assert isinstance(v, str)
        parts = v.split(".")
        assert len(parts) >= 2
        assert all(p.isdigit() for p in parts)

    def test_sqlite_source_id_returns_string(self) -> None:
        s = fn("sqlite_source_id")
        assert isinstance(s, str)
        assert len(s) > 0


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

    def test_max_with_null_returns_null(self) -> None:
        # Scalar MAX propagates NULL: any NULL argument → NULL result.
        # This matches SQLite's multi-argument MAX() semantics, where NULL
        # infects the result (unlike the aggregate MAX which ignores NULLs).
        assert fn("max", 1, None) is None
        assert fn("max", None, 1) is None

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

    def test_date_plus_month_leap_year_overflow(self) -> None:
        # Jan 31 + 1 month: Feb 31 does not exist. SQLite overflows into the
        # next month.  2024 is a leap year (Feb has 29 days), so overflow is
        # 31 - 29 = 2 days, landing on March 2.
        assert fn("date", "2024-01-31", "+1 months") == "2024-03-02"

    def test_date_plus_month_non_leap_overflow(self) -> None:
        # Jan 31 + 1 month: Feb 31 does not exist. 2023 is NOT a leap year
        # (Feb has 28 days), so overflow is 31 - 28 = 3 days → March 3.
        assert fn("date", "2023-01-31", "+1 months") == "2023-03-03"

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

    # ------------------------------------------------------------------
    # Time-only string inputs (HH:MM, HH:MM:SS)
    # ------------------------------------------------------------------

    def test_time_from_bare_hhmmss(self) -> None:
        # SQLite accepts bare 'HH:MM:SS' as a time value anchored to 2000-01-01.
        assert fn("time", "12:34:56") == "12:34:56"

    def test_time_from_bare_hhmm(self) -> None:
        # 'HH:MM' with no seconds — seconds default to 00.
        assert fn("time", "00:00") == "00:00:00"

    def test_date_from_bare_time_string(self) -> None:
        # date() of a bare time string anchors to 2000-01-01.
        assert fn("date", "12:34:56") == "2000-01-01"

    # ------------------------------------------------------------------
    # weekday N modifier
    # ------------------------------------------------------------------

    def test_date_weekday_same_day(self) -> None:
        # 2024-01-15 is a Monday (SQLite weekday 1). Advancing to weekday 1
        # when already on Monday → same date.
        assert fn("date", "2024-01-15", "weekday 1") == "2024-01-15"

    def test_date_weekday_advance_to_tuesday(self) -> None:
        # 2024-01-15 is Monday; next Tuesday (weekday 2) is 2024-01-16.
        assert fn("date", "2024-01-15", "weekday 2") == "2024-01-16"

    def test_date_weekday_advance_to_sunday(self) -> None:
        # 2024-01-15 is Monday; next Sunday (weekday 0) is 2024-01-21.
        assert fn("date", "2024-01-15", "weekday 0") == "2024-01-21"

    # ------------------------------------------------------------------
    # unixepoch modifier — forces numeric interpretation of the time value
    # ------------------------------------------------------------------

    def test_date_unixepoch_modifier_rejects_date_string(self) -> None:
        # SQLite returns NULL when the unixepoch modifier is applied to a
        # date-formatted string (it has no numeric prefix).  Mini-sqlite
        # previously ignored the modifier and returned the date as-is;
        # the corrected behaviour matches sqlite3.
        assert fn("date", "2024-01-15", "unixepoch") is None

    def test_date_unixepoch_modifier_numeric_string(self) -> None:
        # Numeric string IS valid input for the unixepoch modifier — it
        # is parsed as an integer count of seconds since the epoch.
        assert fn("date", "1704067200", "unixepoch") == "2024-01-01"


# ===========================================================================
# JSON path-shortcut operator helpers (__json_arrow / __json_arrow_text)
# ===========================================================================
#
# These are the implementations behind the `->` and `->>` operators
# (SQLite 3.38+).  The adapter rewrites the operator into a call to one of
# these functions; they are not user-facing under their double-underscore
# names but are reachable via the registry like any other scalar.


class TestJsonArrow:
    """``__json_arrow(j, path)`` — implements ``j -> path`` (JSON-typed result)."""

    def test_array_index(self) -> None:
        # JSON form of an integer is the JSON-encoded string "1"
        assert fn("__json_arrow", "[1,2,3]", 0) == "1"

    def test_array_index_middle(self) -> None:
        assert fn("__json_arrow", "[1,2,3]", 1) == "2"

    def test_array_string_value(self) -> None:
        assert fn("__json_arrow", '["a","b","c"]', 1) == '"b"'

    def test_object_key(self) -> None:
        assert fn("__json_arrow", '{"a":1,"b":2}', "a") == "1"

    def test_object_missing_key(self) -> None:
        assert fn("__json_arrow", '{"a":1}', "missing") is None

    def test_object_returns_nested_as_json(self) -> None:
        result = fn("__json_arrow", '{"x":{"y":42}}', "x")
        assert result == '{"y":42}'

    def test_array_returns_array_as_json(self) -> None:
        assert fn("__json_arrow", '{"items":[1,2,3]}', "items") == "[1,2,3]"

    def test_explicit_dollar_path(self) -> None:
        assert fn("__json_arrow", '[10,20,30]', "$[2]") == "30"

    def test_explicit_nested_dollar_path(self) -> None:
        assert fn("__json_arrow", '{"a":{"b":7}}', "$.a.b") == "7"

    def test_null_json_propagates(self) -> None:
        assert fn("__json_arrow", None, 0) is None

    def test_null_path_propagates(self) -> None:
        assert fn("__json_arrow", "[1,2,3]", None) is None

    def test_non_string_json_returns_null(self) -> None:
        assert fn("__json_arrow", 42, 0) is None

    def test_invalid_json_returns_null(self) -> None:
        assert fn("__json_arrow", "not json", 0) is None

    def test_array_out_of_bounds_returns_null(self) -> None:
        assert fn("__json_arrow", "[1,2,3]", 99) is None

    def test_boolean_path_is_rejected(self) -> None:
        # SQLite rejects booleans on the right of -> / ->>; we return NULL.
        assert fn("__json_arrow", "[1,2,3]", True) is None

    def test_float_path_is_rejected(self) -> None:
        # Floats are also not a valid path type.
        assert fn("__json_arrow", "[1,2,3]", 1.5) is None


class TestJsonArrowText:
    """``__json_arrow_text(j, path)`` — implements ``j ->> path`` (SQL-typed)."""

    def test_array_index_returns_integer(self) -> None:
        assert fn("__json_arrow_text", "[1,2,3]", 0) == 1

    def test_array_string_value_unwrapped(self) -> None:
        assert fn("__json_arrow_text", '["a","b","c"]', 1) == "b"

    def test_object_key_returns_integer(self) -> None:
        assert fn("__json_arrow_text", '{"a":1,"b":2}', "a") == 1

    def test_object_key_returns_text(self) -> None:
        assert fn("__json_arrow_text", '{"name":"alice"}', "name") == "alice"

    def test_object_value_stays_json_text(self) -> None:
        """``->>`` does NOT unwrap composite values — they stay as JSON text."""
        result = fn("__json_arrow_text", '{"a":{"b":1}}', "a")
        assert result == '{"b":1}'

    def test_array_value_stays_json_text(self) -> None:
        result = fn("__json_arrow_text", '{"items":[1,2,3]}', "items")
        assert result == "[1,2,3]"

    def test_null_json_propagates(self) -> None:
        assert fn("__json_arrow_text", None, 0) is None

    def test_null_path_propagates(self) -> None:
        assert fn("__json_arrow_text", "[1,2,3]", None) is None

    def test_missing_path_returns_null(self) -> None:
        assert fn("__json_arrow_text", '{"a":1}', "missing") is None

    def test_explicit_dollar_path(self) -> None:
        assert fn("__json_arrow_text", '{"a":{"b":42}}', "$.a.b") == 42

    def test_non_string_json_returns_null(self) -> None:
        assert fn("__json_arrow_text", 42, 0) is None

    def test_invalid_json_returns_null(self) -> None:
        assert fn("__json_arrow_text", "not json", 0) is None


# ---------------------------------------------------------------------------
# Hyperbolic trig — sinh / cosh / tanh / asinh / acosh / atanh
# ---------------------------------------------------------------------------


class TestHyperbolicTrig:
    """Hyperbolic trig functions match Python ``math`` (and thus SQLite)."""

    def test_sinh_zero(self) -> None:
        assert fn("sinh", 0) == 0.0

    def test_sinh_one(self) -> None:
        assert fn("sinh", 1) == pytest.approx(math.sinh(1))

    def test_cosh_zero(self) -> None:
        assert fn("cosh", 0) == 1.0

    def test_cosh_one(self) -> None:
        assert fn("cosh", 1) == pytest.approx(math.cosh(1))

    def test_tanh_zero(self) -> None:
        assert fn("tanh", 0) == 0.0

    def test_tanh_large(self) -> None:
        # tanh saturates to ±1 for |x| > ~20
        assert fn("tanh", 100) == pytest.approx(1.0)
        assert fn("tanh", -100) == pytest.approx(-1.0)

    def test_asinh_inverse(self) -> None:
        # asinh(sinh(x)) == x for all reals
        assert fn("asinh", math.sinh(1.5)) == pytest.approx(1.5)

    def test_asinh_negative(self) -> None:
        # asinh is defined for all reals, including negatives
        assert fn("asinh", -1) == pytest.approx(math.asinh(-1))

    def test_acosh_one(self) -> None:
        # acosh(1) = 0 (the minimum of the domain)
        assert fn("acosh", 1) == 0.0

    def test_acosh_below_domain_returns_null(self) -> None:
        # acosh is undefined for x < 1; should yield NULL not error.
        assert fn("acosh", 0.5) is None

    def test_atanh_zero(self) -> None:
        assert fn("atanh", 0) == 0.0

    def test_atanh_at_boundary_returns_null(self) -> None:
        # atanh(±1) = ±∞ → not finite → NULL
        assert fn("atanh", 1) is None
        assert fn("atanh", -1) is None

    def test_atanh_outside_domain_returns_null(self) -> None:
        # atanh undefined for |x| > 1
        assert fn("atanh", 2) is None

    def test_null_propagates_for_all_hyperbolic(self) -> None:
        for name in ("sinh", "cosh", "tanh", "asinh", "acosh", "atanh"):
            assert fn(name, None) is None, f"{name}(NULL) should be NULL"

    def test_non_numeric_returns_null(self) -> None:
        for name in ("sinh", "cosh", "tanh", "asinh", "acosh", "atanh"):
            assert fn(name, "hello") is None, f"{name}('hello') should be NULL"


# ---------------------------------------------------------------------------
# trunc(X) — truncate toward zero
# ---------------------------------------------------------------------------


class TestTrunc:
    """``trunc(X)`` drops the fractional part of *X*, keeping the sign."""

    def test_positive_real(self) -> None:
        assert fn("trunc", 3.7) == 3.0

    def test_negative_real(self) -> None:
        # Differs from floor(−3.7) = −4.0
        assert fn("trunc", -3.7) == -3.0

    def test_positive_integer(self) -> None:
        # Already truncated; should round-trip as REAL (per SQLite)
        assert fn("trunc", 5) == 5.0

    def test_zero(self) -> None:
        assert fn("trunc", 0) == 0.0
        assert fn("trunc", 0.0) == 0.0

    def test_returns_real_not_int(self) -> None:
        # SQLite returns REAL even when the truncated value is whole
        assert isinstance(fn("trunc", 3.0), float)

    def test_null_propagates(self) -> None:
        assert fn("trunc", None) is None

    def test_non_numeric_returns_null(self) -> None:
        assert fn("trunc", "hello") is None


# ---------------------------------------------------------------------------
# Optimizer hints — likely / unlikely / likelihood
# ---------------------------------------------------------------------------


class TestOptimizerHints:
    """``likely``, ``unlikely``, and ``likelihood`` are pure identity passes."""

    def test_likely_passes_integer(self) -> None:
        assert fn("likely", 42) == 42

    def test_likely_passes_string(self) -> None:
        assert fn("likely", "hello") == "hello"

    def test_likely_passes_null(self) -> None:
        assert fn("likely", None) is None

    def test_unlikely_passes_value(self) -> None:
        assert fn("unlikely", 7) == 7
        assert fn("unlikely", 3.14) == 3.14
        assert fn("unlikely", None) is None

    def test_likelihood_passes_first_arg(self) -> None:
        # The second argument is a probability hint — ignored at runtime.
        assert fn("likelihood", 99, 0.5) == 99
        assert fn("likelihood", "x", 0.99) == "x"
        assert fn("likelihood", None, 0.01) is None


# ---------------------------------------------------------------------------
# sqlite_compileoption_used / sqlite_compileoption_get
# ---------------------------------------------------------------------------


class TestSqliteCompileOptions:
    """Mini-sqlite has no SQLite compile-time options; both functions stub out."""

    def test_compileoption_used_returns_zero(self) -> None:
        # Any name returns 0 — no options are defined in mini-sqlite.
        assert fn("sqlite_compileoption_used", "THREADSAFE") == 0
        assert fn("sqlite_compileoption_used", "ENABLE_RTREE") == 0
        assert fn("sqlite_compileoption_used", "ANY_FAKE_NAME") == 0

    def test_compileoption_get_returns_null(self) -> None:
        # Any index returns NULL — no options exist.
        assert fn("sqlite_compileoption_get", 0) is None
        assert fn("sqlite_compileoption_get", 100) is None


# ---------------------------------------------------------------------------
# Datetime modifier — timezone offsets, auto, julianday-no-op
# ---------------------------------------------------------------------------


class TestTimezoneOffsetModifier:
    """``+HH:MM`` / ``-HH:MM`` / ``+HH:MM:SS`` shift the datetime by that offset."""

    def test_positive_hours(self) -> None:
        # Adding +02:00 moves UTC clock forward by 2 hours.
        assert fn("datetime", "2024-03-15 14:30:00", "+02:00") == "2024-03-15 16:30:00"

    def test_negative_hours_minutes(self) -> None:
        # -05:30 moves clock back 5h30m.
        assert fn("datetime", "2024-03-15 14:30:00", "-05:30") == "2024-03-15 09:00:00"

    def test_with_seconds(self) -> None:
        # +02:30:45 → +2h30m45s
        assert fn("datetime", "2024-03-15 14:30:00", "+02:30:45") == "2024-03-15 17:00:45"

    def test_zero_offset(self) -> None:
        # +00:00 is a no-op.
        assert fn("datetime", "2024-03-15 14:30:00", "+00:00") == "2024-03-15 14:30:00"

    def test_offset_at_day_boundary(self) -> None:
        # -05:00 from 02:00 should roll back to previous day.
        assert fn("datetime", "2024-03-15 02:00:00", "-05:00") == "2024-03-14 21:00:00"

    def test_hour_out_of_range_returns_null(self) -> None:
        # SQLite returns NULL for invalid offsets like +99:00.
        assert fn("datetime", "2024-03-15 14:30:00", "+99:00") is None

    def test_minute_out_of_range_returns_null(self) -> None:
        assert fn("datetime", "2024-03-15 14:30:00", "+02:99") is None

    def test_with_date_only(self) -> None:
        # Applying timezone to date() crops back to date — verify via date().
        # date('2024-03-15', '+12:00') → still 2024-03-15 (midnight + 12h)
        assert fn("date", "2024-03-15", "+12:00") == "2024-03-15"

    def test_offset_chained_with_other_modifier(self) -> None:
        # Modifiers compose left-to-right.
        result = fn(
            "datetime", "2024-03-15 14:30:00", "+02:00", "+1 day"
        )
        assert result == "2024-03-16 16:30:00"


class TestAutoModifier:
    """The ``auto`` modifier no longer triggers NULL propagation.

    Mini-sqlite's :func:`_parse_timevalue` dispatches numeric time values
    by Python type (``int`` → Unix epoch, ``float`` → Julian day), so
    ``auto`` is a semantic no-op here.  Accepting it matches SQLite's
    behaviour on string inputs (pass-through).
    """

    def test_auto_modifier_no_op_on_string(self) -> None:
        # 'auto' should not cause NULL propagation; passes the datetime through.
        assert fn("datetime", "2024-03-15 14:30:00", "auto") == "2024-03-15 14:30:00"

    def test_auto_modifier_with_chained_offset(self) -> None:
        # 'auto' composes with subsequent modifiers.
        assert (
            fn("datetime", "2024-03-15 14:30:00", "auto", "+1 day")
            == "2024-03-16 14:30:00"
        )

    def test_unrecognised_modifier_still_returns_null(self) -> None:
        # Confirm that we didn't accidentally swallow unknown modifiers.
        assert fn("datetime", "2024-03-15 14:30:00", "totally_made_up") is None


# ---------------------------------------------------------------------------
# strftime %P (lowercase am/pm) cross-platform
# ---------------------------------------------------------------------------


class TestStrftimeLowerCaseAmPm:
    """``%P`` produces ``am``/``pm`` on every platform.

    Python's macOS libc returns the literal ``'P'`` for ``strftime('%P')``
    rather than ``'pm'``.  We pre-process ``%P`` ourselves so output is
    identical on Linux, macOS, and Windows CI.
    """

    def test_pm_at_afternoon(self) -> None:
        assert fn("strftime", "%P", "2024-03-15 14:30:00") == "pm"

    def test_am_at_morning(self) -> None:
        assert fn("strftime", "%P", "2024-03-15 06:30:00") == "am"

    def test_am_at_midnight(self) -> None:
        # 00:00 — should be 'am' (12 AM convention).
        assert fn("strftime", "%P", "2024-03-15 00:00:00") == "am"

    def test_pm_at_noon(self) -> None:
        # 12:00 — should be 'pm' (12 PM convention).
        assert fn("strftime", "%P", "2024-03-15 12:00:00") == "pm"

    def test_combined_format(self) -> None:
        # %P composes with other specifiers.
        assert (
            fn("strftime", "%I:%M %P", "2024-03-15 14:30:00") == "02:30 pm"
        )


# ---------------------------------------------------------------------------
# concat / concat_ws / octet_length (SQLite 3.44+ string family additions)
# ---------------------------------------------------------------------------


class TestConcat:
    """``CONCAT(...)`` — variadic, NULLs treated as empty string."""

    def test_two_strings(self) -> None:
        assert fn("concat", "a", "b") == "ab"

    def test_three_strings(self) -> None:
        assert fn("concat", "a", "b", "c") == "abc"

    def test_null_skipped(self) -> None:
        # NULLs are treated as empty strings — NOT as NULL-propagation.
        assert fn("concat", "a", None, "c") == "ac"

    def test_all_nulls_returns_empty(self) -> None:
        assert fn("concat", None, None, None) == ""

    def test_numeric_coerced(self) -> None:
        # SQLite coerces non-strings via their text representation.
        assert fn("concat", "id=", 42) == "id=42"

    def test_float_coerced(self) -> None:
        assert fn("concat", "v=", 3.14) == "v=3.14"

    def test_single_arg(self) -> None:
        # Minimum 1 arg — single-arg form is just identity-via-str.
        assert fn("concat", "hello") == "hello"

    def test_zero_args_raises(self) -> None:
        from sql_vm.errors import WrongNumberOfArguments

        try:
            fn("concat")
            raise AssertionError("expected WrongNumberOfArguments")
        except WrongNumberOfArguments:
            pass


class TestConcatWs:
    """``CONCAT_WS(sep, ...)`` — separator-aware concatenation."""

    def test_basic(self) -> None:
        assert fn("concat_ws", "-", "a", "b", "c") == "a-b-c"

    def test_null_arg_skipped(self) -> None:
        # Distinct from concat: NULL values are SKIPPED, separator not doubled.
        assert fn("concat_ws", "-", "a", None, "c") == "a-c"

    def test_null_separator_returns_null(self) -> None:
        # Unlike CONCAT, a NULL separator propagates — the whole result is NULL.
        assert fn("concat_ws", None, "a", "b") is None

    def test_multi_char_separator(self) -> None:
        assert fn("concat_ws", " | ", "a", "b", "c") == "a | b | c"

    def test_empty_separator(self) -> None:
        # Empty-string sep behaves like concat.
        assert fn("concat_ws", "", "a", "b", "c") == "abc"

    def test_numeric_args_coerced(self) -> None:
        assert fn("concat_ws", ",", 1, 2, 3) == "1,2,3"

    def test_only_separator(self) -> None:
        # SQLite accepts just the sep and returns an empty string.
        assert fn("concat_ws", "-") == ""


class TestOctetLength:
    """``OCTET_LENGTH(s)`` — byte length of UTF-8-encoded text."""

    def test_ascii(self) -> None:
        # ASCII: bytes = chars.
        assert fn("octet_length", "hello") == 5

    def test_empty_string(self) -> None:
        assert fn("octet_length", "") == 0

    def test_non_ascii_utf8(self) -> None:
        # 'café' is 4 chars but 5 bytes ('é' = 2 bytes in UTF-8).
        assert fn("octet_length", "café") == 5

    def test_emoji_4_bytes(self) -> None:
        # '🦀' is 1 char but 4 bytes in UTF-8.
        assert fn("octet_length", "🦀") == 4

    def test_null_propagates(self) -> None:
        assert fn("octet_length", None) is None

    def test_blob(self) -> None:
        assert fn("octet_length", b"\x01\x02\x03\xff") == 4

    def test_integer_uses_decimal_string(self) -> None:
        # SQLite: OCTET_LENGTH(123) → 3 (the byte length of "123")
        assert fn("octet_length", 123) == 3

    def test_negative_integer(self) -> None:
        # OCTET_LENGTH(-42) → 3 (chars '-', '4', '2')
        assert fn("octet_length", -42) == 3
