"""Tests for the Lattice tokenizer.

These tests verify that the Lattice tokenizer correctly handles all CSS
tokens (true superset) plus the 5 new Lattice tokens: VARIABLE,
EQUALS_EQUALS, NOT_EQUALS, GREATER_EQUALS, LESS_EQUALS.
"""

from __future__ import annotations

from lattice_lexer import __version__, create_lattice_lexer, tokenize_lattice


# ---------------------------------------------------------------------------
# Helper Functions
# ---------------------------------------------------------------------------


def _type_name(token: object) -> str:
    """Get the string name of a token's type.

    The Token class uses ``.type`` which can be either a string (from
    GrammarLexer) or a ``TokenType`` enum (from the hand-written lexer).
    """
    t = token.type  # type: ignore[attr-defined]
    if isinstance(t, str):
        return t
    return t.name


def _types(source: str) -> list[str]:
    """Tokenize source and return just the token type names (no EOF)."""
    tokens = tokenize_lattice(source)
    return [_type_name(t) for t in tokens if _type_name(t) != "EOF"]


def _pairs(source: str) -> list[tuple[str, str]]:
    """Tokenize source and return (type, value) pairs (no EOF)."""
    tokens = tokenize_lattice(source)
    return [
        (_type_name(t), t.value)
        for t in tokens
        if _type_name(t) != "EOF"
    ]


# ===========================================================================
# Factory Function Tests
# ===========================================================================


class TestFactory:
    """Test that factory functions return correct types."""

    def test_version_exists(self) -> None:
        """Package has a version."""
        assert __version__ == "0.1.0"

    def test_create_lattice_lexer_returns_lexer(self) -> None:
        """create_lattice_lexer returns a GrammarLexer instance."""
        lexer = create_lattice_lexer("h1 { color: red; }")
        assert hasattr(lexer, "tokenize")

    def test_tokenize_lattice_returns_list(self) -> None:
        """tokenize_lattice returns a list of tokens."""
        tokens = tokenize_lattice("h1 { color: red; }")
        assert isinstance(tokens, list)
        assert len(tokens) > 0

    def test_eof_token(self) -> None:
        """Token list ends with EOF."""
        tokens = tokenize_lattice("")
        assert len(tokens) == 1
        assert _type_name(tokens[0]) == "EOF"


# ===========================================================================
# Variable Token Tests (NEW — Lattice extension)
# ===========================================================================


class TestVariableToken:
    """Test the VARIABLE token type — the primary Lattice extension."""

    def test_simple_variable(self) -> None:
        """$color is tokenized as VARIABLE."""
        assert _pairs("$color") == [("VARIABLE", "$color")]

    def test_variable_with_hyphens(self) -> None:
        """$font-size is a single VARIABLE token."""
        assert _pairs("$font-size") == [("VARIABLE", "$font-size")]

    def test_variable_with_underscores(self) -> None:
        """$_private is a valid VARIABLE."""
        assert _pairs("$_private") == [("VARIABLE", "$_private")]

    def test_variable_with_digits(self) -> None:
        """$color2 is a valid VARIABLE."""
        assert _pairs("$color2") == [("VARIABLE", "$color2")]

    def test_variable_in_declaration(self) -> None:
        """$color: red; tokenizes correctly."""
        assert _types("$color: red;") == [
            "VARIABLE", "COLON", "IDENT", "SEMICOLON",
        ]

    def test_variable_in_value(self) -> None:
        """color: $brand; tokenizes correctly."""
        assert _types("color: $brand;") == [
            "IDENT", "COLON", "VARIABLE", "SEMICOLON",
        ]

    def test_dollar_equals_not_variable(self) -> None:
        """$= is DOLLAR_EQUALS, not VARIABLE.

        The VARIABLE regex requires a letter/underscore after $, so $=
        does not match VARIABLE. It falls through to DOLLAR_EQUALS.
        """
        assert _pairs("$=") == [("DOLLAR_EQUALS", "$=")]


# ===========================================================================
# Comparison Operator Tests (NEW — Lattice extension)
# ===========================================================================


class TestComparisonOperators:
    """Test the comparison operator tokens — Lattice extensions for @if."""

    def test_equals_equals(self) -> None:
        """== is EQUALS_EQUALS, not two EQUALS tokens."""
        assert _pairs("==") == [("EQUALS_EQUALS", "==")]

    def test_not_equals(self) -> None:
        """!= is NOT_EQUALS, not BANG + EQUALS."""
        assert _pairs("!=") == [("NOT_EQUALS", "!=")]

    def test_greater_equals(self) -> None:
        """>= is GREATER_EQUALS, not GREATER + EQUALS."""
        assert _pairs(">=") == [("GREATER_EQUALS", ">=")]

    def test_less_equals(self) -> None:
        """<= is LESS_EQUALS, not two separate tokens."""
        assert _pairs("<=") == [("LESS_EQUALS", "<=")]

    def test_single_greater(self) -> None:
        """> alone is still GREATER."""
        assert _pairs(">") == [("GREATER", ">")]

    def test_single_equals(self) -> None:
        """= alone is still EQUALS."""
        assert _pairs("=") == [("EQUALS", "=")]

    def test_single_bang(self) -> None:
        """! alone is still BANG."""
        assert _pairs("!") == [("BANG", "!")]

    def test_comparison_in_expression(self) -> None:
        """$x == 10 tokenizes as VARIABLE EQUALS_EQUALS NUMBER."""
        assert _types("$x == 10") == [
            "VARIABLE", "EQUALS_EQUALS", "NUMBER",
        ]


# ===========================================================================
# CSS Token Passthrough Tests
# ===========================================================================


class TestCSSPassthrough:
    """Verify all standard CSS tokens still work correctly."""

    def test_ident(self) -> None:
        """CSS identifiers tokenize as IDENT."""
        assert _pairs("color") == [("IDENT", "color")]

    def test_number(self) -> None:
        """Bare numbers tokenize as NUMBER."""
        assert _pairs("42") == [("NUMBER", "42")]

    def test_dimension(self) -> None:
        """Numbers with units tokenize as DIMENSION."""
        assert _pairs("10px") == [("DIMENSION", "10px")]

    def test_percentage(self) -> None:
        """Numbers with % tokenize as PERCENTAGE."""
        assert _pairs("50%") == [("PERCENTAGE", "50%")]

    def test_string_double(self) -> None:
        """Double-quoted strings tokenize as STRING."""
        assert _pairs('"hello"') == [("STRING", "hello")]

    def test_string_single(self) -> None:
        """Single-quoted strings tokenize as STRING."""
        assert _pairs("'world'") == [("STRING", "world")]

    def test_hash(self) -> None:
        """Hash values tokenize as HASH."""
        assert _pairs("#fff") == [("HASH", "#fff")]

    def test_at_keyword(self) -> None:
        """At-keywords tokenize as AT_KEYWORD."""
        assert _pairs("@media") == [("AT_KEYWORD", "@media")]

    def test_function(self) -> None:
        """Function tokens include the opening paren."""
        assert _pairs("rgb(") == [("FUNCTION", "rgb(")]

    def test_custom_property(self) -> None:
        """CSS custom properties tokenize as CUSTOM_PROPERTY."""
        assert _pairs("--main-color") == [("CUSTOM_PROPERTY", "--main-color")]

    def test_url_token(self) -> None:
        """url() with unquoted content is a single URL_TOKEN."""
        assert _pairs("url(path.png)") == [("URL_TOKEN", "url(path.png)")]


# ===========================================================================
# Lattice At-Keyword Tests
# ===========================================================================


class TestLatticeAtKeywords:
    """Test that Lattice at-keywords tokenize as AT_KEYWORD."""

    def test_mixin(self) -> None:
        """@mixin is AT_KEYWORD."""
        assert _pairs("@mixin") == [("AT_KEYWORD", "@mixin")]

    def test_include(self) -> None:
        """@include is AT_KEYWORD."""
        assert _pairs("@include") == [("AT_KEYWORD", "@include")]

    def test_if(self) -> None:
        """@if is AT_KEYWORD."""
        assert _pairs("@if") == [("AT_KEYWORD", "@if")]

    def test_else(self) -> None:
        """@else is AT_KEYWORD."""
        assert _pairs("@else") == [("AT_KEYWORD", "@else")]

    def test_for(self) -> None:
        """@for is AT_KEYWORD."""
        assert _pairs("@for") == [("AT_KEYWORD", "@for")]

    def test_each(self) -> None:
        """@each is AT_KEYWORD."""
        assert _pairs("@each") == [("AT_KEYWORD", "@each")]

    def test_function_keyword(self) -> None:
        """@function is AT_KEYWORD."""
        assert _pairs("@function") == [("AT_KEYWORD", "@function")]

    def test_return(self) -> None:
        """@return is AT_KEYWORD."""
        assert _pairs("@return") == [("AT_KEYWORD", "@return")]

    def test_use(self) -> None:
        """@use is AT_KEYWORD."""
        assert _pairs("@use") == [("AT_KEYWORD", "@use")]


# ===========================================================================
# Comment Tests
# ===========================================================================


class TestComments:
    """Test comment handling — CSS block comments and Lattice line comments."""

    def test_block_comment_skipped(self) -> None:
        """/* ... */ comments are skipped."""
        assert _types("/* comment */ color") == ["IDENT"]

    def test_line_comment_skipped(self) -> None:
        """// line comments are skipped (Lattice extension)."""
        assert _types("// comment\ncolor") == ["IDENT"]

    def test_line_comment_doesnt_eat_next_line(self) -> None:
        """// comment only extends to end of line."""
        assert _types("// comment\ncolor: red;") == [
            "IDENT", "COLON", "IDENT", "SEMICOLON",
        ]


# ===========================================================================
# Full Lattice Source Tests
# ===========================================================================


class TestFullSource:
    """Test tokenization of realistic Lattice source fragments."""

    def test_variable_declaration(self) -> None:
        """$color: #4a90d9;"""
        assert _types("$color: #4a90d9;") == [
            "VARIABLE", "COLON", "HASH", "SEMICOLON",
        ]

    def test_mixin_definition_header(self) -> None:
        """@mixin button($bg, $fg: white) {

        Note: button( is a FUNCTION token (identifier + open paren),
        following CSS tokenization rules where rgb( → FUNCTION.
        """
        assert _types("@mixin button($bg, $fg: white) {") == [
            "AT_KEYWORD", "FUNCTION",
            "VARIABLE", "COMMA",
            "VARIABLE", "COLON", "IDENT", "RPAREN",
            "LBRACE",
        ]

    def test_include_call(self) -> None:
        """@include button(red);

        button( → FUNCTION token, just like rgb( or calc(.
        """
        assert _types("@include button(red);") == [
            "AT_KEYWORD", "FUNCTION",
            "IDENT", "RPAREN",
            "SEMICOLON",
        ]

    def test_if_expression(self) -> None:
        """@if $theme == dark {"""
        assert _types("@if $theme == dark {") == [
            "AT_KEYWORD", "VARIABLE",
            "EQUALS_EQUALS", "IDENT",
            "LBRACE",
        ]

    def test_for_loop(self) -> None:
        """@for $i from 1 through 12 {"""
        assert _types("@for $i from 1 through 12 {") == [
            "AT_KEYWORD", "VARIABLE",
            "IDENT", "NUMBER",
            "IDENT", "NUMBER",
            "LBRACE",
        ]

    def test_function_return(self) -> None:
        """@return $value * 2;"""
        assert _types("@return $value * 2;") == [
            "AT_KEYWORD", "VARIABLE",
            "STAR", "NUMBER",
            "SEMICOLON",
        ]

    def test_use_directive(self) -> None:
        """@use "colors" as c;"""
        assert _types('@use "colors" as c;') == [
            "AT_KEYWORD", "STRING",
            "IDENT", "IDENT",
            "SEMICOLON",
        ]

    def test_plain_css_passthrough(self) -> None:
        """Plain CSS tokenizes correctly through Lattice lexer."""
        types = _types("h1 { color: red; font-size: 2em; }")
        assert types == [
            "IDENT", "LBRACE",
            "IDENT", "COLON", "IDENT", "SEMICOLON",
            "IDENT", "COLON", "DIMENSION", "SEMICOLON",
            "RBRACE",
        ]
