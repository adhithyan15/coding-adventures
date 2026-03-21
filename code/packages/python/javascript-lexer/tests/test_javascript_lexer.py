"""Tests for the JavaScript Lexer.

These tests verify that the grammar-driven lexer, when loaded with the
``javascript.tokens`` grammar file, correctly tokenizes JavaScript source code.

The key insight being tested here is that **no new lexer code was written**.
The same ``GrammarLexer`` that handles Python and Ruby handles JavaScript —
only the grammar file changed.
"""

from __future__ import annotations

from lexer import Token, TokenType

from javascript_lexer import create_javascript_lexer, tokenize_javascript


# ============================================================================
# Helper — makes assertions more readable
# ============================================================================


def token_types(tokens: list[Token]) -> list[str]:
    """Extract just the type names from a token list."""
    return [t.type.name for t in tokens]


def token_values(tokens: list[Token]) -> list[str]:
    """Extract just the values from a token list."""
    return [t.value for t in tokens]


# ============================================================================
# Test: Basic JavaScript Expressions
# ============================================================================


class TestBasicExpressions:
    """Test that simple JavaScript expressions tokenize correctly."""

    def test_let_assignment(self) -> None:
        """Tokenize ``let x = 1 + 2;`` — a variable declaration."""
        tokens = tokenize_javascript("let x = 1 + 2;")
        assert token_types(tokens) == [
            "KEYWORD", "NAME", "EQUALS", "NUMBER", "PLUS", "NUMBER",
            "SEMICOLON", "EOF",
        ]
        assert token_values(tokens) == ["let", "x", "=", "1", "+", "2", ";", ""]

    def test_arithmetic_operators(self) -> None:
        """Tokenize ``a + b - c * d / e`` — all four arithmetic operators."""
        tokens = tokenize_javascript("a + b - c * d / e")
        assert token_types(tokens) == [
            "NAME", "PLUS", "NAME", "MINUS", "NAME",
            "STAR", "NAME", "SLASH", "NAME", "EOF",
        ]

    def test_parenthesized_expression(self) -> None:
        """Tokenize ``(1 + 2) * 3`` — parentheses for grouping."""
        tokens = tokenize_javascript("(1 + 2) * 3")
        assert token_types(tokens) == [
            "LPAREN", "NUMBER", "PLUS", "NUMBER", "RPAREN",
            "STAR", "NUMBER", "EOF",
        ]


# ============================================================================
# Test: JavaScript Keywords
# ============================================================================


class TestJavaScriptKeywords:
    """Test that JavaScript-specific keywords are recognized correctly."""

    def test_let_keyword(self) -> None:
        """The ``let`` keyword declares block-scoped variables."""
        tokens = tokenize_javascript("let")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "let"

    def test_const_keyword(self) -> None:
        """The ``const`` keyword declares block-scoped constants."""
        tokens = tokenize_javascript("const")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "const"

    def test_function_keyword(self) -> None:
        """The ``function`` keyword declares functions."""
        tokens = tokenize_javascript("function")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "function"

    def test_boolean_and_null_keywords(self) -> None:
        """JavaScript uses ``true``, ``false``, ``null``, ``undefined``."""
        tokens = tokenize_javascript("true false null undefined")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["true", "false", "null", "undefined"]

    def test_keyword_vs_name(self) -> None:
        """A keyword embedded in a longer name should NOT be a keyword."""
        tokens = tokenize_javascript("letters")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "letters"


# ============================================================================
# Test: JavaScript-Specific Operators
# ============================================================================


class TestJavaScriptOperators:
    """Test JavaScript-specific operators."""

    def test_strict_equality(self) -> None:
        """The ``===`` operator tests strict equality."""
        tokens = tokenize_javascript("x === 1")
        assert tokens[1].value == "==="

    def test_strict_inequality(self) -> None:
        """The ``!==`` operator tests strict inequality."""
        tokens = tokenize_javascript("x !== 1")
        assert tokens[1].value == "!=="

    def test_equality(self) -> None:
        """The ``==`` operator tests equality."""
        tokens = tokenize_javascript("x == 1")
        assert token_types(tokens) == ["NAME", "EQUALS_EQUALS", "NUMBER", "EOF"]

    def test_arrow(self) -> None:
        """The ``=>`` operator creates arrow functions."""
        tokens = tokenize_javascript("x => x")
        assert tokens[1].value == "=>"


# ============================================================================
# Test: Delimiters
# ============================================================================


class TestDelimiters:
    """Test JavaScript delimiters: braces, brackets, semicolons."""

    def test_curly_braces(self) -> None:
        """Tokenize ``{ }`` — block delimiters."""
        tokens = tokenize_javascript("{ }")
        assert token_types(tokens) == ["LBRACE", "RBRACE", "EOF"]

    def test_square_brackets(self) -> None:
        """Tokenize ``[ ]`` — array delimiters."""
        tokens = tokenize_javascript("[ ]")
        assert token_types(tokens) == ["LBRACKET", "RBRACKET", "EOF"]

    def test_semicolon(self) -> None:
        """Semicolons terminate statements in JavaScript."""
        tokens = tokenize_javascript(";")
        assert tokens[0].type == TokenType.SEMICOLON


# ============================================================================
# Test: Identifiers with $
# ============================================================================


class TestIdentifiers:
    """Test that ``$`` is valid in JavaScript identifiers."""

    def test_dollar_sign_identifier(self) -> None:
        """The ``$`` character is valid in JavaScript identifiers (e.g., jQuery)."""
        tokens = tokenize_javascript("$foo")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "$foo"


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateJavaScriptLexer:
    """Test the ``create_javascript_lexer()`` factory function."""

    def test_creates_lexer(self) -> None:
        """The factory function should return a GrammarLexer instance."""
        lexer = create_javascript_lexer("let x = 1;")
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_result(self) -> None:
        """The factory should produce the same tokens as tokenize_javascript()."""
        source = "let x = 1 + 2;"
        tokens_direct = tokenize_javascript(source)
        tokens_factory = create_javascript_lexer(source).tokenize()
        assert tokens_direct == tokens_factory
