"""Tests for the TypeScript Parser.

These tests verify that the grammar-driven parser, when loaded with the
``typescript.grammar`` file, correctly parses TypeScript source code into ASTs.
"""

from __future__ import annotations

from lang_parser import ASTNode
from lexer import Token, TokenType

from typescript_parser import create_typescript_parser, parse_typescript


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
# Test: Variable Declarations
# ============================================================================


class TestVarDeclarations:
    """Test parsing of TypeScript variable declarations."""

    def test_let_declaration(self) -> None:
        """Parse ``let x = 1 + 2;`` — a let variable declaration."""
        ast = parse_typescript("let x = 1 + 2;")
        assert ast.rule_name == "program"

        var_decls = find_nodes(ast, "var_declaration")
        assert len(var_decls) == 1

        tokens = find_tokens(var_decls[0])
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords[0].value == "let"

        names = [t for t in tokens if t.type == TokenType.NAME]
        assert names[0].value == "x"

    def test_const_declaration(self) -> None:
        """Parse ``const y = 42;`` — a const variable declaration."""
        ast = parse_typescript("const y = 42;")

        var_decls = find_nodes(ast, "var_declaration")
        assert len(var_decls) == 1

        tokens = find_tokens(var_decls[0])
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords[0].value == "const"


# ============================================================================
# Test: Assignments
# ============================================================================


class TestAssignments:
    """Test parsing of TypeScript assignment statements."""

    def test_simple_assignment(self) -> None:
        """Parse ``x = 5;`` — a simple assignment."""
        ast = parse_typescript("x = 5;")

        assignments = find_nodes(ast, "assignment")
        assert len(assignments) == 1


# ============================================================================
# Test: Expression Statements
# ============================================================================


class TestExpressionStatements:
    """Test parsing of bare expression statements."""

    def test_expression_statement(self) -> None:
        """Parse ``1 + 2;`` — an expression statement."""
        ast = parse_typescript("1 + 2;")

        expr_stmts = find_nodes(ast, "expression_stmt")
        assert len(expr_stmts) == 1


# ============================================================================
# Test: Operator Precedence
# ============================================================================


class TestOperatorPrecedence:
    """Test that the parser correctly handles operator precedence."""

    def test_multiplication_before_addition(self) -> None:
        """Parse ``1 + 2 * 3;`` — multiplication has higher precedence."""
        ast = parse_typescript("1 + 2 * 3;")

        terms = find_nodes(ast, "term")
        star_terms = [
            t for t in terms
            if any(
                isinstance(c, Token) and c.value == "*"
                for c in t.children
            )
        ]
        assert len(star_terms) == 1


# ============================================================================
# Test: Multiple Statements
# ============================================================================


class TestMultipleStatements:
    """Test parsing of programs with multiple statements."""

    def test_two_var_declarations(self) -> None:
        """Parse two variable declarations."""
        ast = parse_typescript("let x = 1;let y = 2;")

        var_decls = find_nodes(ast, "var_declaration")
        assert len(var_decls) == 2


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTypescriptParser:
    """Test the ``create_typescript_parser()`` factory function."""

    def test_creates_parser(self) -> None:
        """The factory should return a GrammarParser with a parse method."""
        parser = create_typescript_parser("let x = 1;")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result(self) -> None:
        """The factory should produce the same AST as parse_typescript()."""
        source = "let x = 1 + 2;"
        ast_direct = parse_typescript(source)
        ast_factory = create_typescript_parser(source).parse()

        assert ast_direct.rule_name == ast_factory.rule_name
        assert len(ast_direct.children) == len(ast_factory.children)


# ============================================================================
# Test: Version Parameter
# ============================================================================


class TestVersionParameter:
    """Test that the ``version`` parameter loads the correct versioned grammar.

    Each TypeScript version corresponds to both a ``.tokens`` file and a
    ``.grammar`` file under ``code/grammars/typescript/``.  The version-aware
    parser must:

    1. Accept all six valid version strings without raising errors.
    2. Still produce a valid AST — core constructs like ``var_declaration``
       exist in all versioned grammars.
    3. Raise ``ValueError`` for unknown version strings.
    4. Treat ``None`` and ``""`` as "use the generic grammar".
    """

    def test_no_version_uses_generic_grammar(self) -> None:
        """Omitting ``version`` (``None``) loads the generic typescript.grammar."""
        ast = parse_typescript("let x = 1 + 2;")
        assert ast.rule_name == "program"

    def test_empty_string_uses_generic_grammar(self) -> None:
        """An empty string version also loads the generic typescript.grammar."""
        ast = parse_typescript("let x = 1;", "")
        assert ast.rule_name == "program"

    def test_ts10_version(self) -> None:
        """``ts1.0`` grammar parses TypeScript 1.0 source correctly."""
        ast = parse_typescript("var x = 1;", "ts1.0")
        assert ast.rule_name == "program"

    def test_ts20_version(self) -> None:
        """``ts2.0`` grammar parses TypeScript 2.0 source correctly."""
        ast = parse_typescript("let x = 1;", "ts2.0")
        assert ast.rule_name == "program"

    def test_ts30_version(self) -> None:
        """``ts3.0`` grammar parses TypeScript 3.0 source correctly."""
        ast = parse_typescript("const y = 2;", "ts3.0")
        assert ast.rule_name == "program"

    def test_ts40_version(self) -> None:
        """``ts4.0`` grammar parses TypeScript 4.0 source correctly."""
        ast = parse_typescript("let x = 1;", "ts4.0")
        assert ast.rule_name == "program"

    def test_ts50_version(self) -> None:
        """``ts5.0`` grammar parses TypeScript 5.0 source correctly."""
        ast = parse_typescript("const x = 42;", "ts5.0")
        assert ast.rule_name == "program"

    def test_ts58_version(self) -> None:
        """``ts5.8`` grammar parses TypeScript 5.8 source correctly."""
        ast = parse_typescript("let x = 1 + 2;", "ts5.8")
        assert ast.rule_name == "program"

    def test_unknown_version_raises_value_error(self) -> None:
        """An unrecognized version string must raise ``ValueError``."""
        import pytest
        with pytest.raises(ValueError, match="Unknown TypeScript version"):
            parse_typescript("let x = 1;", "ts99.0")

    def test_version_propagates_to_factory(self) -> None:
        """``create_typescript_parser`` with a version should produce a valid AST."""
        # TypeScript 1.0 is ES5-based; ``var`` is the canonical declaration
        # keyword (``let``/``const`` are context keywords, not hard keywords).
        parser = create_typescript_parser("var x = 1;", "ts1.0")
        ast = parser.parse()
        assert ast.rule_name == "program"
