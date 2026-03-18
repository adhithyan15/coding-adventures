"""
Comprehensive Tests for the Lexer
==================================

These tests verify that the lexer correctly tokenizes source code into tokens.
They are organized from simple (individual token types) to complex (full
expressions and edge cases).

Testing philosophy: We test *behavior*, not implementation. We don't care
how the lexer internally reads characters — we only care that given an input
string, it produces the correct sequence of tokens.
"""

from __future__ import annotations

import pytest

from lexer.tokenizer import Lexer, LexerConfig, LexerError, Token, TokenType


# ============================================================================
# Helper
# ============================================================================

def tokenize(source: str, config: LexerConfig | None = None) -> list[Token]:
    """Convenience wrapper — tokenize a string and return the token list."""
    return Lexer(source, config).tokenize()


def token_types(source: str, config: LexerConfig | None = None) -> list[TokenType]:
    """Return just the token types (ignoring values and positions)."""
    return [t.type for t in tokenize(source, config)]


def token_values(source: str, config: LexerConfig | None = None) -> list[str]:
    """Return just the token values."""
    return [t.value for t in tokenize(source, config)]


# ============================================================================
# Single token types
# ============================================================================

class TestSingleTokens:
    """Test that each token type is correctly recognized in isolation."""

    def test_name_simple(self) -> None:
        tokens = tokenize("x")
        assert tokens[0] == Token(TokenType.NAME, "x", 1, 1)
        assert tokens[1].type == TokenType.EOF

    def test_name_with_underscore(self) -> None:
        tokens = tokenize("_foo")
        assert tokens[0] == Token(TokenType.NAME, "_foo", 1, 1)

    def test_name_with_digits(self) -> None:
        tokens = tokenize("var1")
        assert tokens[0] == Token(TokenType.NAME, "var1", 1, 1)

    def test_name_long(self) -> None:
        tokens = tokenize("hello_world_123")
        assert tokens[0] == Token(TokenType.NAME, "hello_world_123", 1, 1)

    def test_number_single_digit(self) -> None:
        tokens = tokenize("5")
        assert tokens[0] == Token(TokenType.NUMBER, "5", 1, 1)

    def test_number_multi_digit(self) -> None:
        tokens = tokenize("42")
        assert tokens[0] == Token(TokenType.NUMBER, "42", 1, 1)

    def test_number_large(self) -> None:
        tokens = tokenize("1000")
        assert tokens[0] == Token(TokenType.NUMBER, "1000", 1, 1)

    def test_number_zero(self) -> None:
        tokens = tokenize("0")
        assert tokens[0] == Token(TokenType.NUMBER, "0", 1, 1)

    def test_string_simple(self) -> None:
        tokens = tokenize('"hello"')
        assert tokens[0] == Token(TokenType.STRING, "hello", 1, 1)

    def test_string_empty(self) -> None:
        tokens = tokenize('""')
        assert tokens[0] == Token(TokenType.STRING, "", 1, 1)

    def test_string_with_spaces(self) -> None:
        tokens = tokenize('"hello world"')
        assert tokens[0] == Token(TokenType.STRING, "hello world", 1, 1)

    def test_plus(self) -> None:
        tokens = tokenize("+")
        assert tokens[0] == Token(TokenType.PLUS, "+", 1, 1)

    def test_minus(self) -> None:
        tokens = tokenize("-")
        assert tokens[0] == Token(TokenType.MINUS, "-", 1, 1)

    def test_star(self) -> None:
        tokens = tokenize("*")
        assert tokens[0] == Token(TokenType.STAR, "*", 1, 1)

    def test_slash(self) -> None:
        tokens = tokenize("/")
        assert tokens[0] == Token(TokenType.SLASH, "/", 1, 1)

    def test_equals(self) -> None:
        tokens = tokenize("=")
        assert tokens[0] == Token(TokenType.EQUALS, "=", 1, 1)

    def test_equals_equals(self) -> None:
        tokens = tokenize("==")
        assert tokens[0] == Token(TokenType.EQUALS_EQUALS, "==", 1, 1)

    def test_lparen(self) -> None:
        tokens = tokenize("(")
        assert tokens[0] == Token(TokenType.LPAREN, "(", 1, 1)

    def test_rparen(self) -> None:
        tokens = tokenize(")")
        assert tokens[0] == Token(TokenType.RPAREN, ")", 1, 1)

    def test_comma(self) -> None:
        tokens = tokenize(",")
        assert tokens[0] == Token(TokenType.COMMA, ",", 1, 1)

    def test_colon(self) -> None:
        tokens = tokenize(":")
        assert tokens[0] == Token(TokenType.COLON, ":", 1, 1)

    def test_newline(self) -> None:
        tokens = tokenize("\n")
        assert tokens[0] == Token(TokenType.NEWLINE, "\\n", 1, 1)
        assert tokens[1].type == TokenType.EOF


# ============================================================================
# End-to-end expression tests
# ============================================================================

class TestExpressions:
    """Test tokenizing complete expressions."""

    def test_assignment_expression(self) -> None:
        """The canonical test case: x = 1 + 2"""
        tokens = tokenize("x = 1 + 2")
        expected_types = [
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.NUMBER,
            TokenType.PLUS,
            TokenType.NUMBER,
            TokenType.EOF,
        ]
        expected_values = ["x", "=", "1", "+", "2", ""]
        assert [t.type for t in tokens] == expected_types
        assert [t.value for t in tokens] == expected_values

    def test_comparison_expression(self) -> None:
        tokens = tokenize("x == 1")
        assert token_types("x == 1") == [
            TokenType.NAME,
            TokenType.EQUALS_EQUALS,
            TokenType.NUMBER,
            TokenType.EOF,
        ]

    def test_arithmetic_expression(self) -> None:
        types = token_types("a + b * c - d / e")
        assert types == [
            TokenType.NAME,
            TokenType.PLUS,
            TokenType.NAME,
            TokenType.STAR,
            TokenType.NAME,
            TokenType.MINUS,
            TokenType.NAME,
            TokenType.SLASH,
            TokenType.NAME,
            TokenType.EOF,
        ]

    def test_function_call_style(self) -> None:
        types = token_types("print(x, y)")
        assert types == [
            TokenType.NAME,
            TokenType.LPAREN,
            TokenType.NAME,
            TokenType.COMMA,
            TokenType.NAME,
            TokenType.RPAREN,
            TokenType.EOF,
        ]

    def test_no_spaces(self) -> None:
        """Tokens should be recognized even without spaces between them."""
        types = token_types("x=1+2")
        assert types == [
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.NUMBER,
            TokenType.PLUS,
            TokenType.NUMBER,
            TokenType.EOF,
        ]

    def test_extra_spaces(self) -> None:
        """Extra whitespace should be ignored."""
        types = token_types("  x   =   1  ")
        assert types == [
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.NUMBER,
            TokenType.EOF,
        ]

    def test_colon_usage(self) -> None:
        """Colon appears in dict-like syntax."""
        types = token_types("key: value")
        assert types == [
            TokenType.NAME,
            TokenType.COLON,
            TokenType.NAME,
            TokenType.EOF,
        ]

    def test_mixed_expression_with_string(self) -> None:
        tokens = tokenize('x = "hello"')
        assert token_types('x = "hello"') == [
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.STRING,
            TokenType.EOF,
        ]
        assert tokens[2].value == "hello"

    def test_equals_followed_by_equals_equals(self) -> None:
        """Make sure = and == are correctly distinguished in sequence."""
        types = token_types("a = b == c")
        assert types == [
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.NAME,
            TokenType.EQUALS_EQUALS,
            TokenType.NAME,
            TokenType.EOF,
        ]


# ============================================================================
# String literals and escapes
# ============================================================================

class TestStrings:
    """Test string literal tokenization, including escape sequences."""

    def test_escape_newline(self) -> None:
        tokens = tokenize(r'"hello\nworld"')
        assert tokens[0].value == "hello\nworld"

    def test_escape_tab(self) -> None:
        tokens = tokenize(r'"col1\tcol2"')
        assert tokens[0].value == "col1\tcol2"

    def test_escape_backslash(self) -> None:
        tokens = tokenize(r'"path\\to\\file"')
        assert tokens[0].value == "path\\to\\file"

    def test_escape_quote(self) -> None:
        tokens = tokenize(r'"He said \"hi\""')
        assert tokens[0].value == 'He said "hi"'

    def test_string_with_digits(self) -> None:
        tokens = tokenize('"abc 123"')
        assert tokens[0].value == "abc 123"

    def test_string_with_operators(self) -> None:
        tokens = tokenize('"1 + 2 = 3"')
        assert tokens[0].value == "1 + 2 = 3"

    def test_unknown_escape(self) -> None:
        """Unknown escape sequences are passed through as-is."""
        tokens = tokenize(r'"hello\xworld"')
        assert tokens[0].value == "helloxworld"


# ============================================================================
# Multi-line input
# ============================================================================

class TestMultiLine:
    """Test tokenization across multiple lines."""

    def test_two_lines(self) -> None:
        tokens = tokenize("x = 1\ny = 2")
        types = [t.type for t in tokens]
        assert types == [
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.NUMBER,
            TokenType.NEWLINE,
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.NUMBER,
            TokenType.EOF,
        ]

    def test_three_lines(self) -> None:
        source = "a = 1\nb = 2\nc = a + b"
        tokens = tokenize(source)
        # Count newlines
        newlines = [t for t in tokens if t.type == TokenType.NEWLINE]
        assert len(newlines) == 2

    def test_trailing_newline(self) -> None:
        tokens = tokenize("x = 1\n")
        types = [t.type for t in tokens]
        assert types == [
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.NUMBER,
            TokenType.NEWLINE,
            TokenType.EOF,
        ]

    def test_blank_lines(self) -> None:
        """Blank lines should produce consecutive NEWLINE tokens."""
        tokens = tokenize("x\n\ny")
        types = [t.type for t in tokens]
        assert types == [
            TokenType.NAME,
            TokenType.NEWLINE,
            TokenType.NEWLINE,
            TokenType.NAME,
            TokenType.EOF,
        ]


# ============================================================================
# Line and column tracking
# ============================================================================

class TestPositionTracking:
    """Test that line and column numbers are correctly tracked."""

    def test_first_token_position(self) -> None:
        tokens = tokenize("x")
        assert tokens[0].line == 1
        assert tokens[0].column == 1

    def test_second_token_after_space(self) -> None:
        tokens = tokenize("x = 1")
        # "x" is at col 1, "=" is at col 3, "1" is at col 5
        assert tokens[0].column == 1  # x
        assert tokens[1].column == 3  # =
        assert tokens[2].column == 5  # 1

    def test_position_on_second_line(self) -> None:
        tokens = tokenize("x\ny")
        # "y" should be on line 2, column 1
        y_token = [t for t in tokens if t.value == "y"][0]
        assert y_token.line == 2
        assert y_token.column == 1

    def test_position_tracking_multi_line(self) -> None:
        source = "abc\nde = 1"
        tokens = tokenize(source)
        # abc: line 1, col 1
        # \n: line 1, col 4
        # de: line 2, col 1
        # =: line 2, col 4
        # 1: line 2, col 6
        assert tokens[0] == Token(TokenType.NAME, "abc", 1, 1)
        de_token = [t for t in tokens if t.value == "de"][0]
        assert de_token.line == 2
        assert de_token.column == 1
        eq_token = [t for t in tokens if t.type == TokenType.EQUALS][0]
        assert eq_token.line == 2
        assert eq_token.column == 4

    def test_eof_position_at_end(self) -> None:
        tokens = tokenize("ab")
        eof = tokens[-1]
        assert eof.type == TokenType.EOF
        # After reading "ab", position is at line 1, col 3
        assert eof.line == 1
        assert eof.column == 3

    def test_position_with_tabs(self) -> None:
        """Tabs should advance the column by 1 (they are single characters)."""
        tokens = tokenize("\tx")
        assert tokens[0].line == 1
        assert tokens[0].column == 2  # tab counts as 1 column advance

    def test_string_position(self) -> None:
        """String token position should point to the opening quote."""
        tokens = tokenize('x = "hi"')
        string_token = [t for t in tokens if t.type == TokenType.STRING][0]
        assert string_token.column == 5


# ============================================================================
# Error cases
# ============================================================================

class TestErrors:
    """Test that the lexer raises appropriate errors for invalid input."""

    def test_unterminated_string(self) -> None:
        with pytest.raises(LexerError, match="Unterminated string literal"):
            tokenize('"hello')

    def test_unterminated_string_with_escape_at_end(self) -> None:
        with pytest.raises(LexerError, match="Unterminated string literal"):
            tokenize('"hello\\')

    def test_unexpected_character(self) -> None:
        with pytest.raises(LexerError, match="Unexpected character"):
            tokenize("@")

    def test_unexpected_character_position(self) -> None:
        """The error should report the correct position."""
        try:
            tokenize("x = @")
        except LexerError as e:
            assert e.line == 1
            assert e.column == 5

    def test_unexpected_character_hash(self) -> None:
        with pytest.raises(LexerError, match="Unexpected character"):
            tokenize("#")

    def test_unexpected_character_on_second_line(self) -> None:
        try:
            tokenize("x = 1\n@")
        except LexerError as e:
            assert e.line == 2
            assert e.column == 1


# ============================================================================
# Keywords vs names
# ============================================================================

class TestKeywords:
    """Test keyword recognition with configurable keyword lists."""

    def test_no_config_means_no_keywords(self) -> None:
        """Without a config, all identifiers are NAME tokens."""
        tokens = tokenize("if else while")
        types = [t.type for t in tokens if t.type != TokenType.EOF]
        assert all(t == TokenType.NAME for t in types)

    def test_python_keywords(self) -> None:
        config = LexerConfig(keywords=["if", "else", "while", "def", "return"])
        tokens = tokenize("if x == 1", config)
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "if"
        assert tokens[1].type == TokenType.NAME
        assert tokens[1].value == "x"

    def test_ruby_keywords(self) -> None:
        config = LexerConfig(keywords=["if", "elsif", "end", "def", "puts"])
        tokens = tokenize("elsif x", config)
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "elsif"

    def test_non_keyword_identifier(self) -> None:
        """Identifiers that look like keywords but aren't should be NAME."""
        config = LexerConfig(keywords=["if"])
        tokens = tokenize("iffy")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "iffy"

    def test_keyword_in_expression(self) -> None:
        config = LexerConfig(keywords=["return"])
        tokens = tokenize("return x + 1", config)
        assert tokens[0] == Token(TokenType.KEYWORD, "return", 1, 1)
        assert tokens[1].type == TokenType.NAME

    def test_multiple_keywords(self) -> None:
        config = LexerConfig(keywords=["def", "return"])
        tokens = tokenize("def foo", config)
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[1].type == TokenType.NAME

    def test_keyword_set_property(self) -> None:
        config = LexerConfig(keywords=["if", "else"])
        assert config.keyword_set == frozenset({"if", "else"})

    def test_default_config_has_no_keywords(self) -> None:
        config = LexerConfig()
        assert config.keyword_set == frozenset()


# ============================================================================
# Edge cases
# ============================================================================

class TestEdgeCases:
    """Test boundary conditions and special inputs."""

    def test_empty_input(self) -> None:
        """Empty input should produce only an EOF token."""
        tokens = tokenize("")
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.EOF

    def test_only_whitespace(self) -> None:
        """Whitespace-only input should produce only an EOF token."""
        tokens = tokenize("   \t  ")
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.EOF

    def test_only_newlines(self) -> None:
        tokens = tokenize("\n\n")
        types = [t.type for t in tokens]
        assert types == [TokenType.NEWLINE, TokenType.NEWLINE, TokenType.EOF]

    def test_single_character_name(self) -> None:
        tokens = tokenize("a")
        assert tokens[0].value == "a"

    def test_underscore_only_name(self) -> None:
        tokens = tokenize("_")
        assert tokens[0] == Token(TokenType.NAME, "_", 1, 1)

    def test_multiple_operators_no_spaces(self) -> None:
        types = token_types("+-*/")
        assert types == [
            TokenType.PLUS,
            TokenType.MINUS,
            TokenType.STAR,
            TokenType.SLASH,
            TokenType.EOF,
        ]

    def test_parentheses_around_expression(self) -> None:
        types = token_types("(1 + 2)")
        assert types == [
            TokenType.LPAREN,
            TokenType.NUMBER,
            TokenType.PLUS,
            TokenType.NUMBER,
            TokenType.RPAREN,
            TokenType.EOF,
        ]

    def test_consecutive_strings(self) -> None:
        tokens = tokenize('"a" "b"')
        assert tokens[0].value == "a"
        assert tokens[1].value == "b"

    def test_token_repr(self) -> None:
        token = Token(TokenType.NAME, "x", 1, 1)
        assert repr(token) == "Token(NAME, 'x', 1:1)"

    def test_lexer_error_message(self) -> None:
        err = LexerError("bad char", 3, 7)
        assert "3:7" in str(err)
        assert "bad char" in str(err)
        assert err.line == 3
        assert err.column == 7

    def test_lexer_is_single_use(self) -> None:
        """Calling tokenize() twice does not re-scan; Lexer is single-use.

        The position cursor is NOT reset between calls, so a second call
        starts at the end of input and produces only EOF.
        """
        lexer = Lexer("x = 1")
        first = lexer.tokenize()
        second = lexer.tokenize()
        assert len(first) == 4  # NAME, EQUALS, NUMBER, EOF
        assert len(second) == 1  # only EOF (position already at end)

    def test_carriage_return_is_whitespace(self) -> None:
        """Carriage return (\\r) should be treated as whitespace."""
        tokens = tokenize("x\r= 1")
        types = [t.type for t in tokens]
        assert TokenType.NAME in types
        assert TokenType.EQUALS in types

    def test_string_containing_newline_literal(self) -> None:
        """A string can contain an actual newline character."""
        tokens = tokenize('"line1\nline2"')
        # The newline is inside the string, so it's part of the string value
        assert tokens[0].type == TokenType.STRING
        assert "\n" in tokens[0].value
