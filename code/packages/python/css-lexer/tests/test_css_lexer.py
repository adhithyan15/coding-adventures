"""Tests for the CSS Lexer — comprehensive stress test of grammar-driven tokenization.

These tests verify that the CSS lexer correctly handles the diverse and complex
token types defined by CSS3. The tests are organized by category, targeting the
specific challenges that make CSS tokenization harder than JSON or Starlark.

The primary stress tests are:
- Compound tokens: DIMENSION vs PERCENTAGE vs NUMBER disambiguation
- Function tokens: URL_TOKEN vs FUNCTION vs IDENT priority ordering
- Multi-line comments in skip patterns
- Error tokens for malformed input
- Custom properties and vendor prefixes
"""

from __future__ import annotations

import pytest

from css_lexer import tokenize_css


def _types(source: str) -> list[str]:
    """Tokenize and return just the type names (without EOF)."""
    tokens = tokenize_css(source)
    return [
        (t.type if isinstance(t.type, str) else t.type.name)
        for t in tokens
        if (t.type if isinstance(t.type, str) else t.type.name) != "EOF"
    ]


def _type_value_pairs(source: str) -> list[tuple[str, str]]:
    """Tokenize and return (type, value) pairs (without EOF)."""
    tokens = tokenize_css(source)
    return [
        (t.type if isinstance(t.type, str) else t.type.name, t.value)
        for t in tokens
        if (t.type if isinstance(t.type, str) else t.type.name) != "EOF"
    ]


# ---------------------------------------------------------------------------
# Basic tokens
# ---------------------------------------------------------------------------


class TestBasicTokens:
    """Test fundamental token types."""

    def test_identifier(self) -> None:
        """Simple CSS identifier."""
        pairs = _type_value_pairs("color")
        assert pairs == [("IDENT", "color")]

    def test_number_integer(self) -> None:
        """Integer number."""
        pairs = _type_value_pairs("42")
        assert pairs == [("NUMBER", "42")]

    def test_number_decimal(self) -> None:
        """Decimal number."""
        pairs = _type_value_pairs("3.14")
        assert pairs == [("NUMBER", "3.14")]

    def test_number_leading_dot(self) -> None:
        r"""Number with leading dot — the lexer's NUMBER regex matches .5
        as a single token because [0-9]* allows empty, then \.? matches
        the dot, then [0-9]+ matches 5."""
        pairs = _type_value_pairs(".5")
        assert pairs == [("NUMBER", ".5")]

    def test_number_scientific(self) -> None:
        """Scientific notation."""
        pairs = _type_value_pairs("1e10")
        # 1e10 matches DIMENSION (number + letters)
        assert pairs[0][0] == "DIMENSION"

    def test_number_negative(self) -> None:
        """Negative number."""
        pairs = _type_value_pairs("-42")
        assert pairs == [("NUMBER", "-42")]

    def test_string_double_quoted(self) -> None:
        """Double-quoted string."""
        pairs = _type_value_pairs('"hello"')
        assert pairs == [("STRING", "hello")]

    def test_string_single_quoted(self) -> None:
        """Single-quoted string."""
        pairs = _type_value_pairs("'world'")
        assert pairs == [("STRING", "world")]

    def test_string_escape_preserved(self) -> None:
        """Escape sequences are preserved raw (escapes: none mode)."""
        pairs = _type_value_pairs(r'"hello\nworld"')
        # With escapes: none, \n is NOT converted to newline
        assert pairs == [("STRING", "hello\\nworld")]

    def test_hash_color(self) -> None:
        """Hash token as color."""
        pairs = _type_value_pairs("#fff")
        assert pairs == [("HASH", "#fff")]

    def test_hash_long_color(self) -> None:
        """6-digit hex color."""
        pairs = _type_value_pairs("#336699")
        assert pairs == [("HASH", "#336699")]

    def test_hash_id_selector(self) -> None:
        """Hash token as ID selector."""
        pairs = _type_value_pairs("#header")
        assert pairs == [("HASH", "#header")]

    def test_at_keyword(self) -> None:
        """At-keyword token."""
        pairs = _type_value_pairs("@media")
        assert pairs == [("AT_KEYWORD", "@media")]

    def test_at_keyword_import(self) -> None:
        """@import at-keyword."""
        pairs = _type_value_pairs("@import")
        assert pairs == [("AT_KEYWORD", "@import")]

    def test_at_keyword_keyframes(self) -> None:
        """@keyframes at-keyword."""
        pairs = _type_value_pairs("@keyframes")
        assert pairs == [("AT_KEYWORD", "@keyframes")]

    def test_at_keyword_vendor_prefix(self) -> None:
        """Vendor-prefixed at-keyword."""
        pairs = _type_value_pairs("@-webkit-keyframes")
        assert pairs == [("AT_KEYWORD", "@-webkit-keyframes")]


# ---------------------------------------------------------------------------
# Compound tokens — the primary stress test
# ---------------------------------------------------------------------------


class TestCompoundTokens:
    """Test DIMENSION, PERCENTAGE, NUMBER priority ordering.

    This is the most important test category. CSS compound tokens require
    correct first-match-wins ordering: DIMENSION > PERCENTAGE > NUMBER.
    If the ordering is wrong, "10px" would tokenize as NUMBER + IDENT.
    """

    def test_dimension_px(self) -> None:
        """10px is a single DIMENSION token, not NUMBER + IDENT."""
        pairs = _type_value_pairs("10px")
        assert pairs == [("DIMENSION", "10px")]

    def test_dimension_em(self) -> None:
        pairs = _type_value_pairs("2em")
        assert pairs == [("DIMENSION", "2em")]

    def test_dimension_rem(self) -> None:
        pairs = _type_value_pairs("1.5rem")
        assert pairs == [("DIMENSION", "1.5rem")]

    def test_dimension_vh(self) -> None:
        pairs = _type_value_pairs("100vh")
        assert pairs == [("DIMENSION", "100vh")]

    def test_dimension_vw(self) -> None:
        pairs = _type_value_pairs("50vw")
        assert pairs == [("DIMENSION", "50vw")]

    def test_dimension_deg(self) -> None:
        """Angle unit."""
        pairs = _type_value_pairs("45deg")
        assert pairs == [("DIMENSION", "45deg")]

    def test_dimension_ms(self) -> None:
        """Time unit."""
        pairs = _type_value_pairs("300ms")
        assert pairs == [("DIMENSION", "300ms")]

    def test_dimension_negative(self) -> None:
        """Negative dimension."""
        pairs = _type_value_pairs("-20px")
        assert pairs == [("DIMENSION", "-20px")]

    def test_dimension_decimal(self) -> None:
        """Decimal dimension."""
        pairs = _type_value_pairs("0.5em")
        assert pairs == [("DIMENSION", "0.5em")]

    def test_percentage(self) -> None:
        """50% is PERCENTAGE, not NUMBER + literal %."""
        pairs = _type_value_pairs("50%")
        assert pairs == [("PERCENTAGE", "50%")]

    def test_percentage_decimal(self) -> None:
        pairs = _type_value_pairs("33.3%")
        assert pairs == [("PERCENTAGE", "33.3%")]

    def test_percentage_negative(self) -> None:
        pairs = _type_value_pairs("-10%")
        assert pairs == [("PERCENTAGE", "-10%")]

    def test_number_then_space_then_ident(self) -> None:
        """Separated number and identifier are two tokens."""
        pairs = _type_value_pairs("10 px")
        assert pairs == [("NUMBER", "10"), ("IDENT", "px")]

    def test_multiple_dimensions(self) -> None:
        """Multiple dimensions in a shorthand property value."""
        types = _types("10px 20px 30px")
        assert types == ["DIMENSION", "DIMENSION", "DIMENSION"]


# ---------------------------------------------------------------------------
# Function tokens
# ---------------------------------------------------------------------------


class TestFunctionTokens:
    """Test FUNCTION and URL_TOKEN tokenization."""

    def test_function_rgb(self) -> None:
        """rgb( is a single FUNCTION token."""
        pairs = _type_value_pairs("rgb(")
        assert pairs == [("FUNCTION", "rgb(")]

    def test_function_calc(self) -> None:
        pairs = _type_value_pairs("calc(")
        assert pairs == [("FUNCTION", "calc(")]

    def test_function_var(self) -> None:
        pairs = _type_value_pairs("var(")
        assert pairs == [("FUNCTION", "var(")]

    def test_function_linear_gradient(self) -> None:
        pairs = _type_value_pairs("linear-gradient(")
        assert pairs == [("FUNCTION", "linear-gradient(")]

    def test_function_vendor_prefix(self) -> None:
        """Vendor-prefixed function."""
        pairs = _type_value_pairs("-webkit-calc(")
        assert pairs == [("FUNCTION", "-webkit-calc(")]

    def test_url_token_unquoted(self) -> None:
        """url() with unquoted content is a single URL_TOKEN."""
        pairs = _type_value_pairs("url(path/to/file.png)")
        assert pairs == [("URL_TOKEN", "url(path/to/file.png)")]

    def test_url_token_before_function(self) -> None:
        """URL_TOKEN takes priority over FUNCTION for url(."""
        pairs = _type_value_pairs("url(image.jpg)")
        assert pairs[0] == ("URL_TOKEN", "url(image.jpg)")

    def test_function_with_args(self) -> None:
        """Complete function call tokenization."""
        types = _types("rgb(255, 0, 0)")
        assert types == ["FUNCTION", "NUMBER", "COMMA", "NUMBER",
                         "COMMA", "NUMBER", "RPAREN"]


# ---------------------------------------------------------------------------
# Operators and delimiters
# ---------------------------------------------------------------------------


class TestOperatorsAndDelimiters:
    """Test operator priority ordering and delimiters."""

    def test_double_colon(self) -> None:
        """:: is COLON_COLON, not two COLON tokens."""
        pairs = _type_value_pairs("::")
        assert pairs == [("COLON_COLON", "::")]

    def test_single_colon(self) -> None:
        pairs = _type_value_pairs(":")
        assert pairs == [("COLON", ":")]

    def test_tilde_equals(self) -> None:
        """~= is TILDE_EQUALS, not TILDE + EQUALS."""
        pairs = _type_value_pairs("~=")
        assert pairs == [("TILDE_EQUALS", "~=")]

    def test_pipe_equals(self) -> None:
        pairs = _type_value_pairs("|=")
        assert pairs == [("PIPE_EQUALS", "|=")]

    def test_caret_equals(self) -> None:
        pairs = _type_value_pairs("^=")
        assert pairs == [("CARET_EQUALS", "^=")]

    def test_dollar_equals(self) -> None:
        pairs = _type_value_pairs("$=")
        assert pairs == [("DOLLAR_EQUALS", "$=")]

    def test_star_equals(self) -> None:
        pairs = _type_value_pairs("*=")
        assert pairs == [("STAR_EQUALS", "*=")]

    def test_all_delimiters(self) -> None:
        """All single-character delimiters."""
        types = _types("{ } ( ) [ ] ; : , . + > ~ * | ! / = &")
        assert types == [
            "LBRACE", "RBRACE", "LPAREN", "RPAREN",
            "LBRACKET", "RBRACKET", "SEMICOLON", "COLON",
            "COMMA", "DOT", "PLUS", "GREATER", "TILDE",
            "STAR", "PIPE", "BANG", "SLASH", "EQUALS", "AMPERSAND",
        ]


# ---------------------------------------------------------------------------
# Comments
# ---------------------------------------------------------------------------


class TestComments:
    """Test multi-line comment handling in skip patterns."""

    def test_single_line_comment(self) -> None:
        """Comments are skipped."""
        types = _types("color /* override */ red")
        assert types == ["IDENT", "IDENT"]

    def test_multi_line_comment(self) -> None:
        """Multi-line comments span newlines."""
        types = _types("color /* line1\nline2\nline3 */ red")
        assert types == ["IDENT", "IDENT"]

    def test_comment_between_tokens(self) -> None:
        types = _types("h1 /* selector */ { /* block */ }")
        assert types == ["IDENT", "LBRACE", "RBRACE"]

    def test_adjacent_comments(self) -> None:
        types = _types("/* a */ /* b */ hello")
        assert types == ["IDENT"]


# ---------------------------------------------------------------------------
# Legacy tokens (CDO/CDC)
# ---------------------------------------------------------------------------


class TestLegacyTokens:
    """Test CDO and CDC (legacy HTML comment delimiters)."""

    def test_cdo(self) -> None:
        pairs = _type_value_pairs("<!--")
        assert pairs == [("CDO", "<!--")]

    def test_cdc(self) -> None:
        pairs = _type_value_pairs("-->")
        assert pairs == [("CDC", "-->")]


# ---------------------------------------------------------------------------
# Custom properties and vendor prefixes
# ---------------------------------------------------------------------------


class TestCustomPropertiesAndPrefixes:
    """Test CSS custom properties (--var) and vendor prefixes (-webkit-)."""

    def test_custom_property(self) -> None:
        """--main-color is CUSTOM_PROPERTY, not IDENT."""
        pairs = _type_value_pairs("--main-color")
        assert pairs == [("CUSTOM_PROPERTY", "--main-color")]

    def test_custom_property_simple(self) -> None:
        pairs = _type_value_pairs("--bg")
        assert pairs == [("CUSTOM_PROPERTY", "--bg")]

    def test_vendor_prefix_ident(self) -> None:
        """Vendor-prefixed identifiers are IDENT."""
        pairs = _type_value_pairs("-webkit-transform")
        assert pairs == [("IDENT", "-webkit-transform")]

    def test_vendor_prefix_moz(self) -> None:
        pairs = _type_value_pairs("-moz-user-select")
        assert pairs == [("IDENT", "-moz-user-select")]


# ---------------------------------------------------------------------------
# Unicode range
# ---------------------------------------------------------------------------


class TestUnicodeRange:
    """Test unicode-range token."""

    def test_unicode_range_simple(self) -> None:
        pairs = _type_value_pairs("U+0025")
        assert pairs == [("UNICODE_RANGE", "U+0025")]

    def test_unicode_range_with_range(self) -> None:
        pairs = _type_value_pairs("U+0025-00FF")
        assert pairs == [("UNICODE_RANGE", "U+0025-00FF")]

    def test_unicode_range_wildcards(self) -> None:
        pairs = _type_value_pairs("U+4??")
        assert pairs == [("UNICODE_RANGE", "U+4??")]


# ---------------------------------------------------------------------------
# Error tokens
# ---------------------------------------------------------------------------


class TestErrorTokens:
    """Test error token recovery for malformed CSS input."""

    def test_bad_string_unclosed(self) -> None:
        """Unclosed double-quoted string produces BAD_STRING."""
        pairs = _type_value_pairs('"unclosed string')
        assert pairs[0][0] == "BAD_STRING"

    def test_bad_url_unclosed(self) -> None:
        """Unclosed url() with unquoted content — when the URL_TOKEN regex
        doesn't match (because there's no closing paren) and FUNCTION
        consumes url(, the remaining content is just IDENT. BAD_URL only
        matches when url(...$ reaches end of input as a single match."""
        # url( matches FUNCTION, then unclosed matches IDENT
        pairs = _type_value_pairs("url(unclosed")
        assert pairs[0] == ("FUNCTION", "url(")
        assert pairs[1] == ("IDENT", "unclosed")


# ---------------------------------------------------------------------------
# Whitespace handling
# ---------------------------------------------------------------------------


class TestWhitespace:
    """Test that whitespace (including newlines) is properly skipped."""

    def test_spaces_skipped(self) -> None:
        types = _types("h1   h2")
        assert types == ["IDENT", "IDENT"]

    def test_tabs_skipped(self) -> None:
        types = _types("h1\th2")
        assert types == ["IDENT", "IDENT"]

    def test_newlines_skipped(self) -> None:
        """Newlines are included in whitespace skip (no NEWLINE tokens)."""
        types = _types("h1\nh2")
        assert types == ["IDENT", "IDENT"]

    def test_mixed_whitespace(self) -> None:
        types = _types("h1 \t\r\n h2")
        assert types == ["IDENT", "IDENT"]


# ---------------------------------------------------------------------------
# Complex inputs (integration tests)
# ---------------------------------------------------------------------------


class TestComplexInputs:
    """Test tokenization of realistic CSS code."""

    def test_simple_declaration(self) -> None:
        """color: red;"""
        types = _types("color: red;")
        assert types == ["IDENT", "COLON", "IDENT", "SEMICOLON"]

    def test_dimension_declaration(self) -> None:
        """margin: 10px 20px;"""
        types = _types("margin: 10px 20px;")
        assert types == ["IDENT", "COLON", "DIMENSION", "DIMENSION",
                         "SEMICOLON"]

    def test_selector(self) -> None:
        """div.class > p:hover"""
        types = _types("div.class > p:hover")
        assert types == ["IDENT", "DOT", "IDENT", "GREATER",
                         "IDENT", "COLON", "IDENT"]

    def test_full_rule(self) -> None:
        """Complete rule with selector and declarations."""
        types = _types("h1 { color: #333; font-size: 16px; }")
        assert types == [
            "IDENT", "LBRACE",
            "IDENT", "COLON", "HASH", "SEMICOLON",
            "IDENT", "COLON", "DIMENSION", "SEMICOLON",
            "RBRACE",
        ]

    def test_at_rule_import(self) -> None:
        """@import "file.css";"""
        types = _types('@import "file.css";')
        assert types == ["AT_KEYWORD", "STRING", "SEMICOLON"]

    def test_at_rule_media(self) -> None:
        """@media screen and (min-width: 768px) { }"""
        source = "@media screen { h1 { color: red; } }"
        types = _types(source)
        assert types[0] == "AT_KEYWORD"
        assert "LBRACE" in types
        assert "RBRACE" in types

    def test_function_call_rgb(self) -> None:
        """color: rgb(255, 0, 0);"""
        types = _types("color: rgb(255, 0, 0);")
        assert types == [
            "IDENT", "COLON",
            "FUNCTION", "NUMBER", "COMMA", "NUMBER", "COMMA",
            "NUMBER", "RPAREN", "SEMICOLON",
        ]

    def test_calc_expression(self) -> None:
        """width: calc(100% - 20px);"""
        types = _types("width: calc(100% - 20px);")
        assert types == [
            "IDENT", "COLON", "FUNCTION", "PERCENTAGE",
            "MINUS", "DIMENSION", "RPAREN", "SEMICOLON",
        ]

    def test_var_function(self) -> None:
        """color: var(--main-color, blue);"""
        types = _types("color: var(--main-color, blue);")
        assert "FUNCTION" in types
        assert "CUSTOM_PROPERTY" in types

    def test_important_annotation(self) -> None:
        """color: red !important;"""
        types = _types("color: red !important;")
        assert types == ["IDENT", "COLON", "IDENT", "BANG",
                         "IDENT", "SEMICOLON"]

    def test_attribute_selector(self) -> None:
        """[type="text"]"""
        types = _types('[type="text"]')
        assert types == ["LBRACKET", "IDENT", "EQUALS", "STRING",
                         "RBRACKET"]

    def test_pseudo_element(self) -> None:
        """p::before"""
        types = _types("p::before")
        assert types == ["IDENT", "COLON_COLON", "IDENT"]

    def test_pseudo_class_function(self) -> None:
        """:nth-child(2n+1)"""
        types = _types(":nth-child(2n+1)")
        assert types[0] == "COLON"
        assert types[1] == "FUNCTION"

    def test_nesting_selector(self) -> None:
        """& .child { }"""
        types = _types("& .child { }")
        assert types[0] == "AMPERSAND"

    def test_multiline_css(self) -> None:
        """A realistic multi-line CSS snippet."""
        source = """\
.container {
  display: flex;
  margin: 0 auto;
  max-width: 1200px;
}
"""
        types = _types(source)
        # Should tokenize without errors
        assert types[0] == "DOT"
        assert types[1] == "IDENT"
        assert types[2] == "LBRACE"
        assert types[-1] == "RBRACE"

    def test_position_tracking(self) -> None:
        """Line and column numbers are tracked correctly."""
        tokens = tokenize_css("h1 {\n  color: red;\n}")
        # h1 is at line 1, column 1
        assert tokens[0].line == 1
        assert tokens[0].column == 1
        # color is at line 2, column 3
        color_token = next(t for t in tokens if t.value == "color")
        assert color_token.line == 2
        assert color_token.column == 3

    def test_empty_input(self) -> None:
        """Empty input produces only EOF."""
        tokens = tokenize_css("")
        assert len(tokens) == 1
        type_name = tokens[0].type if isinstance(tokens[0].type, str) else tokens[0].type.name
        assert type_name == "EOF"

    def test_comments_only(self) -> None:
        """Input with only comments produces only EOF."""
        tokens = tokenize_css("/* comment */")
        assert len(tokens) == 1
