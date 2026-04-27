"""Tests for the Java Lexer.

These tests verify that the grammar-driven lexer, when loaded with the
``java{version}.tokens`` grammar file, correctly tokenizes Java source code.

The key insight being tested here is that **no new lexer code was written**.
The same ``GrammarLexer`` that handles Python and JavaScript handles Java —
only the grammar file changed.
"""

from __future__ import annotations

from lexer import Token, TokenType

from java_lexer import create_java_lexer, tokenize_java


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
# Test: Basic Java Expressions
# ============================================================================


class TestBasicExpressions:
    """Test that simple Java expressions tokenize correctly."""

    def test_class_declaration(self) -> None:
        """Tokenize ``public class Hello { }`` — a basic class declaration."""
        tokens = tokenize_java("public class Hello { }")
        types = token_types(tokens)
        values = token_values(tokens)
        assert "KEYWORD" in types
        assert "public" in values
        assert "class" in values
        assert "Hello" in values
        assert tokens[-1].type == TokenType.EOF

    def test_int_assignment(self) -> None:
        """Tokenize ``int x = 42;`` — a local variable declaration."""
        tokens = tokenize_java("int x = 42;")
        types = token_types(tokens)
        values = token_values(tokens)
        assert "KEYWORD" in types
        assert "int" in values
        assert "x" in values
        assert "42" in values
        assert ";" in values

    def test_arithmetic_operators(self) -> None:
        """Tokenize ``a + b - c * d / e`` — all four arithmetic operators."""
        tokens = tokenize_java("a + b - c * d / e")
        assert token_types(tokens) == [
            "NAME", "PLUS", "NAME", "MINUS", "NAME",
            "STAR", "NAME", "SLASH", "NAME", "EOF",
        ]

    def test_parenthesized_expression(self) -> None:
        """Tokenize ``(1 + 2) * 3`` — parentheses for grouping."""
        tokens = tokenize_java("(1 + 2) * 3")
        assert token_types(tokens) == [
            "LPAREN", "NUMBER", "PLUS", "NUMBER", "RPAREN",
            "STAR", "NUMBER", "EOF",
        ]


# ============================================================================
# Test: Java Keywords
# ============================================================================


class TestJavaKeywords:
    """Test that Java-specific keywords are recognized correctly."""

    def test_public_keyword(self) -> None:
        """The ``public`` keyword is an access modifier."""
        tokens = tokenize_java("public")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "public"

    def test_class_keyword(self) -> None:
        """The ``class`` keyword declares classes."""
        tokens = tokenize_java("class")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "class"

    def test_void_keyword(self) -> None:
        """The ``void`` keyword specifies no return type."""
        tokens = tokenize_java("void")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "void"

    def test_boolean_keywords(self) -> None:
        """Java uses ``true``, ``false``, ``null``."""
        tokens = tokenize_java("true false null")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["true", "false", "null"]

    def test_keyword_vs_name(self) -> None:
        """A keyword embedded in a longer name should NOT be a keyword."""
        tokens = tokenize_java("classes")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "classes"


# ============================================================================
# Test: Java-Specific Operators
# ============================================================================


class TestJavaOperators:
    """Test Java-specific operators."""

    def test_equality(self) -> None:
        """The ``==`` operator tests equality."""
        tokens = tokenize_java("x == 1")
        assert tokens[1].value == "=="

    def test_inequality(self) -> None:
        """The ``!=`` operator tests inequality."""
        tokens = tokenize_java("x != 1")
        assert tokens[1].value == "!="

    def test_less_than_or_equal(self) -> None:
        """The ``<=`` operator tests less-than-or-equal."""
        tokens = tokenize_java("x <= 1")
        assert tokens[1].value == "<="

    def test_greater_than_or_equal(self) -> None:
        """The ``>=`` operator tests greater-than-or-equal."""
        tokens = tokenize_java("x >= 1")
        assert tokens[1].value == ">="


# ============================================================================
# Test: Delimiters
# ============================================================================


class TestDelimiters:
    """Test Java delimiters: braces, brackets, semicolons."""

    def test_curly_braces(self) -> None:
        """Tokenize ``{ }`` — block delimiters."""
        tokens = tokenize_java("{ }")
        assert token_types(tokens) == ["LBRACE", "RBRACE", "EOF"]

    def test_square_brackets(self) -> None:
        """Tokenize ``[ ]`` — array delimiters."""
        tokens = tokenize_java("[ ]")
        assert token_types(tokens) == ["LBRACKET", "RBRACKET", "EOF"]

    def test_semicolon(self) -> None:
        """Semicolons terminate statements in Java."""
        tokens = tokenize_java(";")
        assert tokens[0].type == TokenType.SEMICOLON


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateJavaLexer:
    """Test the ``create_java_lexer()`` factory function."""

    def test_creates_lexer(self) -> None:
        """The factory function should return a GrammarLexer instance."""
        lexer = create_java_lexer("int x = 1;")
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_result(self) -> None:
        """The factory should produce the same tokens as tokenize_java()."""
        source = "int x = 1 + 2;"
        tokens_direct = tokenize_java(source)
        tokens_factory = create_java_lexer(source).tokenize()
        assert tokens_direct == tokens_factory


# ============================================================================
# Test: Version Parameter
# ============================================================================


class TestVersionParameter:
    """Test that the ``version`` parameter loads the correct Java grammar.

    Each Java version corresponds to a ``.tokens`` file under
    ``code/grammars/java/``.  The version-aware API must:

    1. Accept all 10 valid version strings without raising errors.
    2. Still produce correct tokens — ``class`` is a keyword in every Java
       version, making it a reliable sentinel for "the grammar loaded".
    3. Raise ``ValueError`` for unknown version strings.
    4. Treat ``None`` and ``""`` as "use the default java21.tokens grammar".
    """

    def test_no_version_uses_default_grammar(self) -> None:
        """Omitting ``version`` (``None``) loads the default java21.tokens."""
        tokens = tokenize_java("class Hello { }")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "class"

    def test_empty_string_uses_default_grammar(self) -> None:
        """An empty string also loads the default java21.tokens."""
        tokens = tokenize_java("class Hello { }", "")
        assert tokens[0].type == TokenType.KEYWORD

    def test_version_1_0(self) -> None:
        """``1.0`` loads the Java 1.0 grammar (January 1996)."""
        tokens = tokenize_java("class", "1.0")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "class"

    def test_version_1_1(self) -> None:
        """``1.1`` loads the Java 1.1 grammar (February 1997)."""
        tokens = tokenize_java("class", "1.1")
        assert tokens[0].type == TokenType.KEYWORD

    def test_version_1_4(self) -> None:
        """``1.4`` loads the Java 1.4 grammar (February 2002)."""
        tokens = tokenize_java("class", "1.4")
        assert tokens[0].type == TokenType.KEYWORD

    def test_version_5(self) -> None:
        """``5`` loads the Java 5 grammar (September 2004, generics/enums)."""
        tokens = tokenize_java("class", "5")
        assert tokens[0].type == TokenType.KEYWORD

    def test_version_7(self) -> None:
        """``7`` loads the Java 7 grammar (July 2011, try-with-resources)."""
        tokens = tokenize_java("class", "7")
        assert tokens[0].type == TokenType.KEYWORD

    def test_version_8(self) -> None:
        """``8`` loads the Java 8 grammar (March 2014, lambdas/streams)."""
        tokens = tokenize_java("class", "8")
        assert tokens[0].type == TokenType.KEYWORD

    def test_version_10(self) -> None:
        """``10`` loads the Java 10 grammar (March 2018, var)."""
        tokens = tokenize_java("class", "10")
        assert tokens[0].type == TokenType.KEYWORD

    def test_version_14(self) -> None:
        """``14`` loads the Java 14 grammar (March 2020, switch expressions)."""
        tokens = tokenize_java("class", "14")
        assert tokens[0].type == TokenType.KEYWORD

    def test_version_17(self) -> None:
        """``17`` loads the Java 17 grammar (September 2021, sealed classes)."""
        tokens = tokenize_java("class", "17")
        assert tokens[0].type == TokenType.KEYWORD

    def test_version_21(self) -> None:
        """``21`` loads the Java 21 grammar (September 2023, virtual threads)."""
        tokens = tokenize_java("class", "21")
        assert tokens[0].type == TokenType.KEYWORD

    def test_unknown_version_raises_value_error(self) -> None:
        """An unrecognized version string must raise ``ValueError``."""
        import pytest
        with pytest.raises(ValueError, match="Unknown Java version"):
            tokenize_java("class Hello { }", "99")

    def test_version_propagates_to_factory(self) -> None:
        """``create_java_lexer`` with a version should produce valid tokens."""
        lexer = create_java_lexer("class", "8")
        tokens = lexer.tokenize()
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "class"
        assert tokens[-1].type == TokenType.EOF
