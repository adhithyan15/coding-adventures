"""Tests for the JavaScript Parser.

These tests verify that the grammar-driven parser, when loaded with the
``javascript.grammar`` file, correctly parses JavaScript source code into ASTs.
"""

from __future__ import annotations

from lang_parser import ASTNode
from lexer import Token, TokenType

from javascript_parser import create_javascript_parser, parse_javascript


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
    """Test parsing of JavaScript variable declarations."""

    def test_let_declaration(self) -> None:
        """Parse ``let x = 1 + 2;`` — a let variable declaration."""
        ast = parse_javascript("let x = 1 + 2;")
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
        ast = parse_javascript("const y = 42;")

        var_decls = find_nodes(ast, "var_declaration")
        assert len(var_decls) == 1

        tokens = find_tokens(var_decls[0])
        keywords = [t for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords[0].value == "const"


# ============================================================================
# Test: Assignments
# ============================================================================


class TestAssignments:
    """Test parsing of JavaScript assignment statements."""

    def test_simple_assignment(self) -> None:
        """Parse ``x = 5;`` — a simple assignment."""
        ast = parse_javascript("x = 5;")

        assignments = find_nodes(ast, "assignment")
        assert len(assignments) == 1


# ============================================================================
# Test: Expression Statements
# ============================================================================


class TestExpressionStatements:
    """Test parsing of bare expression statements."""

    def test_expression_statement(self) -> None:
        """Parse ``1 + 2;`` — an expression statement."""
        ast = parse_javascript("1 + 2;")

        expr_stmts = find_nodes(ast, "expression_stmt")
        assert len(expr_stmts) == 1


# ============================================================================
# Test: Operator Precedence
# ============================================================================


class TestOperatorPrecedence:
    """Test that the parser correctly handles operator precedence."""

    def test_multiplication_before_addition(self) -> None:
        """Parse ``1 + 2 * 3;`` — multiplication has higher precedence."""
        ast = parse_javascript("1 + 2 * 3;")

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
        ast = parse_javascript("let x = 1;let y = 2;")

        var_decls = find_nodes(ast, "var_declaration")
        assert len(var_decls) == 2


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateJavaScriptParser:
    """Test the ``create_javascript_parser()`` factory function."""

    def test_creates_parser(self) -> None:
        """The factory should return a GrammarParser with a parse method."""
        parser = create_javascript_parser("let x = 1;")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result(self) -> None:
        """The factory should produce the same AST as parse_javascript()."""
        source = "let x = 1 + 2;"
        ast_direct = parse_javascript(source)
        ast_factory = create_javascript_parser(source).parse()

        assert ast_direct.rule_name == ast_factory.rule_name
        assert len(ast_direct.children) == len(ast_factory.children)


# ============================================================================
# Test: Version Parameter
# ============================================================================


class TestVersionParameter:
    """Test that the ``version`` parameter loads the correct ECMAScript grammar.

    Each ECMAScript version corresponds to both a ``.tokens`` and a ``.grammar``
    file under ``code/grammars/ecmascript/``.  The version-aware parser must:

    1. Accept all 14 valid version strings without raising errors.
    2. Still produce a valid AST — ``var x = 1;`` is parseable in every ES version
       (``var`` is ES1+, making it the safest cross-version expression).
    3. Raise ``ValueError`` for unknown version strings.
    4. Treat ``None`` and ``""`` as "use the generic javascript.grammar".
    """

    def test_no_version_uses_generic_grammar(self) -> None:
        """Omitting ``version`` (``None``) loads the generic javascript.grammar."""
        ast = parse_javascript("let x = 1 + 2;")
        assert ast.rule_name == "program"

    def test_empty_string_uses_generic_grammar(self) -> None:
        """An empty string also loads the generic javascript.grammar."""
        ast = parse_javascript("let x = 1;", "")
        assert ast.rule_name == "program"

    def test_es1_version(self) -> None:
        """``es1`` grammar parses ECMAScript 1 source correctly."""
        ast = parse_javascript("var x = 1;", "es1")
        assert ast.rule_name == "program"

    def test_es3_version(self) -> None:
        """``es3`` grammar parses ECMAScript 3 source correctly."""
        ast = parse_javascript("var x = 1;", "es3")
        assert ast.rule_name == "program"

    def test_es5_version(self) -> None:
        """``es5`` grammar parses ECMAScript 5 source correctly."""
        ast = parse_javascript("var x = 1;", "es5")
        assert ast.rule_name == "program"

    def test_es2015_version(self) -> None:
        """``es2015`` grammar parses ES2015 source correctly."""
        ast = parse_javascript("var x = 1;", "es2015")
        assert ast.rule_name == "program"

    def test_es2016_version(self) -> None:
        """``es2016`` grammar parses ES2016 source correctly."""
        ast = parse_javascript("var x = 1;", "es2016")
        assert ast.rule_name == "program"

    def test_es2017_version(self) -> None:
        """``es2017`` grammar parses ES2017 source correctly."""
        ast = parse_javascript("var x = 1;", "es2017")
        assert ast.rule_name == "program"

    def test_es2018_version(self) -> None:
        """``es2018`` grammar parses ES2018 source correctly."""
        ast = parse_javascript("var x = 1;", "es2018")
        assert ast.rule_name == "program"

    def test_es2019_version(self) -> None:
        """``es2019`` grammar parses ES2019 source correctly."""
        ast = parse_javascript("var x = 1;", "es2019")
        assert ast.rule_name == "program"

    def test_es2020_version(self) -> None:
        """``es2020`` grammar parses ES2020 source correctly."""
        ast = parse_javascript("var x = 1;", "es2020")
        assert ast.rule_name == "program"

    def test_es2021_version(self) -> None:
        """``es2021`` grammar parses ES2021 source correctly."""
        ast = parse_javascript("var x = 1;", "es2021")
        assert ast.rule_name == "program"

    def test_es2022_version(self) -> None:
        """``es2022`` grammar parses ES2022 source correctly."""
        ast = parse_javascript("var x = 1;", "es2022")
        assert ast.rule_name == "program"

    def test_es2023_version(self) -> None:
        """``es2023`` grammar parses ES2023 source correctly."""
        ast = parse_javascript("var x = 1;", "es2023")
        assert ast.rule_name == "program"

    def test_es2024_version(self) -> None:
        """``es2024`` grammar parses ES2024 source correctly."""
        ast = parse_javascript("var x = 1;", "es2024")
        assert ast.rule_name == "program"

    def test_es2025_version(self) -> None:
        """``es2025`` grammar parses ES2025 source correctly."""
        ast = parse_javascript("var x = 1;", "es2025")
        assert ast.rule_name == "program"

    def test_unknown_version_raises_value_error(self) -> None:
        """An unrecognized version string must raise ``ValueError``."""
        import pytest
        with pytest.raises(ValueError, match="Unknown ECMAScript version"):
            parse_javascript("var x = 1;", "es99")

    def test_version_propagates_to_factory(self) -> None:
        """``create_javascript_parser`` with a version should produce a valid AST."""
        parser = create_javascript_parser("var x = 1;", "es5")
        ast = parser.parse()
        assert ast.rule_name == "program"
