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


# ============================================================================
# Test: Version Parameter
# ============================================================================


class TestVersionParameter:
    """Test that the ``version`` parameter loads the correct ECMAScript grammar.

    Each ECMAScript version corresponds to a ``.tokens`` file under
    ``code/grammars/ecmascript/``.  The version-aware API must:

    1. Accept all 14 valid version strings without raising errors.
    2. Still produce correct tokens — ``break`` is a hard keyword in every ES
       version, making it a reliable sentinel for "the grammar loaded".
    3. Raise ``ValueError`` for unknown version strings.
    4. Treat ``None`` and ``""`` as "use the generic javascript.tokens grammar".
    """

    def test_no_version_uses_generic_grammar(self) -> None:
        """Omitting ``version`` (``None``) loads the generic javascript.tokens."""
        tokens = tokenize_javascript("let x = 1;")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "let"

    def test_empty_string_uses_generic_grammar(self) -> None:
        """An empty string also loads the generic javascript.tokens."""
        tokens = tokenize_javascript("let x = 1;", "")
        assert tokens[0].type == TokenType.KEYWORD

    def test_es1_version(self) -> None:
        """``es1`` loads the ECMAScript 1 grammar (June 1997)."""
        # ``break`` is a hard keyword since ES1.
        tokens = tokenize_javascript("break", "es1")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "break"

    def test_es3_version(self) -> None:
        """``es3`` loads the ECMAScript 3 grammar (December 1999)."""
        tokens = tokenize_javascript("break", "es3")
        assert tokens[0].type == TokenType.KEYWORD

    def test_es5_version(self) -> None:
        """``es5`` loads the ECMAScript 5 grammar (December 2009)."""
        tokens = tokenize_javascript("break", "es5")
        assert tokens[0].type == TokenType.KEYWORD

    def test_es2015_version(self) -> None:
        """``es2015`` loads the ES2015 grammar (ES6, June 2015)."""
        tokens = tokenize_javascript("const x = 1;", "es2015")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "const"

    def test_es2016_version(self) -> None:
        """``es2016`` loads the ES2016 grammar."""
        tokens = tokenize_javascript("break", "es2016")
        assert tokens[0].type == TokenType.KEYWORD

    def test_es2017_version(self) -> None:
        """``es2017`` loads the ES2017 grammar (async/await)."""
        tokens = tokenize_javascript("break", "es2017")
        assert tokens[0].type == TokenType.KEYWORD

    def test_es2018_version(self) -> None:
        """``es2018`` loads the ES2018 grammar."""
        tokens = tokenize_javascript("break", "es2018")
        assert tokens[0].type == TokenType.KEYWORD

    def test_es2019_version(self) -> None:
        """``es2019`` loads the ES2019 grammar."""
        tokens = tokenize_javascript("break", "es2019")
        assert tokens[0].type == TokenType.KEYWORD

    def test_es2020_version(self) -> None:
        """``es2020`` loads the ES2020 grammar (BigInt, ??  optional chaining)."""
        tokens = tokenize_javascript("break", "es2020")
        assert tokens[0].type == TokenType.KEYWORD

    def test_es2021_version(self) -> None:
        """``es2021`` loads the ES2021 grammar (logical assignment)."""
        tokens = tokenize_javascript("break", "es2021")
        assert tokens[0].type == TokenType.KEYWORD

    def test_es2022_version(self) -> None:
        """``es2022`` loads the ES2022 grammar (top-level await, class fields)."""
        tokens = tokenize_javascript("break", "es2022")
        assert tokens[0].type == TokenType.KEYWORD

    def test_es2023_version(self) -> None:
        """``es2023`` loads the ES2023 grammar."""
        tokens = tokenize_javascript("break", "es2023")
        assert tokens[0].type == TokenType.KEYWORD

    def test_es2024_version(self) -> None:
        """``es2024`` loads the ES2024 grammar."""
        tokens = tokenize_javascript("break", "es2024")
        assert tokens[0].type == TokenType.KEYWORD

    def test_es2025_version(self) -> None:
        """``es2025`` loads the ES2025 grammar (using/await using)."""
        tokens = tokenize_javascript("const x = 1;", "es2025")
        assert tokens[0].type == TokenType.KEYWORD

    def test_unknown_version_raises_value_error(self) -> None:
        """An unrecognized version string must raise ``ValueError``."""
        import pytest
        with pytest.raises(ValueError, match="Unknown ECMAScript version"):
            tokenize_javascript("let x = 1;", "es99")

    def test_version_propagates_to_factory(self) -> None:
        """``create_javascript_lexer`` with a version should produce valid tokens."""
        lexer = create_javascript_lexer("break", "es5")
        tokens = lexer.tokenize()
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "break"
        assert tokens[-1].type == TokenType.EOF
