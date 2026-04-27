"""Tests for the C# Lexer.

These tests verify that the grammar-driven lexer, when loaded with the
``csharp{version}.tokens`` grammar file, correctly tokenizes C# source code.

The key insight being tested here is that **no new lexer code was written**.
The same ``GrammarLexer`` that handles Python, JavaScript, and Java handles
C# — only the grammar file changed.

C# is interesting to test because it has several operators that are absent
in most languages:

- ``?.``  — null-conditional member access (``obj?.Property``)
- ``??``  — null-coalescing operator (``x ?? defaultValue``)
- ``??=`` — null-coalescing assignment (``x ??= defaultValue``)
- ``=>``  — lambda arrow AND expression-bodied member separator
- ``@``   — verbatim identifier prefix (``@class`` means an identifier
  named "class" rather than the keyword)
- ``$``   — interpolated string prefix (``$"Hello {name}"``)

Twelve C# versions are tested — from the original 1.0 release in 2002
through the modern 12.0 release in 2023.
"""

from __future__ import annotations

import pytest

from lexer import Token, TokenType

from csharp_lexer import create_csharp_lexer, tokenize_csharp


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
# Test: Basic C# Expressions
# ============================================================================


class TestBasicExpressions:
    """Test that simple C# expressions tokenize correctly."""

    def test_class_declaration(self) -> None:
        """Tokenize ``public class Hello { }`` — a basic class declaration.

        C# classes always start with an access modifier.  The lexer must
        recognize ``public`` as a KEYWORD, ``class`` as a KEYWORD,
        ``Hello`` as a NAME, and the braces as delimiters.
        """
        tokens = tokenize_csharp("public class Hello { }")
        types = token_types(tokens)
        values = token_values(tokens)
        assert "KEYWORD" in types
        assert "public" in values
        assert "class" in values
        assert "Hello" in values
        assert tokens[-1].type == TokenType.EOF

    def test_variable_declaration(self) -> None:
        """Tokenize ``int x = 42;`` — a local variable declaration.

        C# is statically typed.  ``int`` is a built-in type keyword, ``x``
        is an identifier, ``=`` is the assignment operator, ``42`` is an
        integer literal, and ``;`` terminates the statement.
        """
        tokens = tokenize_csharp("int x = 42;")
        types = token_types(tokens)
        values = token_values(tokens)
        assert "KEYWORD" in types
        assert "int" in values
        assert "x" in values
        assert "42" in values
        assert ";" in values

    def test_arithmetic_operators(self) -> None:
        """Tokenize ``a + b - c * d / e`` — the four arithmetic operators."""
        tokens = tokenize_csharp("a + b - c * d / e")
        assert token_types(tokens) == [
            "NAME", "PLUS", "NAME", "MINUS", "NAME",
            "STAR", "NAME", "SLASH", "NAME", "EOF",
        ]

    def test_parenthesized_expression(self) -> None:
        """Tokenize ``(1 + 2) * 3`` — parentheses for grouping."""
        tokens = tokenize_csharp("(1 + 2) * 3")
        assert token_types(tokens) == [
            "LPAREN", "NUMBER", "PLUS", "NUMBER", "RPAREN",
            "STAR", "NUMBER", "EOF",
        ]

    def test_namespace_declaration(self) -> None:
        """Tokenize ``namespace MyApp { }`` — a C# namespace block.

        Namespaces are unique to C# (and a few other .NET languages).  They
        group related types.  The lexer must recognize ``namespace`` as a
        KEYWORD.
        """
        tokens = tokenize_csharp("namespace MyApp { }")
        values = token_values(tokens)
        assert "namespace" in values

    def test_using_directive(self) -> None:
        """Tokenize ``using System;`` — a C# using directive.

        The ``using`` directive imports a namespace so its members can be
        used without qualification.  It is one of the most common lines
        in any C# file.
        """
        tokens = tokenize_csharp("using System;")
        values = token_values(tokens)
        assert "using" in values
        assert "System" in values
        assert ";" in values


# ============================================================================
# Test: C# Keywords
# ============================================================================


class TestCSharpKeywords:
    """Test that C# keywords are recognized as KEYWORD tokens."""

    def test_public_keyword(self) -> None:
        """``public`` is an access modifier keyword."""
        tokens = tokenize_csharp("public")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "public"

    def test_class_keyword(self) -> None:
        """``class`` declares reference types in C#."""
        tokens = tokenize_csharp("class")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "class"

    def test_void_keyword(self) -> None:
        """``void`` means a method returns no value."""
        tokens = tokenize_csharp("void")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "void"

    def test_namespace_keyword(self) -> None:
        """``namespace`` groups related types and prevents name collisions."""
        tokens = tokenize_csharp("namespace")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "namespace"

    def test_using_keyword(self) -> None:
        """``using`` imports namespaces or wraps disposable resources."""
        tokens = tokenize_csharp("using")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "using"

    def test_int_keyword(self) -> None:
        """``int`` is the 32-bit signed integer built-in type."""
        tokens = tokenize_csharp("int")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "int"

    def test_string_keyword(self) -> None:
        """``string`` is the built-in alias for ``System.String``."""
        tokens = tokenize_csharp("string")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "string"

    def test_bool_keyword(self) -> None:
        """``bool`` is the built-in alias for ``System.Boolean``."""
        tokens = tokenize_csharp("bool")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "bool"

    def test_return_keyword(self) -> None:
        """``return`` exits a method and optionally returns a value."""
        tokens = tokenize_csharp("return")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "return"

    def test_new_keyword(self) -> None:
        """``new`` allocates a new object on the managed heap."""
        tokens = tokenize_csharp("new")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "new"

    def test_boolean_literals(self) -> None:
        """C# uses ``true``, ``false``, and ``null`` as keyword literals."""
        tokens = tokenize_csharp("true false null")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["true", "false", "null"]

    def test_keyword_vs_name(self) -> None:
        """A keyword embedded in a longer name should NOT be a KEYWORD token.

        For example, ``classes`` starts with ``class`` but it is an
        identifier, not a keyword.  The lexer must prefer the longest
        match.
        """
        tokens = tokenize_csharp("classes")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "classes"

    def test_if_else_keywords(self) -> None:
        """``if`` and ``else`` control flow keywords."""
        tokens = tokenize_csharp("if else")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert "if" in keywords
        assert "else" in keywords

    def test_for_foreach_while_keywords(self) -> None:
        """C# loop keywords: ``for``, ``foreach``, ``while``, ``do``."""
        tokens = tokenize_csharp("for foreach while do")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert "for" in keywords
        assert "foreach" in keywords
        assert "while" in keywords
        assert "do" in keywords


# ============================================================================
# Test: C# Operators
# ============================================================================


class TestCSharpOperators:
    """Test C#-specific operators that differ from other languages."""

    def test_equality(self) -> None:
        """The ``==`` operator tests equality."""
        tokens = tokenize_csharp("x == 1")
        assert tokens[1].value == "=="

    def test_inequality(self) -> None:
        """The ``!=`` operator tests inequality."""
        tokens = tokenize_csharp("x != 1")
        assert tokens[1].value == "!="

    def test_less_than_or_equal(self) -> None:
        """The ``<=`` operator."""
        tokens = tokenize_csharp("x <= 1")
        assert tokens[1].value == "<="

    def test_greater_than_or_equal(self) -> None:
        """The ``>=`` operator."""
        tokens = tokenize_csharp("x >= 1")
        assert tokens[1].value == ">="

    def test_lambda_arrow(self) -> None:
        """The ``=>`` lambda arrow is unique to C# (and related languages).

        It appears in lambda expressions::

            Func<int, int> square = x => x * x;

        and in expression-bodied members::

            int Add(int a, int b) => a + b;
        """
        tokens = tokenize_csharp("x => x * 2")
        values = token_values(tokens)
        assert "=>" in values

    def test_null_coalescing_operator(self) -> None:
        """The ``??`` null-coalescing operator provides a default value.

        ``x ?? defaultValue`` returns ``x`` if it is non-null, otherwise
        it returns ``defaultValue``.  This is a C#-specific operator that
        significantly reduces null-check boilerplate.
        """
        tokens = tokenize_csharp("x ?? defaultValue")
        values = token_values(tokens)
        assert "??" in values

    def test_null_conditional_member_access(self) -> None:
        """The ``?.`` null-conditional operator avoids NullReferenceException.

        ``obj?.Property`` returns ``null`` if ``obj`` is null, instead of
        throwing an exception.  This is one of C# 6.0's most useful
        additions.
        """
        tokens = tokenize_csharp("obj?.Property")
        values = token_values(tokens)
        assert "?." in values

    def test_increment_decrement(self) -> None:
        """The ``++`` and ``--`` postfix/prefix operators."""
        tokens = tokenize_csharp("i++ j--")
        values = token_values(tokens)
        assert "++" in values
        assert "--" in values


# ============================================================================
# Test: Delimiters
# ============================================================================


class TestDelimiters:
    """Test that C# delimiters tokenize to the right token types."""

    def test_curly_braces(self) -> None:
        """``{`` and ``}`` delimit blocks: class bodies, method bodies, etc."""
        tokens = tokenize_csharp("{ }")
        types = token_types(tokens)
        assert "LBRACE" in types
        assert "RBRACE" in types

    def test_parentheses(self) -> None:
        """``(`` and ``)`` surround argument lists and conditions."""
        tokens = tokenize_csharp("( )")
        types = token_types(tokens)
        assert "LPAREN" in types
        assert "RPAREN" in types

    def test_square_brackets(self) -> None:
        """``[`` and ``]`` are used for array indexing and attributes."""
        tokens = tokenize_csharp("[ ]")
        types = token_types(tokens)
        assert "LBRACKET" in types
        assert "RBRACKET" in types

    def test_semicolon(self) -> None:
        """```;``` terminates statements in C#."""
        tokens = tokenize_csharp("x = 1;")
        values = token_values(tokens)
        assert ";" in values

    def test_comma(self) -> None:
        """```,``` separates arguments, parameters, and list elements."""
        tokens = tokenize_csharp("a, b, c")
        values = token_values(tokens)
        assert "," in values

    def test_dot(self) -> None:
        """```.``` is the member access operator."""
        tokens = tokenize_csharp("Console.WriteLine")
        values = token_values(tokens)
        assert "." in values


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateCSharpLexer:
    """Test the ``create_csharp_lexer()`` factory function."""

    def test_returns_lexer_with_tokenize_method(self) -> None:
        """The factory must return an object with a ``tokenize()`` method."""
        lexer = create_csharp_lexer("public class Foo { }")
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_tokens_as_tokenize_csharp(self) -> None:
        """The factory and the convenience function must agree."""
        source = "int x = 1 + 2;"
        direct_tokens = tokenize_csharp(source)
        factory_tokens = create_csharp_lexer(source).tokenize()
        assert token_values(direct_tokens) == token_values(factory_tokens)

    def test_factory_with_version(self) -> None:
        """The factory should accept a version string."""
        lexer = create_csharp_lexer("public class Foo { }", "8.0")
        tokens = lexer.tokenize()
        assert tokens[-1].type == TokenType.EOF


# ============================================================================
# Test: Version Parameter
# ============================================================================


class TestVersionParameter:
    """Test that the ``version`` parameter loads the correct grammar.

    Each C# version corresponds to a ``.tokens`` file under
    ``code/grammars/csharp/``.  The version-aware tokenizer must:

    1. Accept all 12 valid version strings without raising errors.
    2. Still produce a valid token stream — ``int x = 1;`` is tokenizable
       in every C# version, making it the safest cross-version expression.
    3. Raise ``ValueError`` for unknown version strings.
    4. Treat ``None`` and ``""`` as "use the default csharp12.0.tokens".
    """

    def test_no_version_uses_default(self) -> None:
        """Omitting ``version`` (``None``) loads the default csharp12.0.tokens."""
        tokens = tokenize_csharp("int x = 42;")
        assert tokens[-1].type == TokenType.EOF

    def test_empty_string_uses_default(self) -> None:
        """An empty string also loads the default csharp12.0.tokens."""
        tokens = tokenize_csharp("int x = 42;", "")
        assert tokens[-1].type == TokenType.EOF

    def test_version_1_0(self) -> None:
        """``1.0`` grammar — C# 1.0 (2002)."""
        tokens = tokenize_csharp("int x = 1;", "1.0")
        assert tokens[-1].type == TokenType.EOF

    def test_version_2_0(self) -> None:
        """``2.0`` grammar — C# 2.0 (2005): generics, nullable types."""
        tokens = tokenize_csharp("int x = 1;", "2.0")
        assert tokens[-1].type == TokenType.EOF

    def test_version_3_0(self) -> None:
        """``3.0`` grammar — C# 3.0 (2007): LINQ, lambdas, var."""
        tokens = tokenize_csharp("int x = 1;", "3.0")
        assert tokens[-1].type == TokenType.EOF

    def test_version_4_0(self) -> None:
        """``4.0`` grammar — C# 4.0 (2010): dynamic, optional params."""
        tokens = tokenize_csharp("int x = 1;", "4.0")
        assert tokens[-1].type == TokenType.EOF

    def test_version_5_0(self) -> None:
        """``5.0`` grammar — C# 5.0 (2012): async/await."""
        tokens = tokenize_csharp("int x = 1;", "5.0")
        assert tokens[-1].type == TokenType.EOF

    def test_version_6_0(self) -> None:
        """``6.0`` grammar — C# 6.0 (2015): ?., $"", nameof."""
        tokens = tokenize_csharp("int x = 1;", "6.0")
        assert tokens[-1].type == TokenType.EOF

    def test_version_7_0(self) -> None:
        """``7.0`` grammar — C# 7.0 (2017): tuples, out vars, patterns."""
        tokens = tokenize_csharp("int x = 1;", "7.0")
        assert tokens[-1].type == TokenType.EOF

    def test_version_8_0(self) -> None:
        """``8.0`` grammar — C# 8.0 (2019): nullable refs, switch exprs."""
        tokens = tokenize_csharp("int x = 1;", "8.0")
        assert tokens[-1].type == TokenType.EOF

    def test_version_9_0(self) -> None:
        """``9.0`` grammar — C# 9.0 (2020): records, init, top-level."""
        tokens = tokenize_csharp("int x = 1;", "9.0")
        assert tokens[-1].type == TokenType.EOF

    def test_version_10_0(self) -> None:
        """``10.0`` grammar — C# 10.0 (2021): record struct, global using."""
        tokens = tokenize_csharp("int x = 1;", "10.0")
        assert tokens[-1].type == TokenType.EOF

    def test_version_11_0(self) -> None:
        """``11.0`` grammar — C# 11.0 (2022): required, raw strings."""
        tokens = tokenize_csharp("int x = 1;", "11.0")
        assert tokens[-1].type == TokenType.EOF

    def test_version_12_0(self) -> None:
        """``12.0`` grammar — C# 12.0 (2023): primary constructors."""
        tokens = tokenize_csharp("int x = 1;", "12.0")
        assert tokens[-1].type == TokenType.EOF

    def test_unknown_version_raises_value_error(self) -> None:
        """An unrecognized version string must raise ``ValueError``."""
        with pytest.raises(ValueError, match="Unknown C# version"):
            tokenize_csharp("int x = 1;", "99.0")

    def test_unknown_version_message_lists_valid(self) -> None:
        """The error message must include the list of valid versions."""
        with pytest.raises(ValueError, match="Valid versions"):
            tokenize_csharp("int x = 1;", "13.0")
