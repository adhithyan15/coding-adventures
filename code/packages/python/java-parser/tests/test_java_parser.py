"""Tests for the Java Parser.

These tests verify that the grammar-driven parser, when loaded with the
``java{version}.grammar`` file, correctly parses Java source code into ASTs.

The test suite covers: basic class declarations, methods, for loops,
if/else, try/catch, and version switching.
"""

from __future__ import annotations

from lang_parser import ASTNode
from lexer import Token, TokenType

from java_parser import create_java_parser, parse_java


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
    """Test parsing of Java class declarations."""

    def test_simple_class(self) -> None:
        """Parse ``class Hello { }`` — a minimal class declaration."""
        ast = parse_java("class Hello { }")
        assert ast.rule_name == "program"

        tokens = find_tokens(ast)
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        assert any(t.value == "class" for t in keywords)

        names = [t for t in tokens if t.type == TokenType.NAME]
        assert any(t.value == "Hello" for t in names)

    def test_public_class(self) -> None:
        """Parse ``public class Main { }`` — a public class declaration."""
        ast = parse_java("public class Main { }")
        assert ast.rule_name == "program"

        tokens = find_tokens(ast)
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        keyword_values = [t.value for t in keywords]
        assert "public" in keyword_values
        assert "class" in keyword_values


# ============================================================================
# Test: Method Declaration
# ============================================================================


class TestMethodDeclaration:
    """Test parsing of Java method declarations."""

    def test_void_method(self) -> None:
        """Parse a simple void method inside a class."""
        source = "void main() { }"
        ast = parse_java(source)
        assert ast.rule_name == "program"

        tokens = find_tokens(ast)
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        assert any(t.value == "void" for t in keywords)

        names = [t for t in tokens if t.type == TokenType.NAME]
        assert any(t.value == "main" for t in names)


# ============================================================================
# Test: For Loop
# ============================================================================


class TestForLoop:
    """Test parsing of Java for loops."""

    def test_for_statement(self) -> None:
        """Parse ``for (int i = 0; i < 10; i = i + 1) { }`` — a basic for loop."""
        source = "for (int i = 0; i < 10; i = i + 1) { }"
        ast = parse_java(source)
        assert ast.rule_name == "program"

        tokens = find_tokens(ast)
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        assert any(t.value == "for" for t in keywords)


# ============================================================================
# Test: If/Else
# ============================================================================


class TestIfElse:
    """Test parsing of Java if/else statements."""

    def test_if_statement(self) -> None:
        """Parse ``if (x > 0) { }`` — a basic if statement."""
        source = "if (x > 0) { }"
        ast = parse_java(source)
        assert ast.rule_name == "program"

        tokens = find_tokens(ast)
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        assert any(t.value == "if" for t in keywords)

    def test_if_else_statement(self) -> None:
        """Parse ``if (x > 0) { } else { }`` — an if/else statement."""
        source = "if (x > 0) { } else { }"
        ast = parse_java(source)
        assert ast.rule_name == "program"

        tokens = find_tokens(ast)
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        keyword_values = [t.value for t in keywords]
        assert "if" in keyword_values
        assert "else" in keyword_values


# ============================================================================
# Test: Try/Catch
# ============================================================================


class TestTryCatch:
    """Test parsing of Java try/catch statements."""

    def test_try_catch(self) -> None:
        """Parse ``try { } catch (Exception e) { }`` — a try/catch block."""
        source = "try { } catch (Exception e) { }"
        ast = parse_java(source)
        assert ast.rule_name == "program"

        tokens = find_tokens(ast)
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        keyword_values = [t.value for t in keywords]
        assert "try" in keyword_values
        assert "catch" in keyword_values


# ============================================================================
# Test: Expression Statements
# ============================================================================


class TestExpressionStatements:
    """Test parsing of bare expression statements."""

    def test_expression_statement(self) -> None:
        """Parse ``1 + 2;`` — an expression statement."""
        ast = parse_java("1 + 2;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateJavaParser:
    """Test the ``create_java_parser()`` factory function."""

    def test_creates_parser(self) -> None:
        """The factory should return a GrammarParser with a parse method."""
        parser = create_java_parser("int x = 1;")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result(self) -> None:
        """The factory should produce the same AST as parse_java()."""
        source = "int x = 1 + 2;"
        ast_direct = parse_java(source)
        ast_factory = create_java_parser(source).parse()

        assert ast_direct.rule_name == ast_factory.rule_name
        assert len(ast_direct.children) == len(ast_factory.children)


# ============================================================================
# Test: Version Parameter
# ============================================================================


class TestVersionParameter:
    """Test that the ``version`` parameter loads the correct Java grammar.

    Each Java version corresponds to both a ``.tokens`` and a ``.grammar``
    file under ``code/grammars/java/``.  The version-aware parser must:

    1. Accept all 10 valid version strings without raising errors.
    2. Still produce a valid AST — ``int x = 1;`` is parseable in every
       Java version, making it the safest cross-version expression.
    3. Raise ``ValueError`` for unknown version strings.
    4. Treat ``None`` and ``""`` as "use the default java21.grammar".
    """

    def test_no_version_uses_default_grammar(self) -> None:
        """Omitting ``version`` (``None``) loads the default java21.grammar."""
        ast = parse_java("int x = 1 + 2;")
        assert ast.rule_name == "program"

    def test_empty_string_uses_default_grammar(self) -> None:
        """An empty string also loads the default java21.grammar."""
        ast = parse_java("int x = 1;", "")
        assert ast.rule_name == "program"

    def test_version_1_0(self) -> None:
        """``1.0`` grammar parses Java 1.0 source correctly."""
        ast = parse_java("int x = 1;", "1.0")
        assert ast.rule_name == "program"

    def test_version_1_1(self) -> None:
        """``1.1`` grammar parses Java 1.1 source correctly."""
        ast = parse_java("int x = 1;", "1.1")
        assert ast.rule_name == "program"

    def test_version_1_4(self) -> None:
        """``1.4`` grammar parses Java 1.4 source correctly."""
        ast = parse_java("int x = 1;", "1.4")
        assert ast.rule_name == "program"

    def test_version_5(self) -> None:
        """``5`` grammar parses Java 5 source correctly."""
        ast = parse_java("int x = 1;", "5")
        assert ast.rule_name == "program"

    def test_version_7(self) -> None:
        """``7`` grammar parses Java 7 source correctly."""
        ast = parse_java("int x = 1;", "7")
        assert ast.rule_name == "program"

    def test_version_8(self) -> None:
        """``8`` grammar parses Java 8 source correctly."""
        ast = parse_java("int x = 1;", "8")
        assert ast.rule_name == "program"

    def test_version_10(self) -> None:
        """``10`` grammar parses Java 10 source correctly."""
        ast = parse_java("int x = 1;", "10")
        assert ast.rule_name == "program"

    def test_version_14(self) -> None:
        """``14`` grammar parses Java 14 source correctly."""
        ast = parse_java("int x = 1;", "14")
        assert ast.rule_name == "program"

    def test_version_17(self) -> None:
        """``17`` grammar parses Java 17 source correctly."""
        ast = parse_java("int x = 1;", "17")
        assert ast.rule_name == "program"

    def test_version_21(self) -> None:
        """``21`` grammar parses Java 21 source correctly."""
        ast = parse_java("int x = 1;", "21")
        assert ast.rule_name == "program"

    def test_unknown_version_raises_value_error(self) -> None:
        """An unrecognized version string must raise ``ValueError``."""
        import pytest
        with pytest.raises(ValueError, match="Unknown Java version"):
            parse_java("int x = 1;", "99")

    def test_version_propagates_to_factory(self) -> None:
        """``create_java_parser`` with a version should produce a valid AST."""
        parser = create_java_parser("int x = 1;", "8")
        ast = parser.parse()
        assert ast.rule_name == "program"
