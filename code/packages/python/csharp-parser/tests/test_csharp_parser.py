"""Tests for the C# Parser.

These tests verify that the grammar-driven parser, when loaded with the
``csharp{version}.grammar`` file, correctly parses C# source code into ASTs.

The test suite covers: basic class declarations, namespaced classes, method
declarations, and version switching across all twelve C# versions.

The key insight here is the same as with the lexer: **no new parser code
was written**.  The same ``GrammarParser`` that handles Java, Python, or
JavaScript handles C# — just with a different grammar file.
"""

from __future__ import annotations

import pytest

from lang_parser import ASTNode
from lexer import Token, TokenType

from csharp_parser import create_csharp_parser, parse_csharp


# ============================================================================
# Helpers
# ============================================================================


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all descendant nodes with the given rule name."""
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if isinstance(child, ASTNode):
            results.extend(find_nodes(child, rule_name))
    return results


def find_tokens(node: ASTNode) -> list[Token]:
    """Recursively collect all Token leaves from an AST."""
    tokens: list[Token] = []
    for child in node.children:
        if isinstance(child, Token):
            tokens.append(child)
        elif isinstance(child, ASTNode):
            tokens.extend(find_tokens(child))
    return tokens


# ============================================================================
# Test: Basic Class Declaration
# ============================================================================


class TestClassDeclaration:
    """Test parsing of C# class declarations.

    A C# class declaration looks like::

        class Hello { }

    or with an access modifier::

        public class Main { }

    The parser must recognize the ``class`` keyword and the class name,
    and produce a root ``program`` node.
    """

    def test_simple_class(self) -> None:
        """Parse ``class Hello { }`` — a minimal class declaration."""
        ast = parse_csharp("class Hello { }")
        assert ast.rule_name == "program"

        tokens = find_tokens(ast)
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        assert any(t.value == "class" for t in keywords)

        names = [t for t in tokens if t.type == TokenType.NAME]
        assert any(t.value == "Hello" for t in names)

    def test_public_class(self) -> None:
        """Parse ``public class Main { }`` — a public class declaration.

        In C#, ``public`` is an access modifier that makes the class
        visible to all other code.  Most top-level classes in C# are
        ``public``.
        """
        ast = parse_csharp("public class Main { }")
        assert ast.rule_name == "program"

        tokens = find_tokens(ast)
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        keyword_values = [t.value for t in keywords]
        assert "public" in keyword_values
        assert "class" in keyword_values


# ============================================================================
# Test: Namespaced Class
# ============================================================================


class TestNamespacedClass:
    """Test parsing of C# namespace declarations.

    Namespaces group related types.  A typical C# file looks like::

        namespace MyApp
        {
            public class Greeter { }
        }

    The parser must recognize the ``namespace`` keyword and produce a
    valid AST.
    """

    def test_namespace_declaration(self) -> None:
        """Parse ``namespace MyApp { }`` — a standalone namespace block."""
        ast = parse_csharp("namespace MyApp { }")
        assert ast.rule_name == "program"

        tokens = find_tokens(ast)
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        assert any(t.value == "namespace" for t in keywords)

        names = [t for t in tokens if t.type == TokenType.NAME]
        assert any(t.value == "MyApp" for t in names)


# ============================================================================
# Test: Method Declaration
# ============================================================================


class TestMethodDeclaration:
    """Test parsing of C# method declarations.

    A C# method declaration looks like::

        void Main() { }

    or with a return type and parameters::

        int Add(int a, int b) { return a + b; }

    The parser must recognize the method name, parameter list, and body.
    """

    def test_void_method(self) -> None:
        """Parse a simple void method — the most basic C# method form."""
        source = "void Main() { }"
        ast = parse_csharp(source)
        assert ast.rule_name == "program"

        tokens = find_tokens(ast)
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        assert any(t.value == "void" for t in keywords)

        names = [t for t in tokens if t.type == TokenType.NAME]
        assert any(t.value == "Main" for t in names)


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateCSharpParser:
    """Test the ``create_csharp_parser()`` factory function.

    The factory is useful when you need access to the ``GrammarParser``
    object before calling ``.parse()`` — for example, to inspect the
    grammar or to call ``.parse()`` multiple times.
    """

    def test_creates_parser_with_parse_method(self) -> None:
        """The factory must return an object with a ``parse()`` method."""
        parser = create_csharp_parser("int x = 1;")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result_as_parse_csharp(self) -> None:
        """The factory and parse_csharp() must produce equivalent ASTs."""
        source = "class Foo { }"
        ast_direct = parse_csharp(source)
        ast_factory = create_csharp_parser(source).parse()

        assert ast_direct.rule_name == ast_factory.rule_name
        assert len(ast_direct.children) == len(ast_factory.children)

    def test_factory_with_version(self) -> None:
        """The factory must accept a version string."""
        parser = create_csharp_parser("public class Foo { }", "8.0")
        ast = parser.parse()
        assert ast.rule_name == "program"


# ============================================================================
# Test: Version Parameter
# ============================================================================


class TestVersionParameter:
    """Test that the ``version`` parameter loads the correct C# grammar.

    Each C# version corresponds to both a ``.tokens`` and a ``.grammar``
    file under ``code/grammars/csharp/``.  The version-aware parser must:

    1. Accept all 12 valid version strings without raising errors.
    2. Still produce a valid AST — ``class Foo { }`` is parseable in
       every C# version, making it the safest cross-version expression.
    3. Raise ``ValueError`` for unknown version strings.
    4. Treat ``None`` and ``""`` as "use the default csharp12.0.grammar".
    """

    def test_no_version_uses_default_grammar(self) -> None:
        """Omitting ``version`` (``None``) loads the default csharp12.0.grammar."""
        ast = parse_csharp("class Foo { }")
        assert ast.rule_name == "program"

    def test_empty_string_uses_default_grammar(self) -> None:
        """An empty string also loads the default csharp12.0.grammar."""
        ast = parse_csharp("class Foo { }", "")
        assert ast.rule_name == "program"

    def test_version_1_0(self) -> None:
        """``1.0`` grammar — C# 1.0 (2002): original release."""
        ast = parse_csharp("class Foo { }", "1.0")
        assert ast.rule_name == "program"

    def test_version_2_0(self) -> None:
        """``2.0`` grammar — C# 2.0 (2005): generics, nullable types."""
        ast = parse_csharp("class Foo { }", "2.0")
        assert ast.rule_name == "program"

    def test_version_3_0(self) -> None:
        """``3.0`` grammar — C# 3.0 (2007): LINQ, lambdas, var."""
        ast = parse_csharp("class Foo { }", "3.0")
        assert ast.rule_name == "program"

    def test_version_4_0(self) -> None:
        """``4.0`` grammar — C# 4.0 (2010): dynamic, optional params."""
        ast = parse_csharp("class Foo { }", "4.0")
        assert ast.rule_name == "program"

    def test_version_5_0(self) -> None:
        """``5.0`` grammar — C# 5.0 (2012): async/await."""
        ast = parse_csharp("class Foo { }", "5.0")
        assert ast.rule_name == "program"

    def test_version_6_0(self) -> None:
        """``6.0`` grammar — C# 6.0 (2015): ?., $"", nameof."""
        ast = parse_csharp("class Foo { }", "6.0")
        assert ast.rule_name == "program"

    def test_version_7_0(self) -> None:
        """``7.0`` grammar — C# 7.0 (2017): tuples, patterns."""
        ast = parse_csharp("class Foo { }", "7.0")
        assert ast.rule_name == "program"

    def test_version_8_0(self) -> None:
        """``8.0`` grammar — C# 8.0 (2019): nullable refs, switch exprs."""
        ast = parse_csharp("class Foo { }", "8.0")
        assert ast.rule_name == "program"

    def test_version_9_0(self) -> None:
        """``9.0`` grammar — C# 9.0 (2020): records, init, top-level."""
        ast = parse_csharp("class Foo { }", "9.0")
        assert ast.rule_name == "program"

    def test_version_10_0(self) -> None:
        """``10.0`` grammar — C# 10.0 (2021): record struct, global using."""
        ast = parse_csharp("class Foo { }", "10.0")
        assert ast.rule_name == "program"

    def test_version_11_0(self) -> None:
        """``11.0`` grammar — C# 11.0 (2022): required, raw strings, file."""
        ast = parse_csharp("class Foo { }", "11.0")
        assert ast.rule_name == "program"

    def test_version_12_0(self) -> None:
        """``12.0`` grammar — C# 12.0 (2023): primary constructors."""
        ast = parse_csharp("class Foo { }", "12.0")
        assert ast.rule_name == "program"

    def test_unknown_version_raises_value_error(self) -> None:
        """An unrecognized version string must raise ``ValueError``."""
        with pytest.raises(ValueError, match="Unknown C# version"):
            parse_csharp("class Foo { }", "99.0")

    def test_version_propagates_to_factory(self) -> None:
        """``create_csharp_parser`` with a version should produce a valid AST."""
        parser = create_csharp_parser("class Foo { }", "8.0")
        ast = parser.parse()
        assert ast.rule_name == "program"
