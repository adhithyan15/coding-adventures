"""Tests for the Starlark Parser.

These tests verify that the grammar-driven parser, when loaded with the
``starlark.grammar`` file, correctly parses Starlark source code into ASTs.

The key insight being tested is that **no new parser code was written**.
The same ``GrammarParser`` that handles Python and Ruby handles Starlark —
only the grammar file changed. The parser produces generic ``ASTNode`` trees
that reflect the grammar's rule structure.

Understanding the Starlark AST Structure
-----------------------------------------

The grammar-driven parser produces ``ASTNode`` objects where:

- ``rule_name`` is the name of the grammar rule that matched
  (e.g., ``"file"``, ``"assign_stmt"``, ``"expression"``)
- ``children`` is a list of ``ASTNode`` and ``Token`` objects that
  were matched by the rule's body

The top-level rule is ``file`` (not ``program`` as in Ruby). A file
contains statements, each of which is either a ``simple_stmt`` or a
``compound_stmt``.

For example, parsing ``x = 1\\n`` produces::

    ASTNode(rule_name="file", children=[
        ASTNode(rule_name="statement", children=[
            ASTNode(rule_name="simple_stmt", children=[
                ASTNode(rule_name="assign_stmt", children=[
                    ASTNode(rule_name="expression_list", children=[
                        ASTNode(rule_name="expression", children=[...])
                    ]),
                    ASTNode(rule_name="assign_op", children=[Token(EQUALS, "=")]),
                    ASTNode(rule_name="expression_list", children=[
                        ASTNode(rule_name="expression", children=[...])
                    ])
                ]),
                Token(NEWLINE, ...)
            ])
        ])
    ])

Test Organization
-----------------

1. **Simple assignments** — ``x = 1``, ``x = 1 + 2``
2. **Function calls** — ``print("hello")``, ``f(1, 2)``
3. **Arithmetic expressions** — ``1 + 2 * 3``
4. **Function definitions** — ``def add(x, y): return x + y``
5. **If/else** — conditional execution
6. **For loops** — iteration over collections
7. **BUILD-file style** — ``cc_library(name = "foo", ...)``
8. **Multiple statements** — programs with more than one statement
9. **Factory function** — ``create_starlark_parser()``
"""

from __future__ import annotations

from lang_parser import ASTNode
from lexer import Token, TokenType

from starlark_parser import create_starlark_parser, parse_starlark


# ============================================================================
# Helpers — make AST inspection more readable
# ============================================================================


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all descendant nodes with the given rule name.

    This is a depth-first search through the AST. It is useful for
    finding specific constructs (like all "assign_stmt" nodes) without
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
    """Test parsing of Starlark assignment statements.

    Assignments are the most common statement in BUILD files::

        name = "my_library"
        srcs = ["main.cc", "util.cc"]
        timeout = 300

    The parser must recognize the expression_list, assignment operator,
    and right-hand-side expression_list, building an AST that reflects
    the assignment structure through the ``assign_stmt`` rule.
    """

    def test_simple_assignment(self) -> None:
        """Parse ``x = 1`` — the simplest possible assignment.

        Expected AST structure:
            file -> statement -> simple_stmt -> assign_stmt
        """
        ast = parse_starlark("x = 1\n")
        assert ast.rule_name == "file"

        # Should have an assign_stmt somewhere in the tree
        assign_stmts = find_nodes(ast, "assign_stmt")
        assert len(assign_stmts) >= 1

        # The assignment should contain the name "x" and value "1"
        tokens = find_tokens(assign_stmts[0])
        names = [t for t in tokens if t.type == TokenType.NAME]
        assert any(t.value == "x" for t in names)

    def test_assignment_with_arithmetic(self) -> None:
        """Parse ``x = 1 + 2`` — assignment with a binary expression.

        The right-hand side ``1 + 2`` should be parsed as an expression
        containing two terms connected by a PLUS operator.
        """
        ast = parse_starlark("x = 1 + 2\n")
        assert ast.rule_name == "file"

        # Find arithmetic nodes. There will be multiple arith nodes in the
        # tree (one for "x" on the LHS and one for "1 + 2" on the RHS).
        # The one with the PLUS token is the RHS arith node.
        arith_nodes = find_nodes(ast, "arith")
        assert len(arith_nodes) >= 1

        # Find the arith node that contains a PLUS among its direct children
        plus_ariths = [
            node for node in arith_nodes
            if any(isinstance(c, Token) and c.value == "+" for c in node.children)
        ]
        assert len(plus_ariths) >= 1

    def test_assignment_with_string(self) -> None:
        """Parse ``name = "starlark"`` — assignment with a string value.

        String assignments are ubiquitous in BUILD files.
        """
        ast = parse_starlark('name = "starlark"\n')

        assign_stmts = find_nodes(ast, "assign_stmt")
        assert len(assign_stmts) >= 1

        tokens = find_tokens(assign_stmts[0])
        string_tokens = [t for t in tokens if t.type == TokenType.STRING]
        assert len(string_tokens) >= 1


# ============================================================================
# Test: Function Calls
# ============================================================================


class TestFunctionCalls:
    """Test parsing of Starlark function calls.

    Function calls are central to Starlark's use in BUILD files. Most
    BUILD rules are function calls::

        cc_library(
            name = "foo",
            srcs = ["foo.cc"],
        )

    The parser recognizes function calls as primary expressions with
    a call suffix: ``atom { suffix }`` where suffix includes
    ``LPAREN [ arguments ] RPAREN``.
    """

    def test_simple_function_call(self) -> None:
        """Parse ``print("hello")`` — a function call with one string argument."""
        ast = parse_starlark('print("hello")\n')
        assert ast.rule_name == "file"

        # Should have tokens for print, (, "hello", )
        all_tokens = find_tokens(ast)
        names = [t for t in all_tokens if t.type == TokenType.NAME]
        assert any(t.value == "print" for t in names)

        strings = [t for t in all_tokens if t.type == TokenType.STRING]
        assert len(strings) >= 1

    def test_function_call_multiple_args(self) -> None:
        """Parse ``add(1, 2)`` — a function call with two arguments."""
        ast = parse_starlark("add(1, 2)\n")

        all_tokens = find_tokens(ast)
        names = [t for t in all_tokens if t.type == TokenType.NAME]
        assert any(t.value == "add" for t in names)

        # Should have two integer tokens
        int_tokens = [t for t in all_tokens if t.type == "INT"]
        assert len(int_tokens) == 2


# ============================================================================
# Test: Arithmetic Expressions
# ============================================================================


class TestArithmeticExpressions:
    """Test parsing of arithmetic expressions with operator precedence.

    The Starlark grammar encodes precedence through rule nesting:
    - ``arith`` handles ``+`` and ``-``
    - ``term`` handles ``*``, ``/``, ``//``, ``%``
    - ``factor`` handles unary ``+``, ``-``, ``~``
    - ``power`` handles ``**``

    This means ``1 + 2 * 3`` parses as ``1 + (2 * 3)`` because ``*``
    is handled at the ``term`` level, which is deeper in the parse tree.
    """

    def test_simple_addition(self) -> None:
        """Parse ``1 + 2`` — simple addition."""
        ast = parse_starlark("1 + 2\n")

        arith_nodes = find_nodes(ast, "arith")
        assert len(arith_nodes) >= 1

        tokens = find_tokens(arith_nodes[0])
        values = [t.value for t in tokens]
        assert "1" in values
        assert "+" in values
        assert "2" in values

    def test_multiplication_before_addition(self) -> None:
        """Parse ``1 + 2 * 3`` — multiplication has higher precedence.

        In the AST, ``2 * 3`` should be grouped inside a ``term`` node,
        while ``1`` and ``(2 * 3)`` are children of the ``arith`` node.
        """
        ast = parse_starlark("1 + 2 * 3\n")

        # The top-level arith should have a PLUS
        arith_nodes = find_nodes(ast, "arith")
        assert len(arith_nodes) >= 1

        # There should be term nodes containing multiplication
        term_nodes = find_nodes(ast, "term")
        star_terms = [
            t for t in term_nodes
            if any(
                isinstance(c, Token) and c.value == "*"
                for c in t.children
            )
        ]
        assert len(star_terms) >= 1


# ============================================================================
# Test: Function Definitions
# ============================================================================


class TestFunctionDefinitions:
    """Test parsing of Starlark function definitions.

    Function definitions use indented blocks (suites)::

        def add(x, y):
            return x + y

    The parser must handle:
    - The ``def`` keyword
    - Function name (NAME)
    - Parameter list in parentheses
    - Colon
    - Indented suite (the function body)
    """

    def test_simple_function_def(self) -> None:
        """Parse a function that returns a value.

        Source::

            def add(x, y):
                return x + y
        """
        ast = parse_starlark("def add(x, y):\n    return x + y\n")
        assert ast.rule_name == "file"

        # Should have a def_stmt
        def_stmts = find_nodes(ast, "def_stmt")
        assert len(def_stmts) == 1

        # The def should contain the function name "add"
        tokens = find_tokens(def_stmts[0])
        names = [t for t in tokens if t.type == TokenType.NAME]
        assert any(t.value == "add" for t in names)

        # Should have a return_stmt inside
        return_stmts = find_nodes(def_stmts[0], "return_stmt")
        assert len(return_stmts) == 1


# ============================================================================
# Test: If/Else Statements
# ============================================================================


class TestIfElse:
    """Test parsing of Starlark if/else statements.

    Starlark's conditional syntax is identical to Python's::

        if condition:
            body
        elif other_condition:
            other_body
        else:
            fallback_body
    """

    def test_if_else(self) -> None:
        """Parse an if/else block.

        Source::

            if x:
                y = 1
            else:
                y = 2
        """
        ast = parse_starlark("if x:\n    y = 1\nelse:\n    y = 2\n")
        assert ast.rule_name == "file"

        # Should have an if_stmt
        if_stmts = find_nodes(ast, "if_stmt")
        assert len(if_stmts) == 1

        # The if_stmt should contain assign_stmts for both branches
        assign_stmts = find_nodes(if_stmts[0], "assign_stmt")
        assert len(assign_stmts) == 2


# ============================================================================
# Test: For Loops
# ============================================================================


class TestForLoops:
    """Test parsing of Starlark for loops.

    Starlark for-loops iterate over finite collections. Unlike Python,
    there is no ``while`` loop — this guarantees termination::

        for item in items:
            process(item)
    """

    def test_simple_for_loop(self) -> None:
        """Parse a for loop that iterates over a collection.

        Source::

            for x in items:
                print(x)
        """
        ast = parse_starlark("for x in items:\n    print(x)\n")
        assert ast.rule_name == "file"

        # Should have a for_stmt
        for_stmts = find_nodes(ast, "for_stmt")
        assert len(for_stmts) == 1

        # Should contain the loop variable "x" and collection "items"
        tokens = find_tokens(for_stmts[0])
        names = [t for t in tokens if t.type == TokenType.NAME]
        name_values = [t.value for t in names]
        assert "x" in name_values
        assert "items" in name_values


# ============================================================================
# Test: BUILD-File Style Function Calls
# ============================================================================


class TestBuildFileStyle:
    """Test parsing of BUILD-file style function calls.

    The primary use case for Starlark is BUILD files, which consist of
    function calls with keyword arguments spread across multiple lines::

        cc_library(
            name = "foo",
            srcs = ["foo.cc"],
            deps = ["//lib:bar"],
        )

    This tests the parser's ability to handle:
    - Multi-line function calls (bracket suppression in the lexer)
    - Keyword arguments (``name = value``)
    - List literals (``[...]``)
    """

    def test_build_rule_call(self) -> None:
        """Parse a BUILD-file style function call with keyword arguments.

        This is the quintessential Starlark use case: a build rule
        declaration with named arguments and list values.
        """
        source = 'cc_library(\n    name = "foo",\n    srcs = ["foo.cc"],\n)\n'
        ast = parse_starlark(source)
        assert ast.rule_name == "file"

        # Should contain the rule name "cc_library"
        all_tokens = find_tokens(ast)
        names = [t for t in all_tokens if t.type == TokenType.NAME]
        assert any(t.value == "cc_library" for t in names)

        # Should contain string literals for the name and source file
        strings = [t for t in all_tokens if t.type == TokenType.STRING]
        string_values = [t.value for t in strings]
        assert "foo" in string_values
        assert "foo.cc" in string_values


# ============================================================================
# Test: Multiple Statements
# ============================================================================


class TestMultipleStatements:
    """Test parsing of programs with multiple statements.

    The ``file`` rule matches ``{ NEWLINE | statement }``, meaning zero
    or more statements (possibly interspersed with blank lines). The
    parser must correctly separate statements by NEWLINE tokens and
    parse each one independently.
    """

    def test_two_assignments(self) -> None:
        """Parse two assignment statements separated by a newline."""
        ast = parse_starlark("x = 1\ny = 2\n")
        assert ast.rule_name == "file"

        assign_stmts = find_nodes(ast, "assign_stmt")
        assert len(assign_stmts) == 2

    def test_mixed_statements(self) -> None:
        """Parse a mix of assignments and expression statements."""
        ast = parse_starlark("x = 5\ny = x + 1\n")

        assign_stmts = find_nodes(ast, "assign_stmt")
        assert len(assign_stmts) == 2


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateStarlarkParser:
    """Test the ``create_starlark_parser()`` factory function.

    While ``parse_starlark()`` is the simpler interface,
    ``create_starlark_parser()`` gives access to the ``GrammarParser``
    object, which is useful for advanced inspection or integration
    with custom pipelines.
    """

    def test_creates_parser(self) -> None:
        """The factory should return a GrammarParser with a parse method."""
        parser = create_starlark_parser("x = 1\n")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result(self) -> None:
        """The factory should produce the same AST as parse_starlark()."""
        source = "x = 1 + 2\n"
        ast_direct = parse_starlark(source)
        ast_factory = create_starlark_parser(source).parse()

        # Both should have the same rule_name and same number of children
        assert ast_direct.rule_name == ast_factory.rule_name
        assert len(ast_direct.children) == len(ast_factory.children)

    def test_factory_with_function_call(self) -> None:
        """Verify the factory works with function calls."""
        parser = create_starlark_parser('print("world")\n')
        ast = parser.parse()
        assert ast.rule_name == "file"
