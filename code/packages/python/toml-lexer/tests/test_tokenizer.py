"""Tests for the TOML lexer thin wrapper.

These tests verify that the grammar-driven lexer, configured with
``toml.tokens``, correctly tokenizes TOML text per the TOML v1.0.0
specification (https://toml.io/en/v1.0.0).

TOML is significantly more complex than JSON: it has four string types,
date/time literals, bare keys, comments, and newline sensitivity. The
ordering of token patterns matters — the tests in ``TestTokenOrdering``
verify that ambiguous inputs resolve to the correct, most-specific token
type.
"""

from __future__ import annotations

import pytest
from lexer import GrammarLexer

from toml_lexer import LexerError, Token, create_toml_lexer, tokenize_toml

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
#
# These helper functions strip away the boilerplate of extracting token
# types and values from the full Token objects. They also filter out
# NEWLINE and EOF tokens by default, since most tests care only about
# the "content" tokens.


def _get_type_name(token: Token) -> str:
    """Extract the type name from a Token, handling both str and enum types."""
    return token.type if isinstance(token.type, str) else token.type.name


def token_types(source: str, *, keep_newlines: bool = False) -> list[str]:
    """Tokenize and return just the type names (excluding EOF).

    By default, NEWLINE tokens are also excluded because most tests focus
    on content tokens. Pass ``keep_newlines=True`` to include them.
    """
    tokens = tokenize_toml(source)
    skip = {"EOF"} if keep_newlines else {"EOF", "NEWLINE"}
    return [_get_type_name(t) for t in tokens if _get_type_name(t) not in skip]


def token_values(source: str, *, keep_newlines: bool = False) -> list[str]:
    """Tokenize and return just the values (excluding EOF)."""
    tokens = tokenize_toml(source)
    skip = {"EOF"} if keep_newlines else {"EOF", "NEWLINE"}
    return [t.value for t in tokens if _get_type_name(t) not in skip]


def token_pairs(
    source: str, *, keep_newlines: bool = False,
) -> list[tuple[str, str]]:
    """Tokenize and return (type, value) pairs (excluding EOF)."""
    tokens = tokenize_toml(source)
    skip = {"EOF"} if keep_newlines else {"EOF", "NEWLINE"}
    return [
        (_get_type_name(t), t.value)
        for t in tokens
        if _get_type_name(t) not in skip
    ]


# ---------------------------------------------------------------------------
# Factory function tests
# ---------------------------------------------------------------------------


class TestFactory:
    """Tests for the create_toml_lexer factory function."""

    def test_returns_grammar_lexer(self) -> None:
        """create_toml_lexer should return a GrammarLexer instance."""
        lexer = create_toml_lexer('key = "value"')
        assert isinstance(lexer, GrammarLexer)

    def test_factory_produces_tokens(self) -> None:
        """The factory-created lexer should produce valid tokens."""
        lexer = create_toml_lexer('key = "value"')
        tokens = lexer.tokenize()
        assert len(tokens) >= 2  # At minimum: some content + EOF
        assert _get_type_name(tokens[-1]) == "EOF"


# ---------------------------------------------------------------------------
# Basic string tests
# ---------------------------------------------------------------------------


class TestBasicStrings:
    """Tests for TOML basic strings (double-quoted, escape-supporting)."""

    def test_simple_basic_string(self) -> None:
        """A simple double-quoted string."""
        types = token_types('"hello"')
        assert types == ["BASIC_STRING"]

    def test_basic_string_value(self) -> None:
        """Basic string token value includes surrounding quotes.

        The TOML grammar uses ``escapes: none``, which tells the lexer to
        leave the entire matched text as-is — quotes included. This is
        deliberate: TOML has four string types with different escape rules,
        so quote stripping and escape processing are deferred to the parser.
        """
        values = token_values('"hello"')
        assert values == ['"hello"']

    def test_empty_basic_string(self) -> None:
        """An empty basic string (quotes preserved in value)."""
        values = token_values('""')
        assert values == ['""']

    def test_basic_string_with_escapes(self) -> None:
        r"""Basic string containing escape sequences (kept raw by lexer).

        The TOML grammar uses ``escapes: none``, which means the lexer
        does NOT process escape sequences or strip quotes. The parser
        layer handles both. So the full ``"hello\nworld"`` is the token
        value, including quotes and the literal backslash-n.
        """
        values = token_values(r'"hello\nworld"')
        assert values == ['"hello\\nworld"']

    def test_basic_string_with_spaces(self) -> None:
        """Basic string with internal spaces (quotes preserved)."""
        values = token_values('"hello world"')
        assert values == ['"hello world"']

    def test_basic_string_with_unicode(self) -> None:
        r"""Basic string with unicode escape (kept raw, quotes preserved)."""
        values = token_values(r'"caf\u00E9"')
        assert values == ['"caf\\u00E9"']


# ---------------------------------------------------------------------------
# Multi-line basic string tests
# ---------------------------------------------------------------------------


class TestMultiLineBasicStrings:
    """Tests for TOML multi-line basic strings (triple-double-quoted).

    Multi-line basic strings are delimited by ``\"\"\"`` on each end. They
    can contain literal newlines and support escape sequences (though the
    lexer leaves escapes unprocessed due to ``escapes: none``).
    """

    def test_simple_ml_basic_string(self) -> None:
        """A multi-line basic string on one line."""
        types = token_types('"""hello"""')
        assert types == ["ML_BASIC_STRING"]

    def test_ml_basic_string_value(self) -> None:
        """Value includes triple quotes (escapes: none keeps raw text)."""
        values = token_values('"""hello"""')
        assert values == ['"""hello"""']

    def test_ml_basic_string_with_newline(self) -> None:
        """Multi-line basic string containing a literal newline."""
        source = '"""hello\nworld"""'
        values = token_values(source)
        assert values == ['"""hello\nworld"""']

    def test_empty_ml_basic_string(self) -> None:
        """An empty multi-line basic string (quotes preserved)."""
        values = token_values('""""""')
        assert values == ['""""""']


# ---------------------------------------------------------------------------
# Literal string tests
# ---------------------------------------------------------------------------


class TestLiteralStrings:
    """Tests for TOML literal strings (single-quoted, no escapes).

    Literal strings are enclosed in single quotes. There is no escaping
    whatsoever — what you see is what you get. This makes them ideal for
    Windows paths and regex patterns.
    """

    def test_simple_literal_string(self) -> None:
        """A simple single-quoted string."""
        types = token_types("'hello'")
        assert types == ["LITERAL_STRING"]

    def test_literal_string_value(self) -> None:
        """Literal string value includes surrounding quotes (escapes: none)."""
        values = token_values("'hello'")
        assert values == ["'hello'"]

    def test_literal_string_with_backslash(self) -> None:
        r"""Backslashes in literal strings are literal (quotes preserved)."""
        values = token_values(r"'C:\Users\Admin'")
        assert values == ["'C:\\Users\\Admin'"]

    def test_empty_literal_string(self) -> None:
        """An empty literal string (quotes preserved)."""
        values = token_values("''")
        assert values == ["''"]


# ---------------------------------------------------------------------------
# Multi-line literal string tests
# ---------------------------------------------------------------------------


class TestMultiLineLiteralStrings:
    """Tests for TOML multi-line literal strings (triple-single-quoted).

    Like literal strings, there is no escaping. Unlike single-line literal
    strings, these can span multiple lines.
    """

    def test_simple_ml_literal_string(self) -> None:
        """A multi-line literal string on one line."""
        types = token_types("'''hello'''")
        assert types == ["ML_LITERAL_STRING"]

    def test_ml_literal_string_value(self) -> None:
        """Value includes triple quotes (escapes: none keeps raw text)."""
        values = token_values("'''hello'''")
        assert values == ["'''hello'''"]

    def test_ml_literal_string_with_newline(self) -> None:
        """Multi-line literal string containing a literal newline."""
        source = "'''hello\nworld'''"
        values = token_values(source)
        assert values == ["'''hello\nworld'''"]

    def test_empty_ml_literal_string(self) -> None:
        """An empty multi-line literal string (quotes preserved)."""
        values = token_values("''''''")
        assert values == ["''''''"]


# ---------------------------------------------------------------------------
# Integer tests
# ---------------------------------------------------------------------------


class TestIntegers:
    """Tests for TOML integer literals.

    TOML supports four integer notations: decimal, hexadecimal (0x),
    octal (0o), and binary (0b). All emit the same INTEGER token type
    thanks to the ``-> INTEGER`` alias in the grammar. Underscores are
    allowed as visual separators between digits.
    """

    def test_simple_integer(self) -> None:
        """A simple decimal integer."""
        pairs = token_pairs("42")
        assert pairs == [("INTEGER", "42")]

    def test_zero(self) -> None:
        """The number zero."""
        pairs = token_pairs("0")
        assert pairs == [("INTEGER", "0")]

    def test_positive_sign(self) -> None:
        """An integer with an explicit positive sign."""
        pairs = token_pairs("+42")
        assert pairs == [("INTEGER", "+42")]

    def test_negative_sign(self) -> None:
        """A negative integer."""
        pairs = token_pairs("-42")
        assert pairs == [("INTEGER", "-42")]

    def test_underscore_separator(self) -> None:
        """Underscores between digits are valid separators."""
        pairs = token_pairs("1_000")
        assert pairs == [("INTEGER", "1_000")]

    def test_large_underscore_integer(self) -> None:
        """A large number with multiple underscore separators."""
        pairs = token_pairs("1_000_000")
        assert pairs == [("INTEGER", "1_000_000")]

    def test_hex_integer(self) -> None:
        """Hexadecimal integer with 0x prefix."""
        pairs = token_pairs("0xDEADBEEF")
        assert pairs == [("INTEGER", "0xDEADBEEF")]

    def test_hex_lowercase(self) -> None:
        """Hexadecimal with lowercase digits."""
        pairs = token_pairs("0xff")
        assert pairs == [("INTEGER", "0xff")]

    def test_hex_with_underscores(self) -> None:
        """Hex integer with underscore separators."""
        pairs = token_pairs("0xdead_beef")
        assert pairs == [("INTEGER", "0xdead_beef")]

    def test_octal_integer(self) -> None:
        """Octal integer with 0o prefix."""
        pairs = token_pairs("0o755")
        assert pairs == [("INTEGER", "0o755")]

    def test_binary_integer(self) -> None:
        """Binary integer with 0b prefix."""
        pairs = token_pairs("0b11010110")
        assert pairs == [("INTEGER", "0b11010110")]

    def test_binary_with_underscores(self) -> None:
        """Binary integer with underscore separators."""
        pairs = token_pairs("0b1101_0110")
        assert pairs == [("INTEGER", "0b1101_0110")]


# ---------------------------------------------------------------------------
# Float tests
# ---------------------------------------------------------------------------


class TestFloats:
    """Tests for TOML float literals.

    TOML floats come in three forms: decimal (``3.14``), scientific
    (``1e10``), and special values (``inf``, ``nan``). All are aliased
    to the FLOAT token type. Signs and underscores are supported.
    """

    def test_simple_float(self) -> None:
        """A simple decimal float."""
        pairs = token_pairs("3.14")
        assert pairs == [("FLOAT", "3.14")]

    def test_negative_float(self) -> None:
        """A negative float."""
        pairs = token_pairs("-3.14")
        assert pairs == [("FLOAT", "-3.14")]

    def test_positive_float(self) -> None:
        """A float with explicit positive sign."""
        pairs = token_pairs("+3.14")
        assert pairs == [("FLOAT", "+3.14")]

    def test_float_with_underscores(self) -> None:
        """Float with underscore separators."""
        pairs = token_pairs("3_141.592_653")
        assert pairs == [("FLOAT", "3_141.592_653")]

    def test_scientific_notation(self) -> None:
        """Float in scientific notation."""
        pairs = token_pairs("1e10")
        assert pairs == [("FLOAT", "1e10")]

    def test_scientific_with_decimal(self) -> None:
        """Scientific notation with decimal point."""
        pairs = token_pairs("6.022e23")
        assert pairs == [("FLOAT", "6.022e23")]

    def test_scientific_negative_exponent(self) -> None:
        """Scientific notation with negative exponent."""
        pairs = token_pairs("1e-10")
        assert pairs == [("FLOAT", "1e-10")]

    def test_scientific_positive_exponent(self) -> None:
        """Scientific notation with explicit positive exponent."""
        pairs = token_pairs("1e+10")
        assert pairs == [("FLOAT", "1e+10")]

    def test_scientific_uppercase_e(self) -> None:
        """Scientific notation with uppercase E."""
        pairs = token_pairs("1E10")
        assert pairs == [("FLOAT", "1E10")]

    def test_inf(self) -> None:
        """Positive infinity."""
        pairs = token_pairs("inf")
        assert pairs == [("FLOAT", "inf")]

    def test_positive_inf(self) -> None:
        """Explicitly positive infinity."""
        pairs = token_pairs("+inf")
        assert pairs == [("FLOAT", "+inf")]

    def test_negative_inf(self) -> None:
        """Negative infinity."""
        pairs = token_pairs("-inf")
        assert pairs == [("FLOAT", "-inf")]

    def test_nan(self) -> None:
        """Not a number."""
        pairs = token_pairs("nan")
        assert pairs == [("FLOAT", "nan")]

    def test_positive_nan(self) -> None:
        """Explicitly positive NaN."""
        pairs = token_pairs("+nan")
        assert pairs == [("FLOAT", "+nan")]

    def test_negative_nan(self) -> None:
        """Negative NaN."""
        pairs = token_pairs("-nan")
        assert pairs == [("FLOAT", "-nan")]


# ---------------------------------------------------------------------------
# Boolean tests
# ---------------------------------------------------------------------------


class TestBooleans:
    """Tests for TOML boolean literals.

    TOML booleans are always lowercase: ``true`` and ``false``. They are
    matched as literal tokens before the BARE_KEY pattern can claim them.
    """

    def test_true(self) -> None:
        """The literal true."""
        pairs = token_pairs("true")
        assert pairs == [("TRUE", "true")]

    def test_false(self) -> None:
        """The literal false."""
        pairs = token_pairs("false")
        assert pairs == [("FALSE", "false")]


# ---------------------------------------------------------------------------
# Date/time tests
# ---------------------------------------------------------------------------


class TestDateTimes:
    """Tests for TOML date/time literals.

    TOML has four date/time types, ordered from most to least specific:

    1. OFFSET_DATETIME — full date + time + timezone
    2. LOCAL_DATETIME — full date + time, no timezone
    3. LOCAL_DATE — date only
    4. LOCAL_TIME — time only

    The ordering in the grammar ensures the most specific pattern matches
    first. Without this ordering, ``1979-05-27T07:32:00Z`` might match
    as LOCAL_DATE (``1979-05-27``) followed by other tokens.
    """

    def test_offset_datetime_utc(self) -> None:
        """Offset datetime with Z (UTC) suffix."""
        pairs = token_pairs("1979-05-27T07:32:00Z")
        assert pairs == [("OFFSET_DATETIME", "1979-05-27T07:32:00Z")]

    def test_offset_datetime_positive_offset(self) -> None:
        """Offset datetime with positive timezone offset."""
        pairs = token_pairs("1979-05-27T07:32:00+05:30")
        assert pairs == [("OFFSET_DATETIME", "1979-05-27T07:32:00+05:30")]

    def test_offset_datetime_negative_offset(self) -> None:
        """Offset datetime with negative timezone offset."""
        pairs = token_pairs("1979-05-27T07:32:00-05:00")
        assert pairs == [("OFFSET_DATETIME", "1979-05-27T07:32:00-05:00")]

    def test_offset_datetime_with_fractional_seconds(self) -> None:
        """Offset datetime with fractional seconds."""
        pairs = token_pairs("1979-05-27T07:32:00.999999Z")
        assert pairs == [("OFFSET_DATETIME", "1979-05-27T07:32:00.999999Z")]

    def test_offset_datetime_space_separator(self) -> None:
        """Offset datetime with space instead of T separator."""
        pairs = token_pairs("1979-05-27 07:32:00Z")
        assert pairs == [("OFFSET_DATETIME", "1979-05-27 07:32:00Z")]

    def test_local_datetime(self) -> None:
        """Local datetime (no timezone)."""
        pairs = token_pairs("1979-05-27T07:32:00")
        assert pairs == [("LOCAL_DATETIME", "1979-05-27T07:32:00")]

    def test_local_datetime_with_fractional_seconds(self) -> None:
        """Local datetime with fractional seconds."""
        pairs = token_pairs("1979-05-27T07:32:00.123")
        assert pairs == [("LOCAL_DATETIME", "1979-05-27T07:32:00.123")]

    def test_local_datetime_space_separator(self) -> None:
        """Local datetime with space separator."""
        pairs = token_pairs("1979-05-27 07:32:00")
        assert pairs == [("LOCAL_DATETIME", "1979-05-27 07:32:00")]

    def test_local_date(self) -> None:
        """Local date (date only)."""
        pairs = token_pairs("1979-05-27")
        assert pairs == [("LOCAL_DATE", "1979-05-27")]

    def test_local_time(self) -> None:
        """Local time (time only)."""
        pairs = token_pairs("07:32:00")
        assert pairs == [("LOCAL_TIME", "07:32:00")]

    def test_local_time_with_fractional_seconds(self) -> None:
        """Local time with fractional seconds."""
        pairs = token_pairs("07:32:00.999")
        assert pairs == [("LOCAL_TIME", "07:32:00.999")]


# ---------------------------------------------------------------------------
# Bare key tests
# ---------------------------------------------------------------------------


class TestBareKeys:
    """Tests for TOML bare keys.

    Bare keys are unquoted key names composed of ASCII letters, digits,
    hyphens, and underscores. They can only appear as key names (the
    grammar ensures this), never as values.

    Because BARE_KEY is the last pattern in the grammar, it acts as a
    catch-all for anything that doesn't match a more specific pattern.
    """

    def test_simple_bare_key(self) -> None:
        """A simple alphabetic bare key."""
        pairs = token_pairs("server")
        assert pairs == [("BARE_KEY", "server")]

    def test_bare_key_with_hyphens(self) -> None:
        """A bare key containing hyphens."""
        pairs = token_pairs("my-key")
        assert pairs == [("BARE_KEY", "my-key")]

    def test_bare_key_with_underscores(self) -> None:
        """A bare key containing underscores."""
        pairs = token_pairs("my_key")
        assert pairs == [("BARE_KEY", "my_key")]

    def test_bare_key_alphanumeric(self) -> None:
        """A bare key with both letters and numbers."""
        pairs = token_pairs("key123")
        assert pairs == [("BARE_KEY", "key123")]

    def test_bare_key_all_chars(self) -> None:
        """A bare key with letters, digits, hyphens, and underscores."""
        pairs = token_pairs("my-key_123")
        assert pairs == [("BARE_KEY", "my-key_123")]


# ---------------------------------------------------------------------------
# Delimiter tests
# ---------------------------------------------------------------------------


class TestDelimiters:
    """Tests for TOML structural delimiters."""

    def test_equals(self) -> None:
        """The equals sign between key and value."""
        types = token_types("=")
        assert types == ["EQUALS"]

    def test_dot(self) -> None:
        """The dot in dotted keys."""
        types = token_types(".")
        assert types == ["DOT"]

    def test_comma(self) -> None:
        """The comma separating array/inline-table elements."""
        types = token_types(",")
        assert types == ["COMMA"]

    def test_brackets(self) -> None:
        """Square brackets for tables and arrays."""
        types = token_types("[]")
        assert types == ["LBRACKET", "RBRACKET"]

    def test_braces(self) -> None:
        """Curly braces for inline tables."""
        types = token_types("{}")
        assert types == ["LBRACE", "RBRACE"]


# ---------------------------------------------------------------------------
# Comment tests
# ---------------------------------------------------------------------------


class TestComments:
    """Tests for TOML comment handling.

    Comments in TOML start with ``#`` and run to the end of the line.
    The lexer skips them entirely — no COMMENT token is emitted. The
    trailing newline is NOT consumed by the comment skip pattern; it
    becomes a NEWLINE token.
    """

    def test_full_line_comment_skipped(self) -> None:
        """A full-line comment produces no content tokens."""
        types = token_types("# this is a comment")
        assert types == []

    def test_inline_comment_skipped(self) -> None:
        """An inline comment after a value is skipped."""
        types = token_types('key = "value" # comment')
        assert types == ["BARE_KEY", "EQUALS", "BASIC_STRING"]

    def test_comment_does_not_consume_newline(self) -> None:
        """The newline after a comment becomes a NEWLINE token.

        This is important because TOML uses newlines to delimit
        key-value pairs. If comments consumed their trailing newline,
        the grammar would not be able to detect line boundaries.
        """
        types = token_types("# comment\nkey = 1", keep_newlines=True)
        assert "NEWLINE" in types


# ---------------------------------------------------------------------------
# Newline tests
# ---------------------------------------------------------------------------


class TestNewlines:
    """Tests for NEWLINE token emission.

    TOML is newline-sensitive — key-value pairs are terminated by
    newlines. The lexer emits NEWLINE tokens for each ``\\n`` character.
    This is unlike JSON, where all whitespace (including newlines) is
    silently skipped.
    """

    def test_newline_emitted(self) -> None:
        """A newline produces a NEWLINE token."""
        types = token_types("a\nb", keep_newlines=True)
        assert "NEWLINE" in types

    def test_newline_between_pairs(self) -> None:
        """Newlines separate key-value pairs."""
        types = token_types("a = 1\nb = 2", keep_newlines=True)
        assert types == [
            "BARE_KEY", "EQUALS", "INTEGER",
            "NEWLINE",
            "BARE_KEY", "EQUALS", "INTEGER",
        ]

    def test_multiple_newlines(self) -> None:
        """Multiple consecutive newlines each produce a NEWLINE token."""
        types = token_types("a\n\nb", keep_newlines=True)
        newline_count = types.count("NEWLINE")
        assert newline_count == 2

    def test_trailing_newline(self) -> None:
        """A trailing newline produces a NEWLINE token."""
        types = token_types("a\n", keep_newlines=True)
        assert types == ["BARE_KEY", "NEWLINE"]


# ---------------------------------------------------------------------------
# Token ordering tests — the critical disambiguation cases
# ---------------------------------------------------------------------------


class TestTokenOrdering:
    """Tests that ambiguous inputs resolve to the correct token type.

    These tests verify the first-match-wins ordering in ``toml.tokens``.
    Because BARE_KEY matches almost everything (letters, digits, hyphens),
    more specific patterns must come first. Without careful ordering:

    - ``true`` would match BARE_KEY instead of TRUE
    - ``42`` would match BARE_KEY instead of INTEGER
    - ``3.14`` would match INTEGER(3) then DOT then BARE_KEY(14)
    - ``1979-05-27`` would match INTEGER then ... chaos
    - ``inf`` would match BARE_KEY instead of FLOAT

    These tests are the safety net for the grammar's pattern ordering.
    """

    def test_true_is_not_bare_key(self) -> None:
        """``true`` matches TRUE, not BARE_KEY."""
        pairs = token_pairs("true")
        assert pairs[0][0] == "TRUE"

    def test_false_is_not_bare_key(self) -> None:
        """``false`` matches FALSE, not BARE_KEY."""
        pairs = token_pairs("false")
        assert pairs[0][0] == "FALSE"

    def test_integer_is_not_bare_key(self) -> None:
        """``42`` matches INTEGER, not BARE_KEY."""
        pairs = token_pairs("42")
        assert pairs[0][0] == "INTEGER"

    def test_float_is_not_integer(self) -> None:
        """``3.14`` matches FLOAT, not INTEGER followed by DOT."""
        pairs = token_pairs("3.14")
        assert len(pairs) == 1
        assert pairs[0][0] == "FLOAT"

    def test_date_is_not_bare_key(self) -> None:
        """``1979-05-27`` matches LOCAL_DATE, not BARE_KEY."""
        pairs = token_pairs("1979-05-27")
        assert pairs[0][0] == "LOCAL_DATE"

    def test_inf_is_float_not_bare_key(self) -> None:
        """``inf`` matches FLOAT, not BARE_KEY."""
        pairs = token_pairs("inf")
        assert pairs[0][0] == "FLOAT"

    def test_nan_is_float_not_bare_key(self) -> None:
        """``nan`` matches FLOAT, not BARE_KEY."""
        pairs = token_pairs("nan")
        assert pairs[0][0] == "FLOAT"

    def test_time_is_not_integer_sequence(self) -> None:
        """``07:32:00`` matches LOCAL_TIME, not a sequence of integers."""
        pairs = token_pairs("07:32:00")
        assert len(pairs) == 1
        assert pairs[0][0] == "LOCAL_TIME"

    def test_datetime_is_not_date_plus_time(self) -> None:
        """``1979-05-27T07:32:00`` matches LOCAL_DATETIME as one token."""
        pairs = token_pairs("1979-05-27T07:32:00")
        assert len(pairs) == 1
        assert pairs[0][0] == "LOCAL_DATETIME"

    def test_offset_datetime_is_not_local_datetime(self) -> None:
        """``1979-05-27T07:32:00Z`` matches OFFSET_DATETIME, not LOCAL_DATETIME."""
        pairs = token_pairs("1979-05-27T07:32:00Z")
        assert len(pairs) == 1
        assert pairs[0][0] == "OFFSET_DATETIME"

    def test_ml_basic_before_basic(self) -> None:
        """Triple-double-quotes match ML_BASIC_STRING, not empty BASIC_STRING."""
        pairs = token_pairs('"""hello"""')
        assert len(pairs) == 1
        assert pairs[0][0] == "ML_BASIC_STRING"

    def test_ml_literal_before_literal(self) -> None:
        """Triple-single-quotes match ML_LITERAL_STRING, not empty LITERAL_STRING."""
        pairs = token_pairs("'''hello'''")
        assert len(pairs) == 1
        assert pairs[0][0] == "ML_LITERAL_STRING"

    def test_hex_integer_not_bare_key(self) -> None:
        """``0xFF`` matches INTEGER (via HEX_INTEGER alias), not BARE_KEY."""
        pairs = token_pairs("0xFF")
        assert pairs[0][0] == "INTEGER"

    def test_scientific_float_not_integer(self) -> None:
        """``1e10`` matches FLOAT, not INTEGER."""
        pairs = token_pairs("1e10")
        assert pairs[0][0] == "FLOAT"


# ---------------------------------------------------------------------------
# Position tracking tests
# ---------------------------------------------------------------------------


class TestPositionTracking:
    """Tests for line and column tracking in tokens.

    The lexer tracks the line and column where each token starts. This is
    critical for error messages — when the parser finds a syntax error, it
    needs to tell the user exactly where in the file the problem is.
    """

    def test_first_token_position(self) -> None:
        """The first token starts at line 1, column 1."""
        tokens = tokenize_toml("key")
        assert tokens[0].line == 1
        assert tokens[0].column == 1

    def test_second_line_position(self) -> None:
        """Tokens on the second line have line == 2."""
        tokens = tokenize_toml("a = 1\nb = 2")
        # Find the 'b' token
        b_tokens = [t for t in tokens if t.value == "b"]
        assert len(b_tokens) == 1
        assert b_tokens[0].line == 2

    def test_column_after_whitespace(self) -> None:
        """Column accounts for skipped whitespace."""
        tokens = tokenize_toml("key = 42")
        # The '42' token should be at column 7 (after 'key = ')
        int_tokens = [t for t in tokens if t.value == "42"]
        assert len(int_tokens) == 1
        assert int_tokens[0].column == 7

    def test_multiline_column_resets(self) -> None:
        """Column resets to 1 at the start of each new line."""
        tokens = tokenize_toml("a = 1\nb = 2")
        b_tokens = [t for t in tokens if t.value == "b"]
        assert b_tokens[0].column == 1


# ---------------------------------------------------------------------------
# Full TOML snippet tests
# ---------------------------------------------------------------------------


class TestFullSnippets:
    """Tests for tokenizing complete TOML structures.

    These integration tests verify that real-world TOML fragments
    tokenize correctly. They test the interactions between different
    token types, comments, newlines, and whitespace.
    """

    def test_key_value_pair(self) -> None:
        """A simple key = value pair."""
        types = token_types('name = "Tom"')
        assert types == ["BARE_KEY", "EQUALS", "BASIC_STRING"]

    def test_dotted_key(self) -> None:
        """A dotted key (nested table shorthand)."""
        types = token_types('server.port = 8080')
        assert types == [
            "BARE_KEY", "DOT", "BARE_KEY", "EQUALS", "INTEGER",
        ]

    def test_table_header(self) -> None:
        """A table header [section]."""
        types = token_types("[database]")
        assert types == ["LBRACKET", "BARE_KEY", "RBRACKET"]

    def test_array_of_tables(self) -> None:
        """An array-of-tables header [[items]]."""
        types = token_types("[[items]]")
        assert types == [
            "LBRACKET", "LBRACKET", "BARE_KEY", "RBRACKET", "RBRACKET",
        ]

    def test_inline_table(self) -> None:
        """An inline table value."""
        types = token_types('point = { x = 1, y = 2 }')
        assert types == [
            "BARE_KEY", "EQUALS",
            "LBRACE",
            "BARE_KEY", "EQUALS", "INTEGER", "COMMA",
            "BARE_KEY", "EQUALS", "INTEGER",
            "RBRACE",
        ]

    def test_array_value(self) -> None:
        """An array value."""
        types = token_types("ports = [8000, 8001, 8002]")
        assert types == [
            "BARE_KEY", "EQUALS",
            "LBRACKET",
            "INTEGER", "COMMA", "INTEGER", "COMMA", "INTEGER",
            "RBRACKET",
        ]

    def test_multiline_toml_document(self) -> None:
        """A multi-line TOML document with table, keys, and comments."""
        source = (
            "[server]\n"
            'host = "localhost"\n'
            "port = 8080\n"
            "# Enable debug mode\n"
            "debug = true\n"
        )
        types = token_types(source)
        assert types == [
            # [server]
            "LBRACKET", "BARE_KEY", "RBRACKET",
            # host = "localhost"
            "BARE_KEY", "EQUALS", "BASIC_STRING",
            # port = 8080
            "BARE_KEY", "EQUALS", "INTEGER",
            # # Enable debug mode (comment skipped)
            # debug = true
            "BARE_KEY", "EQUALS", "TRUE",
        ]

    def test_mixed_value_types(self) -> None:
        """A document with various value types."""
        source = (
            'title = "TOML"\n'
            "enabled = true\n"
            "pi = 3.14\n"
            "answer = 42\n"
            "birthday = 1979-05-27\n"
        )
        types = token_types(source)
        assert types == [
            "BARE_KEY", "EQUALS", "BASIC_STRING",
            "BARE_KEY", "EQUALS", "TRUE",
            "BARE_KEY", "EQUALS", "FLOAT",
            "BARE_KEY", "EQUALS", "INTEGER",
            "BARE_KEY", "EQUALS", "LOCAL_DATE",
        ]

    def test_quoted_keys(self) -> None:
        """Quoted keys (basic and literal string keys)."""
        types = token_types('"quoted-key" = 1')
        assert types == ["BASIC_STRING", "EQUALS", "INTEGER"]

    def test_literal_string_key(self) -> None:
        """A literal string used as a key."""
        types = token_types("'literal-key' = 1")
        assert types == ["LITERAL_STRING", "EQUALS", "INTEGER"]


# ---------------------------------------------------------------------------
# EOF token tests
# ---------------------------------------------------------------------------


class TestEOF:
    """Tests for the EOF token."""

    def test_always_ends_with_eof(self) -> None:
        """Token list always ends with EOF."""
        tokens = tokenize_toml("key = 42")
        assert _get_type_name(tokens[-1]) == "EOF"

    def test_empty_input_has_eof(self) -> None:
        """Empty input still produces an EOF token."""
        tokens = tokenize_toml("")
        assert len(tokens) == 1
        assert _get_type_name(tokens[0]) == "EOF"


# ---------------------------------------------------------------------------
# Error case tests
# ---------------------------------------------------------------------------


class TestErrors:
    """Tests for lexer error handling.

    The lexer should raise ``LexerError`` when it encounters characters
    that don't match any token pattern. This happens when the input
    contains invalid TOML syntax at the lexical level.
    """

    def test_unterminated_basic_string(self) -> None:
        """An unterminated basic string should raise LexerError.

        When the lexer encounters a ``"`` but never finds the closing
        ``"``, it cannot match the BASIC_STRING pattern. The ``"``
        character doesn't match any other pattern either, so the lexer
        raises LexerError.
        """
        with pytest.raises(LexerError):
            tokenize_toml('"unterminated')

    def test_unterminated_literal_string(self) -> None:
        """An unterminated literal string should raise LexerError."""
        with pytest.raises(LexerError):
            tokenize_toml("'unterminated")

    def test_invalid_character(self) -> None:
        """A character that matches no pattern should raise LexerError.

        The backslash character ``\\`` outside of a string doesn't match
        any TOML token pattern.
        """
        with pytest.raises(LexerError):
            tokenize_toml("\\")
