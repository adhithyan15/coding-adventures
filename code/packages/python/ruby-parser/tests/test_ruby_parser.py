"""Tests for the Ruby Parser.

These tests verify that the grammar-driven parser, when loaded with the
``ruby.grammar`` file, correctly parses Ruby source code into ASTs.

The key insight being tested is that **no new parser code was written**.
The same ``GrammarParser`` that handles Python handles Ruby — only the
grammar file changed. The parser produces generic ``ASTNode`` trees that
reflect the grammar's rule structure.

Understanding the AST Structure
-------------------------------

The grammar-driven parser produces ``ASTNode`` objects where:

- ``rule_name`` is the name of the grammar rule that matched
  (e.g., ``"program"``, ``"assignment"``, ``"expression"``)
- ``children`` is a list of ``ASTNode`` and ``Token`` objects that
  were matched by the rule's body

For example, parsing ``x = 1 + 2`` produces::

    ASTNode(rule_name="program", children=[
        ASTNode(rule_name="statement", children=[
            ASTNode(rule_name="assignment", children=[
                Token(NAME, "x"),
                Token(EQUALS, "="),
                ASTNode(rule_name="expression", children=[
                    ASTNode(rule_name="term", children=[
                        ASTNode(rule_name="factor", children=[
                            Token(NUMBER, "1")
                        ])
                    ]),
                    Token(PLUS, "+"),
                    ASTNode(rule_name="term", children=[
                        ASTNode(rule_name="factor", children=[
                            Token(NUMBER, "2")
                        ])
                    ])
                ])
            ])
        ])
    ])

Test Organization
-----------------

1. **Assignments** — ``x = 1``, ``x = 1 + 2``
2. **Operator precedence** — ``1 + 2 * 3`` vs ``(1 + 2) * 3``
3. **Multiple statements** — programs with more than one statement
4. **Method calls** — ``puts("hello")``, ``add(1, 2)``
5. **Factory function** — ``create_ruby_parser()``
"""

from __future__ import annotations

from lang_parser import ASTNode
from lexer import Token, TokenType

from ruby_parser import create_ruby_parser, parse_ruby


# ============================================================================
# Helpers — make AST inspection more readable
# ============================================================================


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all descendant nodes with the given rule name.

    This is a depth-first search through the AST. It is useful for
    finding specific constructs (like all "factor" nodes) without
    having to manually walk the tree.

    Args:
        node: The root node to search from.
        rule_name: The grammar rule name to search for.

    Returns:
        A list of all matching ASTNode objects, in depth-first order.
    """
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if isinstance(child, ASTNode):
            results.extend(find_nodes(child, rule_name))
    return results


def find_tokens(node: ASTNode) -> list[Token]:
    """Recursively collect all Token leaves from an AST.

    This "flattens" the tree back into a token list, which is useful
    for verifying that all tokens from the source are accounted for
    in the AST.

    Args:
        node: The root node to collect tokens from.

    Returns:
        A list of all Token objects in the tree, in depth-first order.
    """
    tokens: list[Token] = []
    for child in node.children:
        if isinstance(child, Token):
            tokens.append(child)
        elif isinstance(child, ASTNode):
            tokens.extend(find_tokens(child))
    return tokens


# ============================================================================
# Test: Simple Assignments
# ============================================================================


class TestAssignments:
    """Test parsing of Ruby assignment statements.

    Assignments are one of the most fundamental constructs in any
    language: ``variable = expression``. The parser must recognize the
    NAME, EQUALS, and expression parts, and build an AST that reflects
    the assignment structure.
    """

    def test_simple_assignment(self) -> None:
        """Parse ``x = 1`` — the simplest possible assignment.

        Expected AST structure:
            program -> statement -> assignment -> [NAME, EQUALS, expression]
        """
        ast = parse_ruby("x = 1")
        assert ast.rule_name == "program"

        # The program should have one statement child
        statements = find_nodes(ast, "statement")
        assert len(statements) == 1

        # That statement should contain an assignment
        assignments = find_nodes(ast, "assignment")
        assert len(assignments) == 1

        # The assignment should contain a NAME token with value "x"
        assignment_tokens = find_tokens(assignments[0])
        names = [t for t in assignment_tokens if t.type == TokenType.NAME]
        assert len(names) == 1
        assert names[0].value == "x"

    def test_assignment_with_arithmetic(self) -> None:
        """Parse ``x = 1 + 2`` — assignment with a binary expression.

        The right-hand side ``1 + 2`` should be parsed as an expression
        containing two terms connected by a PLUS operator.
        """
        ast = parse_ruby("x = 1 + 2")
        assert ast.rule_name == "program"

        # Find the expression inside the assignment
        expressions = find_nodes(ast, "expression")
        assert len(expressions) >= 1

        # The expression should contain a PLUS token
        expr_tokens = find_tokens(expressions[0])
        plus_tokens = [t for t in expr_tokens if t.type == TokenType.PLUS]
        assert len(plus_tokens) == 1

    def test_assignment_with_string(self) -> None:
        """Parse ``name = "ruby"`` — assignment with a string value."""
        ast = parse_ruby('name = "ruby"')

        assignments = find_nodes(ast, "assignment")
        assert len(assignments) == 1

        tokens = find_tokens(assignments[0])
        string_tokens = [t for t in tokens if t.type == TokenType.STRING]
        assert len(string_tokens) == 1
        assert string_tokens[0].value == "ruby"

    def test_assignment_with_variable(self) -> None:
        """Parse ``y = x`` — assignment from another variable."""
        ast = parse_ruby("y = x")

        assignments = find_nodes(ast, "assignment")
        assert len(assignments) == 1

        tokens = find_tokens(assignments[0])
        names = [t for t in tokens if t.type == TokenType.NAME]
        # Two NAMEs: "y" (target) and "x" (value)
        assert len(names) == 2
        assert names[0].value == "y"
        assert names[1].value == "x"


# ============================================================================
# Test: Operator Precedence
# ============================================================================


class TestOperatorPrecedence:
    """Test that the parser correctly handles operator precedence.

    The grammar encodes precedence through rule nesting:
    - ``expression`` handles ``+`` and ``-`` (lowest precedence)
    - ``term`` handles ``*`` and ``/`` (higher precedence)
    - ``factor`` handles atoms and parentheses (highest precedence)

    This means ``1 + 2 * 3`` parses as ``1 + (2 * 3)`` because ``*``
    is handled at the ``term`` level, which is deeper in the parse tree.
    """

    def test_addition(self) -> None:
        """Parse ``1 + 2`` — simple addition."""
        ast = parse_ruby("1 + 2")

        # Should have an expression with two terms and a PLUS
        expressions = find_nodes(ast, "expression")
        assert len(expressions) >= 1

        tokens = find_tokens(expressions[0])
        values = [t.value for t in tokens]
        assert "1" in values
        assert "+" in values
        assert "2" in values

    def test_multiplication_before_addition(self) -> None:
        """Parse ``1 + 2 * 3`` — multiplication has higher precedence.

        In the AST, ``2 * 3`` should be grouped together inside a ``term``
        node, while ``1`` and ``2 * 3`` are children of the ``expression``.
        """
        ast = parse_ruby("1 + 2 * 3")

        # The top-level expression should have a PLUS
        expressions = find_nodes(ast, "expression")
        expr_direct_tokens = [
            c for c in expressions[0].children if isinstance(c, Token)
        ]
        plus_tokens = [t for t in expr_direct_tokens if t.value == "+"]
        assert len(plus_tokens) == 1

        # There should be term nodes containing the multiplication
        terms = find_nodes(ast, "term")
        # Find the term that contains the STAR
        star_terms = [
            t for t in terms
            if any(
                isinstance(c, Token) and c.value == "*"
                for c in t.children
            )
        ]
        assert len(star_terms) == 1

    def test_parentheses_override_precedence(self) -> None:
        """Parse ``(1 + 2) * 3`` — parentheses override precedence.

        The parentheses force ``1 + 2`` to be computed first. In the AST,
        the addition should appear inside a ``factor`` (which contains
        a parenthesized expression), and the multiplication should be
        at the ``term`` level.
        """
        ast = parse_ruby("(1 + 2) * 3")

        # The outer structure should be a term with STAR
        terms = find_nodes(ast, "term")
        star_terms = [
            t for t in terms
            if any(
                isinstance(c, Token) and c.value == "*"
                for c in t.children
            )
        ]
        assert len(star_terms) == 1

        # Inside should be a factor containing LPAREN, expression, RPAREN
        factors = find_nodes(ast, "factor")
        paren_factors = [
            f for f in factors
            if any(
                isinstance(c, Token) and c.type == TokenType.LPAREN
                for c in f.children
            )
        ]
        assert len(paren_factors) == 1

    def test_subtraction_and_division(self) -> None:
        """Parse ``10 - 4 / 2`` — division has higher precedence than subtraction."""
        ast = parse_ruby("10 - 4 / 2")

        all_tokens = find_tokens(ast)
        values = [t.value for t in all_tokens]
        assert "10" in values
        assert "-" in values
        assert "4" in values
        assert "/" in values
        assert "2" in values

    def test_chained_addition(self) -> None:
        """Parse ``1 + 2 + 3`` — left-associative addition."""
        ast = parse_ruby("1 + 2 + 3")

        expressions = find_nodes(ast, "expression")
        assert len(expressions) >= 1

        # Should have two PLUS tokens in the expression
        expr_tokens = [
            c for c in expressions[0].children
            if isinstance(c, Token) and c.value == "+"
        ]
        assert len(expr_tokens) == 2


# ============================================================================
# Test: Multiple Statements
# ============================================================================


class TestMultipleStatements:
    """Test parsing of programs with multiple statements.

    The ``program`` rule matches ``{ statement }``, meaning zero or more
    statements. The parser must correctly separate statements (typically
    by newlines) and parse each one independently.
    """

    def test_two_assignments(self) -> None:
        """Parse two assignment statements separated by a newline."""
        ast = parse_ruby("x = 1\ny = 2")
        assert ast.rule_name == "program"

        assignments = find_nodes(ast, "assignment")
        assert len(assignments) == 2

    def test_three_statements(self) -> None:
        """Parse three statements of mixed types."""
        ast = parse_ruby("x = 1\ny = x + 2\nz = y * 3")

        assignments = find_nodes(ast, "assignment")
        assert len(assignments) == 3

    def test_expression_statement(self) -> None:
        """Parse a bare expression as a statement (no assignment)."""
        ast = parse_ruby("1 + 2")

        # Should have one expression_stmt
        expr_stmts = find_nodes(ast, "expression_stmt")
        assert len(expr_stmts) == 1

    def test_mixed_assignments_and_expressions(self) -> None:
        """Parse a mix of assignments and expression statements."""
        ast = parse_ruby("x = 5\nx + 1")

        statements = find_nodes(ast, "statement")
        assert len(statements) == 2


# ============================================================================
# Test: Method Calls
# ============================================================================


class TestMethodCalls:
    """Test parsing of Ruby method calls.

    Method calls in our Ruby grammar have the form:
        (NAME | KEYWORD) LPAREN [ expression { COMMA expression } ] RPAREN

    This handles both regular method calls like ``add(1, 2)`` and
    keyword-based calls like ``puts("hello")``.
    """

    def test_puts_with_string(self) -> None:
        """Parse ``puts("hello")`` — the quintessential Ruby output call."""
        ast = parse_ruby('puts("hello")')

        method_calls = find_nodes(ast, "method_call")
        assert len(method_calls) == 1

        tokens = find_tokens(method_calls[0])
        # Should contain: KEYWORD(puts), LPAREN, STRING(hello), RPAREN
        keyword_tokens = [t for t in tokens if t.type == TokenType.KEYWORD]
        assert len(keyword_tokens) == 1
        assert keyword_tokens[0].value == "puts"

        string_tokens = [t for t in tokens if t.type == TokenType.STRING]
        assert len(string_tokens) == 1
        assert string_tokens[0].value == "hello"

    def test_method_call_with_name(self) -> None:
        """Parse ``add(1, 2)`` — a regular method call with two arguments."""
        ast = parse_ruby("add(1, 2)")

        method_calls = find_nodes(ast, "method_call")
        assert len(method_calls) == 1

        tokens = find_tokens(method_calls[0])
        name_tokens = [t for t in tokens if t.type == TokenType.NAME]
        assert name_tokens[0].value == "add"

        number_tokens = [t for t in tokens if t.type == TokenType.NUMBER]
        assert len(number_tokens) == 2
        assert number_tokens[0].value == "1"
        assert number_tokens[1].value == "2"

    def test_method_call_no_args(self) -> None:
        """Parse ``hello()`` — a method call with no arguments."""
        ast = parse_ruby("hello()")

        method_calls = find_nodes(ast, "method_call")
        assert len(method_calls) == 1

    def test_method_call_with_expression_arg(self) -> None:
        """Parse ``print(1 + 2)`` — a method call with an expression argument."""
        ast = parse_ruby("print(1 + 2)")

        method_calls = find_nodes(ast, "method_call")
        assert len(method_calls) == 1

        # The argument should be parsed as an expression containing PLUS
        expressions = find_nodes(method_calls[0], "expression")
        assert len(expressions) >= 1

    def test_puts_with_variable(self) -> None:
        """Parse ``puts(x)`` — puts with a variable argument."""
        ast = parse_ruby("puts(x)")

        method_calls = find_nodes(ast, "method_call")
        assert len(method_calls) == 1

        tokens = find_tokens(method_calls[0])
        names = [t for t in tokens if t.type == TokenType.NAME]
        assert any(t.value == "x" for t in names)


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateRubyParser:
    """Test the ``create_ruby_parser()`` factory function.

    While ``parse_ruby()`` is the simpler interface, ``create_ruby_parser()``
    gives access to the ``GrammarParser`` object, which is useful for
    advanced inspection or integration with custom pipelines.
    """

    def test_creates_parser(self) -> None:
        """The factory should return a GrammarParser with a parse method."""
        parser = create_ruby_parser("x = 1")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result(self) -> None:
        """The factory should produce the same AST as parse_ruby()."""
        source = "x = 1 + 2"
        ast_direct = parse_ruby(source)
        ast_factory = create_ruby_parser(source).parse()

        # Both should have the same rule_name and same number of children
        assert ast_direct.rule_name == ast_factory.rule_name
        assert len(ast_direct.children) == len(ast_factory.children)

    def test_factory_with_method_call(self) -> None:
        """Verify the factory works with method calls."""
        parser = create_ruby_parser('puts("world")')
        ast = parser.parse()
        assert ast.rule_name == "program"

        method_calls = find_nodes(ast, "method_call")
        assert len(method_calls) == 1
