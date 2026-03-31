"""Tests for the JSON lexer thin wrapper.

These tests verify that the grammar-driven lexer, configured with
``json.tokens``, correctly tokenizes JSON text per RFC 8259.

Escape Sequence Testing Note
-----------------------------

The JSON grammar (``json.tokens``) uses ``escapes: none``, which instructs
the lexer to strip surrounding quotes from STRING tokens but leave escape
sequences (``\\n``, ``\\t``, ``\\uXXXX``, etc.) as raw text. This is by
design: the JSON *parser* is responsible for processing escape sequences,
not the lexer. The lexer's job is pure tokenization.

The ``TestStringEscapes`` class tests escape *processing* behaviour. It uses
a separate helper grammar that omits ``escapes: none``, so the
``GrammarLexer`` performs its default escape processing. This lets us verify
that the lexer *engine* handles JSON escape sequences correctly without
changing the semantics of ``json.tokens``.
"""

from __future__ import annotations

import pytest

from grammar_tools import parse_token_grammar
from json_lexer import create_json_lexer, tokenize_json
from lexer import GrammarLexer, Token


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def token_types(source: str) -> list[str]:
    """Tokenize and return just the type names (excluding EOF)."""
    tokens = tokenize_json(source)
    return [
        t.type if isinstance(t.type, str) else t.type.name
        for t in tokens
        if (t.type if isinstance(t.type, str) else t.type.name) != "EOF"
    ]


def token_values(source: str) -> list[str]:
    """Tokenize and return just the values (excluding EOF)."""
    tokens = tokenize_json(source)
    return [
        t.value
        for t in tokens
        if (t.type if isinstance(t.type, str) else t.type.name) != "EOF"
    ]


# ---------------------------------------------------------------------------
# Helper grammar for escape processing tests
# ---------------------------------------------------------------------------
#
# The real JSON grammar uses ``escapes: none`` because escape decoding is the
# parser's job. For the TestStringEscapes class we need a grammar that DOES
# process escapes so we can verify the lexer engine handles them correctly.
# This grammar is identical to json.tokens except it omits ``escapes: none``.

_ESCAPE_PROCESSING_GRAMMAR_SRC = r"""
STRING   = /"([^"\\]|\\["\\\x2fbfnrt]|\\u[0-9a-fA-F]{4})*"/
NUMBER   = /-?[0-9]+\.?[0-9]*[eE]?[-+]?[0-9]*/
TRUE     = "true"
FALSE    = "false"
NULL     = "null"
LBRACE   = "{"
RBRACE   = "}"
LBRACKET = "["
RBRACKET = "]"
COLON    = ":"
COMMA    = ","
skip:
  WHITESPACE = /[ \t\r\n]+/
"""


def _escape_grammar() -> object:
    """Return a TokenGrammar that processes escape sequences in strings."""
    return parse_token_grammar(_ESCAPE_PROCESSING_GRAMMAR_SRC)


def escape_token_values(source: str) -> list[str]:
    """Tokenize using the escape-processing grammar and return non-EOF values."""
    grammar = _escape_grammar()
    lexer = GrammarLexer(source, grammar)
    tokens = lexer.tokenize()
    return [
        t.value
        for t in tokens
        if (t.type if isinstance(t.type, str) else t.type.name) != "EOF"
    ]


# ---------------------------------------------------------------------------
# Factory function tests
# ---------------------------------------------------------------------------


class TestFactory:
    """Tests for the create_json_lexer factory function."""

    def test_returns_grammar_lexer(self) -> None:
        """create_json_lexer should return a GrammarLexer instance."""
        lexer = create_json_lexer("42")
        assert isinstance(lexer, GrammarLexer)

    def test_factory_produces_tokens(self) -> None:
        """The factory-created lexer should produce valid tokens."""
        lexer = create_json_lexer('"hello"')
        tokens = lexer.tokenize()
        assert len(tokens) >= 2  # STRING + EOF
        assert tokens[-1].type == "EOF" or tokens[-1].type.name == "EOF"


# ---------------------------------------------------------------------------
# Primitive value tests
# ---------------------------------------------------------------------------


class TestPrimitiveValues:
    """Tests for tokenizing JSON primitive values."""

    def test_string_simple(self) -> None:
        """A simple double-quoted string."""
        types = token_types('"hello"')
        assert types == ["STRING"]

    def test_string_value_strips_quotes(self) -> None:
        """String token value should not include surrounding quotes."""
        values = token_values('"hello"')
        assert values == ["hello"]

    def test_string_empty(self) -> None:
        """An empty string."""
        values = token_values('""')
        assert values == [""]

    def test_number_integer(self) -> None:
        """A simple integer."""
        types = token_types("42")
        values = token_values("42")
        assert types == ["NUMBER"]
        assert values == ["42"]

    def test_number_zero(self) -> None:
        """The number zero."""
        values = token_values("0")
        assert values == ["0"]

    def test_number_negative(self) -> None:
        """A negative number (minus is part of the NUMBER token)."""
        types = token_types("-42")
        values = token_values("-42")
        assert types == ["NUMBER"]
        assert values == ["-42"]

    def test_number_decimal(self) -> None:
        """A decimal number."""
        values = token_values("3.14")
        assert values == ["3.14"]

    def test_number_exponent(self) -> None:
        """A number with an exponent."""
        values = token_values("1e10")
        assert values == ["1e10"]

    def test_number_negative_exponent(self) -> None:
        """A number with a negative exponent."""
        values = token_values("2.5e-3")
        assert values == ["2.5e-3"]

    def test_number_uppercase_exponent(self) -> None:
        """A number with uppercase E in exponent."""
        values = token_values("1E10")
        assert values == ["1E10"]

    def test_number_exponent_plus(self) -> None:
        """A number with explicit positive exponent."""
        values = token_values("1e+5")
        assert values == ["1e+5"]

    def test_true(self) -> None:
        """The literal true."""
        types = token_types("true")
        values = token_values("true")
        assert types == ["TRUE"]
        assert values == ["true"]

    def test_false(self) -> None:
        """The literal false."""
        types = token_types("false")
        values = token_values("false")
        assert types == ["FALSE"]
        assert values == ["false"]

    def test_null(self) -> None:
        """The literal null."""
        types = token_types("null")
        values = token_values("null")
        assert types == ["NULL"]
        assert values == ["null"]


# ---------------------------------------------------------------------------
# String escape sequence tests
# ---------------------------------------------------------------------------


class TestStringEscapes:
    r"""Tests for JSON string escape sequences per RFC 8259 section 7.

    These tests use the ``escape_processing_grammar()`` helper (defined at
    module level) rather than the real JSON grammar. The real JSON grammar
    has ``escapes: none`` because escape decoding is the *parser's*
    responsibility. Here we want to test that the *lexer engine* correctly
    handles every JSON escape form, so we use a grammar without
    ``escapes: none``.
    """

    def test_escape_quote(self) -> None:
        r"""Escaped double quote: \" becomes "."""
        values = escape_token_values(r'"He said \"hi\""')
        assert values == ['He said "hi"']

    def test_escape_backslash(self) -> None:
        r"""Escaped backslash: \\ becomes \."""
        values = escape_token_values(r'"path\\to\\file"')
        assert values == ["path\\to\\file"]

    def test_escape_solidus(self) -> None:
        r"""Escaped solidus: \/ becomes /."""
        values = escape_token_values(r'"a\/b"')
        assert values == ["a/b"]

    def test_escape_backspace(self) -> None:
        r"""Escaped backspace: \b becomes backspace character."""
        values = escape_token_values(r'"a\bb"')
        assert values == ["a\bb"]

    def test_escape_form_feed(self) -> None:
        r"""Escaped form feed: \f becomes form feed character."""
        values = escape_token_values(r'"a\fb"')
        assert values == ["a\fb"]

    def test_escape_newline(self) -> None:
        r"""Escaped newline: \n becomes newline character."""
        values = escape_token_values(r'"line1\nline2"')
        assert values == ["line1\nline2"]

    def test_escape_carriage_return(self) -> None:
        r"""Escaped carriage return: \r becomes CR character."""
        values = escape_token_values(r'"a\rb"')
        assert values == ["a\rb"]

    def test_escape_tab(self) -> None:
        r"""Escaped tab: \t becomes tab character."""
        values = escape_token_values(r'"col1\tcol2"')
        assert values == ["col1\tcol2"]

    def test_escape_unicode(self) -> None:
        r"""Unicode escape: \u0041 becomes 'A'."""
        values = escape_token_values(r'"Hello \u0041"')
        assert values == ["Hello A"]

    def test_escape_unicode_non_ascii(self) -> None:
        r"""Unicode escape for non-ASCII: \u00E9 becomes 'e' with acute."""
        values = escape_token_values(r'"\u00E9"')
        assert values == ["\u00e9"]


# ---------------------------------------------------------------------------
# Structural token tests
# ---------------------------------------------------------------------------


class TestStructuralTokens:
    """Tests for JSON structural delimiters."""

    def test_empty_object(self) -> None:
        """Empty object {}."""
        types = token_types("{}")
        assert types == ["LBRACE", "RBRACE"]

    def test_empty_array(self) -> None:
        """Empty array []."""
        types = token_types("[]")
        assert types == ["LBRACKET", "RBRACKET"]

    def test_colon(self) -> None:
        """Colon between key and value."""
        types = token_types('"key": "value"')
        assert types == ["STRING", "COLON", "STRING"]

    def test_comma(self) -> None:
        """Comma between elements."""
        types = token_types("1, 2, 3")
        assert types == ["NUMBER", "COMMA", "NUMBER", "COMMA", "NUMBER"]


# ---------------------------------------------------------------------------
# Whitespace handling tests
# ---------------------------------------------------------------------------


class TestWhitespace:
    """Tests for whitespace handling (all whitespace is skipped)."""

    def test_spaces(self) -> None:
        """Spaces between tokens are skipped."""
        types = token_types('{ "a" : 1 }')
        assert types == ["LBRACE", "STRING", "COLON", "NUMBER", "RBRACE"]

    def test_tabs(self) -> None:
        """Tabs between tokens are skipped."""
        types = token_types('{\t"a"\t:\t1\t}')
        assert types == ["LBRACE", "STRING", "COLON", "NUMBER", "RBRACE"]

    def test_newlines(self) -> None:
        """Newlines between tokens are skipped (no NEWLINE tokens)."""
        types = token_types('{\n"a":\n1\n}')
        assert types == ["LBRACE", "STRING", "COLON", "NUMBER", "RBRACE"]

    def test_carriage_returns(self) -> None:
        """Carriage returns between tokens are skipped."""
        types = token_types('{\r\n"a":\r\n1\r\n}')
        assert types == ["LBRACE", "STRING", "COLON", "NUMBER", "RBRACE"]

    def test_mixed_whitespace(self) -> None:
        """Mixed whitespace is all skipped."""
        types = token_types('{ \t\n\r "a" \t\n : \r\n 1 \t }')
        assert types == ["LBRACE", "STRING", "COLON", "NUMBER", "RBRACE"]


# ---------------------------------------------------------------------------
# Compound structure tests
# ---------------------------------------------------------------------------


class TestCompoundStructures:
    """Tests for tokenizing complete JSON structures."""

    def test_simple_object(self) -> None:
        """A simple key-value object."""
        types = token_types('{"name": "Ada"}')
        assert types == [
            "LBRACE", "STRING", "COLON", "STRING", "RBRACE",
        ]

    def test_object_multiple_pairs(self) -> None:
        """An object with multiple key-value pairs."""
        types = token_types('{"a": 1, "b": 2}')
        assert types == [
            "LBRACE",
            "STRING", "COLON", "NUMBER", "COMMA",
            "STRING", "COLON", "NUMBER",
            "RBRACE",
        ]

    def test_simple_array(self) -> None:
        """A simple array of numbers."""
        types = token_types("[1, 2, 3]")
        assert types == [
            "LBRACKET",
            "NUMBER", "COMMA", "NUMBER", "COMMA", "NUMBER",
            "RBRACKET",
        ]

    def test_nested_object_in_array(self) -> None:
        """Nested object inside an array."""
        types = token_types('[{"a": 1}]')
        assert types == [
            "LBRACKET",
            "LBRACE", "STRING", "COLON", "NUMBER", "RBRACE",
            "RBRACKET",
        ]

    def test_nested_array_in_object(self) -> None:
        """Nested array inside an object."""
        types = token_types('{"list": [1, 2]}')
        assert types == [
            "LBRACE",
            "STRING", "COLON",
            "LBRACKET", "NUMBER", "COMMA", "NUMBER", "RBRACKET",
            "RBRACE",
        ]

    def test_mixed_value_types(self) -> None:
        """An array with all JSON value types."""
        types = token_types('[42, "hi", true, false, null]')
        assert types == [
            "LBRACKET",
            "NUMBER", "COMMA",
            "STRING", "COMMA",
            "TRUE", "COMMA",
            "FALSE", "COMMA",
            "NULL",
            "RBRACKET",
        ]


# ---------------------------------------------------------------------------
# Position tracking tests
# ---------------------------------------------------------------------------


class TestPositionTracking:
    """Tests for line and column tracking."""

    def test_single_line_positions(self) -> None:
        """Tokens on a single line have correct column numbers."""
        tokens = tokenize_json("[1, 2]")
        # Filter out EOF
        non_eof = [t for t in tokens if not (
            t.type == "EOF" or (hasattr(t.type, "name") and t.type.name == "EOF")
        )]
        assert non_eof[0].line == 1  # [
        assert non_eof[0].column == 1

    def test_multiline_positions(self) -> None:
        """Tokens across multiple lines have correct line numbers."""
        source = '{\n  "a": 1\n}'
        tokens = tokenize_json(source)
        non_eof = [t for t in tokens if not (
            t.type == "EOF" or (hasattr(t.type, "name") and t.type.name == "EOF")
        )]
        # { is on line 1
        assert non_eof[0].line == 1
        # "a" is on line 2
        assert non_eof[1].line == 2


# ---------------------------------------------------------------------------
# EOF token test
# ---------------------------------------------------------------------------


class TestEOF:
    """Tests for the EOF token."""

    def test_always_ends_with_eof(self) -> None:
        """Token list always ends with EOF."""
        tokens = tokenize_json("42")
        last = tokens[-1]
        eof_name = last.type if isinstance(last.type, str) else last.type.name
        assert eof_name == "EOF"

    def test_empty_input_has_eof(self) -> None:
        """Empty input still produces an EOF token."""
        tokens = tokenize_json("")
        assert len(tokens) == 1
        eof_name = tokens[0].type if isinstance(tokens[0].type, str) else tokens[0].type.name
        assert eof_name == "EOF"
