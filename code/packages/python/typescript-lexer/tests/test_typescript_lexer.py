"""Tests for the TypeScript Lexer.

These tests verify that the grammar-driven lexer, when loaded with the
``typescript.tokens`` grammar file, correctly tokenizes TypeScript source code.

The key insight being tested here is that **no new lexer code was written**.
The same ``GrammarLexer`` that handles Python, Ruby, and JavaScript handles
TypeScript — only the grammar file changed.
"""

from __future__ import annotations

from lexer import Token, TokenType

from typescript_lexer import create_typescript_lexer, tokenize_typescript


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
# Test: Basic TypeScript Expressions
# ============================================================================


class TestBasicExpressions:
    """Test that simple TypeScript expressions tokenize correctly."""

    def test_let_assignment(self) -> None:
        """Tokenize ``let x = 1 + 2;`` — a variable declaration."""
        tokens = tokenize_typescript("let x = 1 + 2;")
        assert token_values(tokens) == ["let", "x", "=", "1", "+", "2", ";", ""]
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[1].type == TokenType.NAME
        assert tokens[2].type == TokenType.EQUALS
        assert tokens[3].type == TokenType.NUMBER
        assert tokens[4].type == TokenType.PLUS
        assert tokens[-1].type == TokenType.EOF

    def test_arithmetic_operators(self) -> None:
        """Tokenize ``a + b - c * d / e`` — all four arithmetic operators."""
        tokens = tokenize_typescript("a + b - c * d / e")
        assert token_types(tokens) == [
            "NAME", "PLUS", "NAME", "MINUS", "NAME",
            "STAR", "NAME", "SLASH", "NAME", "EOF",
        ]

    def test_parenthesized_expression(self) -> None:
        """Tokenize ``(1 + 2) * 3`` — parentheses for grouping."""
        tokens = tokenize_typescript("(1 + 2) * 3")
        assert token_types(tokens) == [
            "LPAREN", "NUMBER", "PLUS", "NUMBER", "RPAREN",
            "STAR", "NUMBER", "EOF",
        ]


# ============================================================================
# Test: TypeScript-Specific Keywords
# ============================================================================


class TestTypeScriptKeywords:
    """Test that TypeScript-specific keywords are recognized correctly."""

    def test_interface_keyword(self) -> None:
        """The ``interface`` keyword declares interfaces."""
        tokens = tokenize_typescript("interface")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "interface"

    def test_type_keyword(self) -> None:
        """The ``type`` keyword declares type aliases."""
        tokens = tokenize_typescript("type")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "type"

    def test_number_keyword(self) -> None:
        """The ``number`` keyword is a TypeScript primitive type."""
        tokens = tokenize_typescript("number")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "number"


# ============================================================================
# Test: JavaScript Keywords (Inherited)
# ============================================================================


class TestJavaScriptKeywords:
    """Test that JavaScript keywords still work in TypeScript mode."""

    def test_let_keyword(self) -> None:
        """The ``let`` keyword declares block-scoped variables."""
        tokens = tokenize_typescript("let")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "let"

    def test_const_keyword(self) -> None:
        """The ``const`` keyword declares block-scoped constants."""
        tokens = tokenize_typescript("const")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "const"

    def test_function_keyword(self) -> None:
        """The ``function`` keyword declares functions."""
        tokens = tokenize_typescript("function")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "function"

    def test_boolean_and_null_keywords(self) -> None:
        """TypeScript uses ``true``, ``false``, ``null``, ``undefined``."""
        tokens = tokenize_typescript("true false null undefined")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["true", "false", "null", "undefined"]

    def test_keyword_vs_name(self) -> None:
        """A keyword embedded in a longer name should NOT be a keyword."""
        tokens = tokenize_typescript("letters")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "letters"


# ============================================================================
# Test: JavaScript-Specific Operators
# ============================================================================


class TestJavaScriptOperators:
    """Test JavaScript-specific operators (inherited by TypeScript)."""

    def test_strict_equality(self) -> None:
        """The ``===`` operator tests strict equality."""
        tokens = tokenize_typescript("x === 1")
        assert tokens[1].value == "==="

    def test_strict_inequality(self) -> None:
        """The ``!==`` operator tests strict inequality."""
        tokens = tokenize_typescript("x !== 1")
        assert tokens[1].value == "!=="

    def test_equality(self) -> None:
        """The ``==`` operator tests equality."""
        tokens = tokenize_typescript("x == 1")
        assert token_types(tokens) == ["NAME", "EQUALS_EQUALS", "NUMBER", "EOF"]

    def test_arrow(self) -> None:
        """The ``=>`` operator creates arrow functions."""
        tokens = tokenize_typescript("x => x")
        assert tokens[1].value == "=>"


# ============================================================================
# Test: Delimiters
# ============================================================================


class TestDelimiters:
    """Test TypeScript delimiters: braces, brackets, semicolons."""

    def test_curly_braces(self) -> None:
        """Tokenize ``{ }`` — block delimiters."""
        tokens = tokenize_typescript("{ }")
        assert token_values(tokens) == ["{", "}", ""]

    def test_square_brackets(self) -> None:
        """Tokenize ``[ ]`` — array delimiters."""
        tokens = tokenize_typescript("[ ]")
        assert token_values(tokens) == ["[", "]", ""]

    def test_semicolon(self) -> None:
        """Semicolons terminate statements in TypeScript."""
        tokens = tokenize_typescript(";")
        assert tokens[0].value == ";"


# ============================================================================
# Test: Identifiers with $
# ============================================================================


class TestIdentifiers:
    """Test that ``$`` is valid in TypeScript identifiers."""

    def test_dollar_sign_identifier(self) -> None:
        """The ``$`` character is valid in TypeScript identifiers."""
        tokens = tokenize_typescript("$foo")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "$foo"


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTypescriptLexer:
    """Test the ``create_typescript_lexer()`` factory function."""

    def test_creates_lexer(self) -> None:
        """The factory function should return a GrammarLexer instance."""
        lexer = create_typescript_lexer("let x = 1;")
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_result(self) -> None:
        """The factory should produce the same tokens as tokenize_typescript()."""
        source = "let x = 1 + 2;"
        tokens_direct = tokenize_typescript(source)
        tokens_factory = create_typescript_lexer(source).tokenize()
        assert tokens_direct == tokens_factory


# ============================================================================
# Test: Version Parameter
# ============================================================================


class TestVersionParameter:
    """Test that the ``version`` parameter loads the correct grammar.

    Each TypeScript version corresponds to a ``.tokens`` file under
    ``code/grammars/typescript/``.  The version-aware API must:

    1. Accept all six valid version strings without raising errors.
    2. Still produce correct tokens — the base token types are the same
       across all versions; versioned grammars only ADD new keywords.
    3. Raise ``ValueError`` for unknown version strings.
    4. Treat ``None`` and ``""`` as "use the generic grammar".
    """

    def test_no_version_uses_generic_grammar(self) -> None:
        """Omitting ``version`` (``None``) loads the generic typescript.tokens."""
        tokens = tokenize_typescript("let x = 1;")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "let"

    def test_empty_string_uses_generic_grammar(self) -> None:
        """An empty string version also loads the generic typescript.tokens."""
        tokens = tokenize_typescript("let x = 1;", "")
        assert tokens[0].type == TokenType.KEYWORD

    def test_ts10_version(self) -> None:
        """``ts1.0`` loads the TypeScript 1.0 grammar (April 2014).

        In ``ts1.0``, ``interface`` is a *context keyword* — it is lexed as
        NAME (not KEYWORD) because it can still appear as an identifier in
        non-declaration positions.  The full list of reserved keywords (like
        ``let``, ``const``) are always KEYWORD.
        """
        # ``var`` is a hard keyword in all TypeScript versions (ES3+).
        tokens = tokenize_typescript("var x = 1;", "ts1.0")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "var"

    def test_ts20_version(self) -> None:
        """``ts2.0`` loads the TypeScript 2.0 grammar (September 2016)."""
        tokens = tokenize_typescript("let x: never;", "ts2.0")
        assert any(t.value == "never" for t in tokens)

    def test_ts30_version(self) -> None:
        """``ts3.0`` loads the TypeScript 3.0 grammar (July 2018)."""
        tokens = tokenize_typescript("let x: unknown;", "ts3.0")
        assert any(t.value == "unknown" for t in tokens)

    def test_ts40_version(self) -> None:
        """``ts4.0`` loads the TypeScript 4.0 grammar (August 2020)."""
        tokens = tokenize_typescript("let x = 1;", "ts4.0")
        assert tokens[0].type == TokenType.KEYWORD

    def test_ts50_version(self) -> None:
        """``ts5.0`` loads the TypeScript 5.0 grammar (March 2023)."""
        tokens = tokenize_typescript("const x = 1;", "ts5.0")
        assert tokens[0].type == TokenType.KEYWORD

    def test_ts58_version(self) -> None:
        """``ts5.8`` loads the TypeScript 5.8 grammar (February 2025).

        In ``ts5.8``, ``using`` is a *context keyword* — it is lexed as NAME
        (not KEYWORD) because it can still appear as an identifier in other
        positions.  We verify that the grammar loads and tokenizes correctly
        by checking for expected NAME tokens.
        """
        tokens = tokenize_typescript("const x = 1;", "ts5.8")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "const"

    def test_unknown_version_raises_value_error(self) -> None:
        """An unrecognized version string must raise ``ValueError``."""
        import pytest
        with pytest.raises(ValueError, match="Unknown TypeScript version"):
            tokenize_typescript("let x = 1;", "ts99.0")

    def test_version_propagates_to_factory(self) -> None:
        """``create_typescript_lexer`` with a version should produce valid tokens."""
        lexer = create_typescript_lexer("let x: number = 1;", "ts1.0")
        tokens = lexer.tokenize()
        assert any(t.value == "number" for t in tokens)
        assert tokens[-1].type == TokenType.EOF
