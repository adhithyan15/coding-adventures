"""Tests for the grammar-driven parser.

=============================================================================
TESTING STRATEGY
=============================================================================

These tests verify that the GrammarParser correctly interprets .grammar file
rules to build ASTs from token streams. Unlike the hand-written parser tests
(which construct tokens manually), these tests use the actual python.grammar
file to drive parsing — testing the full grammar-driven pipeline.

The tests are organized from simple to complex:

    1. Helper utilities (ast_to_dict)
    2. Single atoms (numbers, strings, names)
    3. Binary operations and precedence
    4. Parenthesized expressions
    5. Assignment statements
    6. Multiple statements
    7. Edge cases and error handling
    8. Tree walking / value extraction

Each test:
    1. Loads the actual python.grammar file
    2. Tokenizes source code using the Lexer
    3. Feeds tokens to GrammarParser
    4. Asserts the resulting generic AST has the expected structure
=============================================================================
"""

from __future__ import annotations

from pathlib import Path

import pytest
from grammar_tools import parse_parser_grammar
from lexer import Lexer, Token, TokenType

from lang_parser.grammar_parser import (
    ASTNode,
    GrammarParseError,
    GrammarParser,
    _compute_node_position,
    _find_first_token,
    _find_last_token,
    collect_tokens,
    find_nodes,
    is_ast_node,
    walk_ast,
)

# =============================================================================
# FIXTURES — Load the grammar once and reuse across tests
# =============================================================================

GRAMMARS_DIR = Path(__file__).parent.parent.parent.parent.parent / "grammars"


@pytest.fixture
def grammar():  # noqa: ANN201
    """Load the python.grammar file and parse it into a ParserGrammar."""
    grammar_path = GRAMMARS_DIR / "python.grammar"
    return parse_parser_grammar(grammar_path.read_text())


# =============================================================================
# HELPER — Convert AST tree to a readable dict for assertions
# =============================================================================


def _type_name(token: Token) -> str:
    """Extract the type name from a token (handles enum and string types)."""
    if isinstance(token.type, str):
        return token.type
    return token.type.name


def ast_to_dict(node: ASTNode | Token) -> dict | str:
    """Convert a generic ASTNode tree to a readable dict for easy assertion.

    This helper transforms the tree into a JSON-like structure that's easy
    to compare in assertions. Each ASTNode becomes a dict with "rule" and
    "children" keys. Tokens become strings of the form "TYPE:value".

    Examples::

        ast_to_dict(ASTNode("factor", [Token(NUMBER, "42", 1, 1)]))
        # Returns: {"rule": "factor", "children": ["NUMBER:42"]}

        ast_to_dict(Token(PLUS, "+", 1, 1))
        # Returns: "PLUS:+"

    Args:
        node: An ASTNode or Token to convert.

    Returns:
        A dict (for ASTNode) or string (for Token) representation.
    """
    if isinstance(node, Token):
        return f"{_type_name(node)}:{node.value}"
    return {
        "rule": node.rule_name,
        "children": [ast_to_dict(child) for child in node.children],
    }


def parse_source(source: str, grammar: object) -> ASTNode:
    """Tokenize source code and parse it with the grammar-driven parser.

    This is the main helper for tests — it handles the full pipeline from
    source text to AST.

    Args:
        source: The source code string to parse.
        grammar: A ParserGrammar (from parse_parser_grammar).

    Returns:
        The root ASTNode of the parse tree.
    """
    tokens = Lexer(source).tokenize()
    parser = GrammarParser(tokens, grammar)  # type: ignore[arg-type]
    return parser.parse()


# =============================================================================
# TEST: ast_to_dict HELPER
# =============================================================================


class TestAstToDict:
    """Verify the ast_to_dict helper works correctly."""

    def test_token_to_string(self) -> None:
        """A Token should become 'TYPE:value'."""
        token = Token(type=TokenType.NUMBER, value="42", line=1, column=1)
        assert ast_to_dict(token) == "NUMBER:42"

    def test_leaf_node(self) -> None:
        """An ASTNode with a single token child."""
        token = Token(type=TokenType.NUMBER, value="42", line=1, column=1)
        node = ASTNode(rule_name="factor", children=[token])
        result = ast_to_dict(node)

        assert result == {"rule": "factor", "children": ["NUMBER:42"]}

    def test_nested_node(self) -> None:
        """A nested ASTNode tree."""
        inner = ASTNode(
            rule_name="factor",
            children=[Token(type=TokenType.NUMBER, value="1", line=1, column=1)],
        )
        outer = ASTNode(rule_name="term", children=[inner])
        result = ast_to_dict(outer)

        assert result == {
            "rule": "term",
            "children": [{"rule": "factor", "children": ["NUMBER:1"]}],
        }


# =============================================================================
# TEST: SINGLE ATOMS
# =============================================================================
#
# The simplest possible inputs: a single value that forms a complete
# expression. These test that the grammar-driven parser can match
# basic token types through the grammar rules.
# =============================================================================


class TestSingleAtoms:
    """Tests for parsing single-token expressions."""

    def test_parse_number(self, grammar: object) -> None:
        """Parsing `42` should produce a program with a single number."""
        ast = parse_source("42", grammar)

        # The root should be a "program" node.
        assert ast.rule_name == "program"

        # Walk into the tree to find the NUMBER token.
        tree = ast_to_dict(ast)
        assert tree["rule"] == "program"

        # The program contains a statement -> expression_stmt -> expression
        # -> term -> factor -> NUMBER:42. Let's verify the number is there.
        # Flatten and check the leaf value exists somewhere in the tree.
        assert _find_token_in_tree(ast, "NUMBER", "42")

    def test_parse_string(self, grammar: object) -> None:
        """Parsing `"hello"` should produce a tree with a STRING token."""
        ast = parse_source('"hello"', grammar)
        assert ast.rule_name == "program"
        assert _find_token_in_tree(ast, "STRING", "hello")

    def test_parse_name(self, grammar: object) -> None:
        """Parsing `x` should produce a tree with a NAME token."""
        ast = parse_source("x", grammar)
        assert ast.rule_name == "program"
        assert _find_token_in_tree(ast, "NAME", "x")


# =============================================================================
# TEST: BINARY OPERATIONS
# =============================================================================


class TestBinaryOperations:
    """Tests for binary operations parsed via grammar rules."""

    def test_addition(self, grammar: object) -> None:
        """`1 + 2` should produce a tree with both numbers and the operator."""
        ast = parse_source("1 + 2", grammar)
        assert ast.rule_name == "program"

        # The tree should contain NUMBER:1, PLUS:+, NUMBER:2
        assert _find_token_in_tree(ast, "NUMBER", "1")
        assert _find_token_in_tree(ast, "PLUS", "+")
        assert _find_token_in_tree(ast, "NUMBER", "2")

    def test_subtraction(self, grammar: object) -> None:
        """`5 - 3` should parse correctly."""
        ast = parse_source("5 - 3", grammar)
        assert _find_token_in_tree(ast, "NUMBER", "5")
        assert _find_token_in_tree(ast, "MINUS", "-")
        assert _find_token_in_tree(ast, "NUMBER", "3")

    def test_multiplication(self, grammar: object) -> None:
        """`4 * 5` should parse correctly."""
        ast = parse_source("4 * 5", grammar)
        assert _find_token_in_tree(ast, "NUMBER", "4")
        assert _find_token_in_tree(ast, "STAR", "*")
        assert _find_token_in_tree(ast, "NUMBER", "5")

    def test_division(self, grammar: object) -> None:
        """`10 / 2` should parse correctly."""
        ast = parse_source("10 / 2", grammar)
        assert _find_token_in_tree(ast, "NUMBER", "10")
        assert _find_token_in_tree(ast, "SLASH", "/")
        assert _find_token_in_tree(ast, "NUMBER", "2")


# =============================================================================
# TEST: OPERATOR PRECEDENCE
# =============================================================================
#
# The grammar encodes precedence through rule nesting:
#   expression = term { (PLUS | MINUS) term }     — lowest precedence
#   term       = factor { (STAR | SLASH) factor }  — higher precedence
#   factor     = NUMBER | STRING | NAME | ...      — highest precedence
#
# This means multiplication/division are parsed INSIDE a term, which is
# then used as an operand of addition/subtraction. The tree structure
# naturally reflects this: * and / end up deeper than + and -.
# =============================================================================


class TestPrecedence:
    """Tests for operator precedence in the grammar-driven parser."""

    def test_mul_binds_tighter_than_add(self, grammar: object) -> None:
        """`1 + 2 * 3` should group multiplication tighter.

        Expected tree structure:
            expression
            ├── term (containing just "1")
            ├── PLUS
            └── term
                ├── factor (containing "2")
                ├── STAR
                └── factor (containing "3")

        The key insight: "2 * 3" is inside a single "term" node, while
        "1" is in a separate "term" node. This means * binds tighter.
        """
        ast = parse_source("1 + 2 * 3", grammar)

        # Navigate: program -> statement -> expression_stmt -> expression
        expression = _find_rule(ast, "expression")
        assert expression is not None

        # The expression should have children:
        # [term("1"), PLUS, term("2 * 3")]
        # The first term contains just "1", the second term contains "2 * 3".
        terms = [
            c for c in expression.children
            if isinstance(c, ASTNode) and c.rule_name == "term"
        ]
        assert len(terms) == 2

        # First term should contain just the number 1
        assert _find_token_in_tree(terms[0], "NUMBER", "1")
        assert not _find_token_in_tree(terms[0], "STAR", "*")

        # Second term should contain 2 * 3
        assert _find_token_in_tree(terms[1], "NUMBER", "2")
        assert _find_token_in_tree(terms[1], "STAR", "*")
        assert _find_token_in_tree(terms[1], "NUMBER", "3")

    def test_div_binds_tighter_than_sub(self, grammar: object) -> None:
        """`10 - 6 / 2` should group division tighter."""
        ast = parse_source("10 - 6 / 2", grammar)
        expression = _find_rule(ast, "expression")
        assert expression is not None

        terms = [
            c for c in expression.children
            if isinstance(c, ASTNode) and c.rule_name == "term"
        ]
        assert len(terms) == 2

        # First term: just 10
        assert _find_token_in_tree(terms[0], "NUMBER", "10")
        # Second term: 6 / 2
        assert _find_token_in_tree(terms[1], "NUMBER", "6")
        assert _find_token_in_tree(terms[1], "SLASH", "/")
        assert _find_token_in_tree(terms[1], "NUMBER", "2")


# =============================================================================
# TEST: PARENTHESIZED EXPRESSIONS
# =============================================================================


class TestParentheses:
    """Tests for parenthesized expressions in the grammar-driven parser."""

    def test_parens_override_precedence(self, grammar: object) -> None:
        """`(1 + 2) * 3` — parens should force addition first.

        Without parentheses, ``1 + 2 * 3`` groups as ``1 + (2 * 3)``.
        With parentheses, ``(1 + 2)`` becomes a single factor that's
        then multiplied by 3. In the tree, the addition should appear
        INSIDE a factor node (deeper than the multiplication).
        """
        ast = parse_source("(1 + 2) * 3", grammar)

        # The top-level expression should be a single term containing *.
        # That term's first factor contains the parenthesized (1 + 2).
        expression = _find_rule(ast, "expression")
        assert expression is not None

        # There should be a STAR somewhere in the expression's direct term
        term = _find_rule(expression, "term")
        assert term is not None
        assert _find_token_in_tree(term, "STAR", "*")

        # The LPAREN/RPAREN should appear in the factor
        assert _find_token_in_tree(ast, "LPAREN", "(")
        assert _find_token_in_tree(ast, "RPAREN", ")")

    def test_nested_parentheses(self, grammar: object) -> None:
        """`((42))` should parse without error."""
        ast = parse_source("((42))", grammar)
        assert _find_token_in_tree(ast, "NUMBER", "42")


# =============================================================================
# TEST: ASSIGNMENT STATEMENTS
# =============================================================================


class TestAssignment:
    """Tests for assignment statements via the grammar-driven parser."""

    def test_simple_assignment(self, grammar: object) -> None:
        """`x = 42` should produce an assignment node."""
        ast = parse_source("x = 42\n", grammar)
        assert ast.rule_name == "program"

        # Should contain an assignment rule node
        assignment = _find_rule(ast, "assignment")
        assert assignment is not None
        assert _find_token_in_tree(assignment, "NAME", "x")
        assert _find_token_in_tree(assignment, "EQUALS", "=")
        assert _find_token_in_tree(assignment, "NUMBER", "42")

    def test_assignment_with_expression(self, grammar: object) -> None:
        """`x = 1 + 2` should have a binary expression on the right side."""
        ast = parse_source("x = 1 + 2\n", grammar)
        assignment = _find_rule(ast, "assignment")
        assert assignment is not None
        assert _find_token_in_tree(assignment, "NAME", "x")
        assert _find_token_in_tree(assignment, "PLUS", "+")
        assert _find_token_in_tree(assignment, "NUMBER", "1")
        assert _find_token_in_tree(assignment, "NUMBER", "2")

    def test_assignment_preserves_precedence(self, grammar: object) -> None:
        """`result = 1 + 2 * 3` — precedence in the value expression."""
        ast = parse_source("result = 1 + 2 * 3\n", grammar)
        assignment = _find_rule(ast, "assignment")
        assert assignment is not None

        # Find the expression inside the assignment
        expression = _find_rule(assignment, "expression")
        assert expression is not None
        terms = [
            c for c in expression.children
            if isinstance(c, ASTNode) and c.rule_name == "term"
        ]
        assert len(terms) == 2


# =============================================================================
# TEST: MULTIPLE STATEMENTS
# =============================================================================


class TestMultipleStatements:
    """Tests for programs with multiple statements."""

    def test_two_assignments(self, grammar: object) -> None:
        """`x = 1\\ny = 2\\n` should produce two statement nodes."""
        ast = parse_source("x = 1\ny = 2\n", grammar)
        assert ast.rule_name == "program"

        # Count the statement children
        statements = [
            c for c in ast.children
            if isinstance(c, ASTNode) and c.rule_name == "statement"
        ]
        assert len(statements) == 2

    def test_assignment_then_expression(self, grammar: object) -> None:
        """`x = 1\\nx + 2\\n` — assignment then expression statement."""
        ast = parse_source("x = 1\nx + 2\n", grammar)
        statements = [
            c for c in ast.children
            if isinstance(c, ASTNode) and c.rule_name == "statement"
        ]
        assert len(statements) == 2

        # First statement should be an assignment
        assert _find_rule(statements[0], "assignment") is not None
        # Second should be an expression_stmt
        assert _find_rule(statements[1], "expression_stmt") is not None

    def test_three_statements(self, grammar: object) -> None:
        """Three-statement program."""
        source = "a = 10\nb = 20\na + b\n"
        ast = parse_source(source, grammar)
        statements = [
            c for c in ast.children
            if isinstance(c, ASTNode) and c.rule_name == "statement"
        ]
        assert len(statements) == 3


# =============================================================================
# TEST: EMPTY PROGRAM
# =============================================================================


class TestEmptyProgram:
    """Tests for edge cases with empty input."""

    def test_empty_program(self, grammar: object) -> None:
        """An empty program should produce a program node with no statements."""
        ast = parse_source("", grammar)
        assert ast.rule_name == "program"
        # The program's children should be empty (no statements).
        statements = [
            c for c in ast.children
            if isinstance(c, ASTNode) and c.rule_name == "statement"
        ]
        assert len(statements) == 0

    def test_only_newlines(self, grammar: object) -> None:
        """A program with only newlines should be empty."""
        ast = parse_source("\n\n\n", grammar)
        assert ast.rule_name == "program"
        statements = [
            c for c in ast.children
            if isinstance(c, ASTNode) and c.rule_name == "statement"
        ]
        assert len(statements) == 0


# =============================================================================
# TEST: ERROR HANDLING
# =============================================================================


class TestErrors:
    """Tests for grammar-driven parser error handling."""

    def test_unexpected_token(self, grammar: object) -> None:
        """A stray operator should raise GrammarParseError."""
        with pytest.raises(GrammarParseError, match="Unexpected token"):
            parse_source(")", grammar)

    def test_error_includes_position(self, grammar: object) -> None:
        """GrammarParseError should include the problematic token."""
        with pytest.raises(GrammarParseError) as exc_info:
            parse_source(")", grammar)

        error = exc_info.value
        assert error.token is not None

    def test_empty_grammar_raises(self) -> None:
        """A grammar with no rules should raise an error."""
        from grammar_tools import ParserGrammar

        empty_grammar = ParserGrammar(rules=[])
        tokens = [Token(type=TokenType.EOF, value="", line=1, column=1)]
        parser = GrammarParser(tokens, empty_grammar)

        with pytest.raises(GrammarParseError, match="no rules"):
            parser.parse()

    def test_undefined_rule_raises(self, grammar: object) -> None:
        """Referencing an undefined rule should raise an error."""
        from grammar_tools import GrammarRule, ParserGrammar, RuleReference

        bad_grammar = ParserGrammar(
            rules=[
                GrammarRule(
                    name="start",
                    body=RuleReference(name="nonexistent", is_token=False),
                    line_number=1,
                )
            ]
        )
        tokens = [Token(type=TokenType.NUMBER, value="42", line=1, column=1)]
        parser = GrammarParser(tokens, bad_grammar)

        with pytest.raises(GrammarParseError):
            parser.parse()


# =============================================================================
# TEST: TREE WALKING / VALUE EXTRACTION
# =============================================================================
#
# The grammar-driven parser produces generic ASTNode trees. These tests
# verify that the trees can be walked to extract meaningful values — which
# is what a bytecode compiler or interpreter would need to do.
# =============================================================================


class TestTreeWalking:
    """Tests for extracting values from the generic AST."""

    def test_extract_number_value(self, grammar: object) -> None:
        """Walk the tree to extract the numeric value from `42`."""
        ast = parse_source("42", grammar)
        numbers = _collect_tokens(ast, "NUMBER")
        assert len(numbers) == 1
        assert numbers[0].value == "42"

    def test_extract_all_tokens_from_expression(self, grammar: object) -> None:
        """Walk `1 + 2 * 3` to find all tokens in order."""
        ast = parse_source("1 + 2 * 3", grammar)
        all_tokens = _collect_all_tokens(ast)

        # Filter out NEWLINE and EOF
        significant = [
            t for t in all_tokens
            if t.type not in (TokenType.NEWLINE, TokenType.EOF)
        ]
        values = [t.value for t in significant]
        assert values == ["1", "+", "2", "*", "3"]

    def test_extract_assignment_parts(self, grammar: object) -> None:
        """Walk `x = 1 + 2` to extract the name and value tokens."""
        ast = parse_source("x = 1 + 2\n", grammar)

        assignment = _find_rule(ast, "assignment")
        assert assignment is not None

        names = _collect_tokens(assignment, "NAME")
        assert len(names) == 1
        assert names[0].value == "x"

        numbers = _collect_tokens(assignment, "NUMBER")
        assert len(numbers) == 2
        assert [n.value for n in numbers] == ["1", "2"]

    def test_ast_node_is_leaf(self) -> None:
        """ASTNode.is_leaf should correctly identify leaf nodes."""
        token = Token(type=TokenType.NUMBER, value="42", line=1, column=1)
        leaf = ASTNode(rule_name="factor", children=[token])
        assert leaf.is_leaf is True
        assert leaf.token == token

        non_leaf = ASTNode(rule_name="term", children=[leaf])
        assert non_leaf.is_leaf is False
        assert non_leaf.token is None

    def test_ast_node_token_property(self) -> None:
        """ASTNode.token should return None for non-leaf nodes."""
        inner = ASTNode(
            rule_name="factor",
            children=[Token(type=TokenType.NUMBER, value="1", line=1, column=1)],
        )
        outer = ASTNode(rule_name="term", children=[inner, inner])
        assert outer.token is None

    def test_ast_to_dict_full_tree(self, grammar: object) -> None:
        """ast_to_dict should produce a complete dict representation."""
        ast = parse_source("42", grammar)
        d = ast_to_dict(ast)

        # Should be a dict with "rule" and "children" keys
        assert isinstance(d, dict)
        assert d["rule"] == "program"
        assert isinstance(d["children"], list)


# =============================================================================
# TEST: GrammarParseError CLASS
# =============================================================================


class TestGrammarParseError:
    """Tests for the GrammarParseError exception class."""

    def test_error_with_token(self) -> None:
        """GrammarParseError with a token should include position info."""
        token = Token(type=TokenType.PLUS, value="+", line=3, column=7)
        error = GrammarParseError("bad syntax", token)

        assert error.token == token
        assert "3:7" in str(error)
        assert "bad syntax" in str(error)

    def test_error_without_token(self) -> None:
        """GrammarParseError without a token should still work."""
        error = GrammarParseError("no rules")

        assert error.token is None
        assert "no rules" in str(error)
        assert "Parse error" in str(error)


# =============================================================================
# TEST: PACKRAT MEMOIZATION
# =============================================================================


class TestPackratMemoization:
    """Tests for packrat memoization correctness."""

    def test_memoization_produces_same_result(self, grammar: object) -> None:
        """Parsing the same input twice should produce identical results.

        This verifies that memoization doesn't corrupt results.
        """
        ast1 = parse_source("1 + 2 * 3", grammar)
        ast2 = parse_source("1 + 2 * 3", grammar)
        assert ast_to_dict(ast1) == ast_to_dict(ast2)

    def test_memoization_with_backtracking(self, grammar: object) -> None:
        """Backtracking should work correctly with memoization.

        When the parser tries an alternative that fails, it backtracks.
        Memoization caches the failure so it doesn't retry.
        """
        # This expression requires the parser to try assignment first,
        # fail, then try expression_stmt.
        ast = parse_source("1 + 2", grammar)
        assert _find_token_in_tree(ast, "NUMBER", "1")
        assert _find_token_in_tree(ast, "PLUS", "+")


# =============================================================================
# TEST: STRING-BASED TOKEN TYPES
# =============================================================================


class TestStringTokenTypes:
    """Tests for parsing with string-based token types.

    When the grammar-driven lexer emits tokens with string types (for
    extended grammars like Starlark), the parser must handle them correctly.
    """

    def test_string_type_token_matching(self) -> None:
        """Tokens with string types should match grammar references."""
        from grammar_tools import GrammarRule, ParserGrammar, RuleReference, Sequence

        # Create a grammar that references "INT" (a string type, not in TokenType)
        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="expr",
                body=Sequence(elements=[
                    RuleReference(name="INT", is_token=True),
                ]),
                line_number=1,
            ),
        ])

        # Create a token with string type "INT"
        tokens = [
            Token(type="INT", value="42", line=1, column=1),
            Token(type="EOF", value="", line=1, column=3),
        ]
        parser = GrammarParser(tokens, grammar)
        ast = parser.parse()
        assert ast.rule_name == "expr"
        assert len(ast.children) == 1
        assert ast.children[0].value == "42"  # type: ignore[union-attr]

    def test_mixed_enum_and_string_types(self) -> None:
        """Parser should handle a mix of enum and string token types."""
        from grammar_tools import (
            GrammarRule,
            ParserGrammar,
            RuleReference,
            Sequence,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="expr",
                body=Sequence(elements=[
                    RuleReference(name="NAME", is_token=True),
                    RuleReference(name="CUSTOM_OP", is_token=True),
                    RuleReference(name="NUMBER", is_token=True),
                ]),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NAME, value="x", line=1, column=1),
            Token(type="CUSTOM_OP", value="<>", line=1, column=3),
            Token(type=TokenType.NUMBER, value="1", line=1, column=6),
            Token(type=TokenType.EOF, value="", line=1, column=7),
        ]
        parser = GrammarParser(tokens, grammar)
        ast = parser.parse()
        assert len(ast.children) == 3


# =============================================================================
# TEST: SIGNIFICANT NEWLINES
# =============================================================================


class TestSignificantNewlines:
    """Tests for grammars where NEWLINE tokens are significant."""

    def test_grammar_with_newlines_detected(self) -> None:
        """When a grammar references NEWLINE, the parser detects it."""
        from grammar_tools import (
            GrammarRule,
            ParserGrammar,
            Repetition,
            RuleReference,
            Sequence,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="file",
                body=Repetition(element=Sequence(elements=[
                    RuleReference(name="NAME", is_token=True),
                    RuleReference(name="NEWLINE", is_token=True),
                ])),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NAME, value="x", line=1, column=1),
            Token(type=TokenType.NEWLINE, value="\\n", line=1, column=2),
            Token(type=TokenType.EOF, value="", line=2, column=1),
        ]
        parser = GrammarParser(tokens, grammar)
        # Parser should detect that NEWLINE is referenced
        assert parser._newlines_significant is True

        ast = parser.parse()
        assert ast.rule_name == "file"

    def test_grammar_without_newlines_insignificant(self) -> None:
        """When no rule references NEWLINE, newlines are insignificant."""
        from grammar_tools import GrammarRule, ParserGrammar, RuleReference

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="expr",
                body=RuleReference(name="NUMBER", is_token=True),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NEWLINE, value="\\n", line=1, column=1),
            Token(type=TokenType.NUMBER, value="42", line=2, column=1),
            Token(type=TokenType.EOF, value="", line=2, column=3),
        ]
        parser = GrammarParser(tokens, grammar)
        assert parser._newlines_significant is False

        # Should skip the leading NEWLINE and parse the number
        ast = parser.parse()
        assert _find_token_in_tree(ast, "NUMBER", "42")


# =============================================================================
# TEST: STARLARK FULL PIPELINE
# =============================================================================


class TestStarlarkPipeline:
    """Full pipeline test: starlark.tokens + starlark.grammar → parse.

    This is the ultimate integration test for the grammar-driven stack.
    """

    @pytest.fixture()
    def starlark_grammar(self):  # noqa: ANN201
        """Load the starlark.grammar file."""
        grammar_path = GRAMMARS_DIR / "starlark.grammar"
        if not grammar_path.exists():
            pytest.skip("starlark.grammar not found")
        return parse_parser_grammar(grammar_path.read_text())

    @pytest.fixture()
    def starlark_tokens(self):  # noqa: ANN201
        """Load the starlark.tokens file."""
        from grammar_tools import parse_token_grammar
        tokens_path = GRAMMARS_DIR / "starlark.tokens"
        if not tokens_path.exists():
            pytest.skip("starlark.tokens not found")
        return parse_token_grammar(tokens_path.read_text())

    def test_simple_assignment(
        self,
        starlark_grammar: object,
        starlark_tokens: object,
    ) -> None:
        """Parse a simple Starlark assignment: x = 1"""
        from lexer.grammar_lexer import GrammarLexer

        tokens = GrammarLexer(
            "x = 1\n", starlark_tokens,  # type: ignore[arg-type]
        ).tokenize()
        parser = GrammarParser(
            tokens, starlark_grammar,  # type: ignore[arg-type]
        )
        ast = parser.parse()
        assert ast.rule_name == "file"

    def test_function_definition(
        self,
        starlark_grammar: object,
        starlark_tokens: object,
    ) -> None:
        """Parse a Starlark function definition."""
        from lexer.grammar_lexer import GrammarLexer

        source = "def add(x, y):\n    return x + y\n"
        tokens = GrammarLexer(
            source, starlark_tokens,  # type: ignore[arg-type]
        ).tokenize()
        parser = GrammarParser(
            tokens, starlark_grammar,  # type: ignore[arg-type]
        )
        ast = parser.parse()
        assert ast.rule_name == "file"
        # Should contain 'def' and 'return' keywords
        all_tokens = _collect_all_tokens(ast)
        values = [t.value for t in all_tokens]
        assert "def" in values
        assert "return" in values

    def test_if_else(
        self,
        starlark_grammar: object,
        starlark_tokens: object,
    ) -> None:
        """Parse a Starlark if/else statement."""
        from lexer.grammar_lexer import GrammarLexer

        source = "if x:\n    y = 1\nelse:\n    y = 2\n"
        tokens = GrammarLexer(
            source, starlark_tokens,  # type: ignore[arg-type]
        ).tokenize()
        parser = GrammarParser(
            tokens, starlark_grammar,  # type: ignore[arg-type]
        )
        ast = parser.parse()
        assert ast.rule_name == "file"

    def test_list_literal(
        self,
        starlark_grammar: object,
        starlark_tokens: object,
    ) -> None:
        """Parse a Starlark list literal."""
        from lexer.grammar_lexer import GrammarLexer

        source = 'x = [1, 2, 3]\n'
        tokens = GrammarLexer(
            source, starlark_tokens,  # type: ignore[arg-type]
        ).tokenize()
        parser = GrammarParser(
            tokens, starlark_grammar,  # type: ignore[arg-type]
        )
        ast = parser.parse()
        assert ast.rule_name == "file"

    def test_for_loop(
        self,
        starlark_grammar: object,
        starlark_tokens: object,
    ) -> None:
        """Parse a Starlark for loop."""
        from lexer.grammar_lexer import GrammarLexer

        source = "for x in items:\n    pass\n"
        tokens = GrammarLexer(
            source, starlark_tokens,  # type: ignore[arg-type]
        ).tokenize()
        parser = GrammarParser(
            tokens, starlark_grammar,  # type: ignore[arg-type]
        )
        ast = parser.parse()
        assert ast.rule_name == "file"


# =============================================================================
# TEST: POSITIVE LOOKAHEAD (&element)
# =============================================================================


class TestPositiveLookahead:
    """Tests for positive lookahead (&element) — succeed if element matches,
    consume nothing."""

    def test_positive_lookahead_succeeds(self) -> None:
        """&NUMBER should succeed when the next token is a NUMBER, consuming nothing."""
        from grammar_tools import (
            GrammarRule,
            ParserGrammar,
            PositiveLookahead,
            RuleReference,
            Sequence,
        )

        # Grammar: start = &NUMBER NUMBER ;
        # The lookahead checks for NUMBER without consuming, then NUMBER consumes it.
        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=Sequence(elements=[
                    PositiveLookahead(element=RuleReference(name="NUMBER", is_token=True)),
                    RuleReference(name="NUMBER", is_token=True),
                ]),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NUMBER, value="42", line=1, column=1),
            Token(type=TokenType.EOF, value="", line=1, column=3),
        ]
        parser = GrammarParser(tokens, grammar)
        ast = parser.parse()
        assert ast.rule_name == "start"
        # The lookahead produces no children; only the actual NUMBER match does.
        assert len(ast.children) == 1
        assert ast.children[0].value == "42"  # type: ignore[union-attr]

    def test_positive_lookahead_fails(self) -> None:
        """&NUMBER should fail when the next token is NAME, causing parse failure."""
        from grammar_tools import (
            GrammarRule,
            ParserGrammar,
            PositiveLookahead,
            RuleReference,
            Sequence,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=Sequence(elements=[
                    PositiveLookahead(element=RuleReference(name="NUMBER", is_token=True)),
                    RuleReference(name="NAME", is_token=True),
                ]),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NAME, value="x", line=1, column=1),
            Token(type=TokenType.EOF, value="", line=1, column=2),
        ]
        parser = GrammarParser(tokens, grammar)
        with pytest.raises(GrammarParseError):
            parser.parse()

    def test_positive_lookahead_does_not_consume(self) -> None:
        """&NAME NAME should parse the NAME once (lookahead does not consume)."""
        from grammar_tools import (
            GrammarRule,
            ParserGrammar,
            PositiveLookahead,
            RuleReference,
            Sequence,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=Sequence(elements=[
                    PositiveLookahead(element=RuleReference(name="NAME", is_token=True)),
                    RuleReference(name="NAME", is_token=True),
                ]),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NAME, value="hello", line=1, column=1),
            Token(type=TokenType.EOF, value="", line=1, column=6),
        ]
        parser = GrammarParser(tokens, grammar)
        ast = parser.parse()
        assert len(ast.children) == 1
        assert ast.children[0].value == "hello"  # type: ignore[union-attr]


# =============================================================================
# TEST: NEGATIVE LOOKAHEAD (!element)
# =============================================================================


class TestNegativeLookahead:
    """Tests for negative lookahead (!element) — succeed if element does NOT
    match, consume nothing."""

    def test_negative_lookahead_succeeds(self) -> None:
        """!PLUS NAME should succeed when the next token is NAME (not PLUS)."""
        from grammar_tools import (
            GrammarRule,
            NegativeLookahead,
            ParserGrammar,
            RuleReference,
            Sequence,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=Sequence(elements=[
                    NegativeLookahead(element=RuleReference(name="PLUS", is_token=True)),
                    RuleReference(name="NAME", is_token=True),
                ]),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NAME, value="x", line=1, column=1),
            Token(type=TokenType.EOF, value="", line=1, column=2),
        ]
        parser = GrammarParser(tokens, grammar)
        ast = parser.parse()
        assert ast.rule_name == "start"
        assert len(ast.children) == 1
        assert ast.children[0].value == "x"  # type: ignore[union-attr]

    def test_negative_lookahead_fails(self) -> None:
        """!NUMBER should fail when the next token IS a NUMBER."""
        from grammar_tools import (
            GrammarRule,
            NegativeLookahead,
            ParserGrammar,
            RuleReference,
            Sequence,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=Sequence(elements=[
                    NegativeLookahead(element=RuleReference(name="NUMBER", is_token=True)),
                    RuleReference(name="NUMBER", is_token=True),
                ]),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NUMBER, value="42", line=1, column=1),
            Token(type=TokenType.EOF, value="", line=1, column=3),
        ]
        parser = GrammarParser(tokens, grammar)
        with pytest.raises(GrammarParseError):
            parser.parse()

    def test_negative_lookahead_does_not_consume(self) -> None:
        """!PLUS NUMBER should parse the NUMBER (lookahead does not consume)."""
        from grammar_tools import (
            GrammarRule,
            NegativeLookahead,
            ParserGrammar,
            RuleReference,
            Sequence,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=Sequence(elements=[
                    NegativeLookahead(element=RuleReference(name="PLUS", is_token=True)),
                    RuleReference(name="NUMBER", is_token=True),
                ]),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NUMBER, value="7", line=1, column=1),
            Token(type=TokenType.EOF, value="", line=1, column=2),
        ]
        parser = GrammarParser(tokens, grammar)
        ast = parser.parse()
        assert len(ast.children) == 1
        assert ast.children[0].value == "7"  # type: ignore[union-attr]


# =============================================================================
# TEST: ONE-OR-MORE REPETITION ({element}+)
# =============================================================================


class TestOneOrMoreRepetition:
    """Tests for one-or-more repetition ({element}+) — must match at least once."""

    def test_one_or_more_single_match(self) -> None:
        """A single NUMBER should satisfy {NUMBER}+."""
        from grammar_tools import (
            GrammarRule,
            OneOrMoreRepetition,
            ParserGrammar,
            RuleReference,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=OneOrMoreRepetition(
                    element=RuleReference(name="NUMBER", is_token=True),
                ),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NUMBER, value="1", line=1, column=1),
            Token(type=TokenType.EOF, value="", line=1, column=2),
        ]
        parser = GrammarParser(tokens, grammar)
        ast = parser.parse()
        assert ast.rule_name == "start"
        assert len(ast.children) == 1
        assert ast.children[0].value == "1"  # type: ignore[union-attr]

    def test_one_or_more_multiple_matches(self) -> None:
        """Multiple NUMBERs should all be consumed by {NUMBER}+."""
        from grammar_tools import (
            GrammarRule,
            OneOrMoreRepetition,
            ParserGrammar,
            RuleReference,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=OneOrMoreRepetition(
                    element=RuleReference(name="NUMBER", is_token=True),
                ),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NUMBER, value="1", line=1, column=1),
            Token(type=TokenType.NUMBER, value="2", line=1, column=3),
            Token(type=TokenType.NUMBER, value="3", line=1, column=5),
            Token(type=TokenType.EOF, value="", line=1, column=6),
        ]
        parser = GrammarParser(tokens, grammar)
        ast = parser.parse()
        assert len(ast.children) == 3
        assert [c.value for c in ast.children] == ["1", "2", "3"]  # type: ignore[union-attr]

    def test_one_or_more_zero_matches_fails(self) -> None:
        """Zero matches should fail for {NUMBER}+ (requires at least one)."""
        from grammar_tools import (
            GrammarRule,
            OneOrMoreRepetition,
            ParserGrammar,
            RuleReference,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=OneOrMoreRepetition(
                    element=RuleReference(name="NUMBER", is_token=True),
                ),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NAME, value="x", line=1, column=1),
            Token(type=TokenType.EOF, value="", line=1, column=2),
        ]
        parser = GrammarParser(tokens, grammar)
        with pytest.raises(GrammarParseError):
            parser.parse()


# =============================================================================
# TEST: SEPARATED REPETITION ({element // separator})
# =============================================================================


class TestSeparatedRepetition:
    """Tests for separated repetition ({element // separator})."""

    def test_separated_single_element(self) -> None:
        """A single NUMBER with no separator should succeed."""
        from grammar_tools import (
            GrammarRule,
            Literal,
            ParserGrammar,
            RuleReference,
            SeparatedRepetition,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=SeparatedRepetition(
                    element=RuleReference(name="NUMBER", is_token=True),
                    separator=Literal(value=","),
                    at_least_one=False,
                ),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NUMBER, value="1", line=1, column=1),
            Token(type=TokenType.EOF, value="", line=1, column=2),
        ]
        parser = GrammarParser(tokens, grammar)
        ast = parser.parse()
        assert ast.rule_name == "start"
        assert len(ast.children) == 1

    def test_separated_multiple_elements(self) -> None:
        """NUMBER COMMA NUMBER COMMA NUMBER should parse as comma-separated list."""
        from grammar_tools import (
            GrammarRule,
            Literal,
            ParserGrammar,
            RuleReference,
            SeparatedRepetition,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=SeparatedRepetition(
                    element=RuleReference(name="NUMBER", is_token=True),
                    separator=Literal(value=","),
                    at_least_one=False,
                ),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NUMBER, value="1", line=1, column=1),
            Token(type=TokenType.COMMA, value=",", line=1, column=2),
            Token(type=TokenType.NUMBER, value="2", line=1, column=3),
            Token(type=TokenType.COMMA, value=",", line=1, column=4),
            Token(type=TokenType.NUMBER, value="3", line=1, column=5),
            Token(type=TokenType.EOF, value="", line=1, column=6),
        ]
        parser = GrammarParser(tokens, grammar)
        ast = parser.parse()
        # children: NUMBER, COMMA, NUMBER, COMMA, NUMBER
        assert len(ast.children) == 5
        values = [c.value for c in ast.children]  # type: ignore[union-attr]
        assert values == ["1", ",", "2", ",", "3"]

    def test_separated_zero_elements_allowed(self) -> None:
        """Zero elements should succeed when at_least_one=False."""
        from grammar_tools import (
            GrammarRule,
            Literal,
            ParserGrammar,
            RuleReference,
            Sequence,
            SeparatedRepetition,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=Sequence(elements=[
                    SeparatedRepetition(
                        element=RuleReference(name="NUMBER", is_token=True),
                        separator=Literal(value=","),
                        at_least_one=False,
                    ),
                    RuleReference(name="NAME", is_token=True),
                ]),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NAME, value="end", line=1, column=1),
            Token(type=TokenType.EOF, value="", line=1, column=4),
        ]
        parser = GrammarParser(tokens, grammar)
        ast = parser.parse()
        assert ast.rule_name == "start"

    def test_separated_at_least_one_fails_on_zero(self) -> None:
        """Zero elements should fail when at_least_one=True."""
        from grammar_tools import (
            GrammarRule,
            Literal,
            ParserGrammar,
            RuleReference,
            SeparatedRepetition,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=SeparatedRepetition(
                    element=RuleReference(name="NUMBER", is_token=True),
                    separator=Literal(value=","),
                    at_least_one=True,
                ),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NAME, value="x", line=1, column=1),
            Token(type=TokenType.EOF, value="", line=1, column=2),
        ]
        parser = GrammarParser(tokens, grammar)
        with pytest.raises(GrammarParseError):
            parser.parse()

    def test_separated_trailing_separator_not_consumed(self) -> None:
        """A trailing separator without a following element should not be consumed."""
        from grammar_tools import (
            GrammarRule,
            Literal,
            ParserGrammar,
            RuleReference,
            Sequence,
            SeparatedRepetition,
        )

        # Grammar: start = { NUMBER // "," } NAME ;
        # Input: 1 , x  — the "," should NOT be consumed because NAME follows, not NUMBER.
        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=Sequence(elements=[
                    SeparatedRepetition(
                        element=RuleReference(name="NUMBER", is_token=True),
                        separator=Literal(value=","),
                        at_least_one=False,
                    ),
                    Literal(value=","),
                    RuleReference(name="NAME", is_token=True),
                ]),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NUMBER, value="1", line=1, column=1),
            Token(type=TokenType.COMMA, value=",", line=1, column=2),
            Token(type=TokenType.NAME, value="x", line=1, column=3),
            Token(type=TokenType.EOF, value="", line=1, column=4),
        ]
        parser = GrammarParser(tokens, grammar)
        ast = parser.parse()
        # The separated rep should only consume "1", leaving ",x" for the rest.
        assert _find_token_in_tree(ast, "NAME", "x")


# =============================================================================
# TEST: AST POSITION FIELDS
# =============================================================================


class TestASTPositionFields:
    """Tests for start_line, start_column, end_line, end_column on ASTNode."""

    def test_position_fields_set_from_tokens(self, grammar: object) -> None:
        """Parsed nodes should have position info derived from their token spans."""
        ast = parse_source("42", grammar)
        # The root program node should have position info.
        assert ast.start_line is not None
        assert ast.start_column is not None
        assert ast.end_line is not None
        assert ast.end_column is not None

    def test_position_spans_multiple_tokens(self, grammar: object) -> None:
        """A node spanning '1 + 2' should have start at '1' and end at '2'."""
        ast = parse_source("1 + 2", grammar)
        expression = _find_rule(ast, "expression")
        assert expression is not None
        assert expression.start_line == 1
        assert expression.start_column == 1
        # The end should be at token "2"
        assert expression.end_line == 1
        assert expression.end_column is not None
        assert expression.end_column > 1

    def test_compute_node_position_empty_children(self) -> None:
        """_compute_node_position should return None for empty children."""
        result = _compute_node_position([])
        assert result is None

    def test_compute_node_position_with_tokens(self) -> None:
        """_compute_node_position should return correct span for tokens."""
        t1 = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        t2 = Token(type=TokenType.PLUS, value="+", line=1, column=3)
        t3 = Token(type=TokenType.NUMBER, value="2", line=1, column=5)
        result = _compute_node_position([t1, t2, t3])
        assert result is not None
        assert result["start_line"] == 1
        assert result["start_column"] == 1
        assert result["end_line"] == 1
        assert result["end_column"] == 5

    def test_compute_node_position_nested_nodes(self) -> None:
        """_compute_node_position should find tokens inside nested ASTNodes."""
        t1 = Token(type=TokenType.NUMBER, value="1", line=2, column=5)
        t2 = Token(type=TokenType.NUMBER, value="2", line=3, column=10)
        inner1 = ASTNode(rule_name="a", children=[t1])
        inner2 = ASTNode(rule_name="b", children=[t2])
        result = _compute_node_position([inner1, inner2])
        assert result is not None
        assert result["start_line"] == 2
        assert result["start_column"] == 5
        assert result["end_line"] == 3
        assert result["end_column"] == 10

    def test_find_first_token_empty(self) -> None:
        """_find_first_token on empty list returns None."""
        assert _find_first_token([]) is None

    def test_find_last_token_empty(self) -> None:
        """_find_last_token on empty list returns None."""
        assert _find_last_token([]) is None

    def test_find_first_token_nested(self) -> None:
        """_find_first_token should recurse into ASTNode children."""
        t = Token(type=TokenType.NAME, value="x", line=1, column=1)
        inner = ASTNode(rule_name="inner", children=[t])
        assert _find_first_token([inner]) is t

    def test_find_last_token_nested(self) -> None:
        """_find_last_token should recurse into ASTNode children."""
        t1 = Token(type=TokenType.NAME, value="x", line=1, column=1)
        t2 = Token(type=TokenType.NAME, value="y", line=1, column=3)
        inner = ASTNode(rule_name="inner", children=[t2])
        assert _find_last_token([t1, inner]) is t2

    def test_find_first_token_skips_empty_nodes(self) -> None:
        """_find_first_token should skip ASTNodes with no tokens."""
        empty = ASTNode(rule_name="empty", children=[])
        t = Token(type=TokenType.NUMBER, value="1", line=1, column=5)
        assert _find_first_token([empty, t]) is t

    def test_find_last_token_skips_empty_nodes(self) -> None:
        """_find_last_token should skip ASTNodes with no tokens."""
        t = Token(type=TokenType.NUMBER, value="1", line=1, column=5)
        empty = ASTNode(rule_name="empty", children=[])
        assert _find_last_token([t, empty]) is t


# =============================================================================
# TEST: walk_ast
# =============================================================================


class TestWalkAst:
    """Tests for the walk_ast utility with enter/leave callbacks."""

    def test_walk_enter_visits_all_nodes(self) -> None:
        """Enter callback should be called for every ASTNode."""
        t = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        child = ASTNode(rule_name="child", children=[t])
        root = ASTNode(rule_name="root", children=[child])

        visited: list[str] = []

        def enter(node: ASTNode, parent: ASTNode | None) -> ASTNode | None:
            visited.append(node.rule_name)
            return None

        walk_ast(root, enter=enter)
        assert visited == ["root", "child"]

    def test_walk_leave_visits_all_nodes(self) -> None:
        """Leave callback should be called after children are walked."""
        t = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        child = ASTNode(rule_name="child", children=[t])
        root = ASTNode(rule_name="root", children=[child])

        visited: list[str] = []

        def leave(node: ASTNode, parent: ASTNode | None) -> ASTNode | None:
            visited.append(node.rule_name)
            return None

        walk_ast(root, leave=leave)
        # Leave should visit child before root (depth-first post-order).
        assert visited == ["child", "root"]

    def test_walk_enter_and_leave_order(self) -> None:
        """Enter and leave should interleave correctly for a tree."""
        t = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        child = ASTNode(rule_name="child", children=[t])
        root = ASTNode(rule_name="root", children=[child])

        events: list[str] = []

        def enter(node: ASTNode, parent: ASTNode | None) -> ASTNode | None:
            events.append(f"enter:{node.rule_name}")
            return None

        def leave(node: ASTNode, parent: ASTNode | None) -> ASTNode | None:
            events.append(f"leave:{node.rule_name}")
            return None

        walk_ast(root, enter=enter, leave=leave)
        assert events == [
            "enter:root", "enter:child", "leave:child", "leave:root",
        ]

    def test_walk_enter_replaces_node(self) -> None:
        """Enter callback can replace a node by returning a new ASTNode."""
        t = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        child = ASTNode(rule_name="child", children=[t])
        root = ASTNode(rule_name="root", children=[child])

        def enter(node: ASTNode, parent: ASTNode | None) -> ASTNode | None:
            if node.rule_name == "child":
                return ASTNode(rule_name="replaced", children=node.children)
            return None

        result = walk_ast(root, enter=enter)
        # The root's child should now be "replaced".
        assert result.children[0].rule_name == "replaced"  # type: ignore[union-attr]

    def test_walk_leave_replaces_node(self) -> None:
        """Leave callback can replace a node by returning a new ASTNode."""
        t = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        child = ASTNode(rule_name="child", children=[t])
        root = ASTNode(rule_name="root", children=[child])

        def leave(node: ASTNode, parent: ASTNode | None) -> ASTNode | None:
            if node.rule_name == "root":
                return ASTNode(rule_name="new_root", children=node.children)
            return None

        result = walk_ast(root, leave=leave)
        assert result.rule_name == "new_root"

    def test_walk_parent_tracking(self) -> None:
        """Callbacks should receive the correct parent node."""
        t = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        child = ASTNode(rule_name="child", children=[t])
        root = ASTNode(rule_name="root", children=[child])

        parents: list[str | None] = []

        def enter(node: ASTNode, parent: ASTNode | None) -> ASTNode | None:
            parents.append(parent.rule_name if parent else None)
            return None

        walk_ast(root, enter=enter)
        assert parents == [None, "root"]

    def test_walk_no_callbacks(self) -> None:
        """walk_ast with no callbacks should return the original tree."""
        t = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        root = ASTNode(rule_name="root", children=[t])
        result = walk_ast(root)
        assert result is root


# =============================================================================
# TEST: find_nodes
# =============================================================================


class TestFindNodes:
    """Tests for the find_nodes utility."""

    def test_find_nodes_matching(self) -> None:
        """find_nodes should return all nodes matching the rule name."""
        t1 = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        t2 = Token(type=TokenType.NUMBER, value="2", line=1, column=3)
        leaf1 = ASTNode(rule_name="number", children=[t1])
        leaf2 = ASTNode(rule_name="number", children=[t2])
        other = ASTNode(rule_name="name", children=[
            Token(type=TokenType.NAME, value="x", line=1, column=5),
        ])
        root = ASTNode(rule_name="root", children=[leaf1, other, leaf2])

        results = find_nodes(root, "number")
        assert len(results) == 2
        assert results[0] is leaf1
        assert results[1] is leaf2

    def test_find_nodes_no_match(self) -> None:
        """find_nodes should return an empty list when no nodes match."""
        t = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        root = ASTNode(rule_name="root", children=[t])
        results = find_nodes(root, "nonexistent")
        assert results == []

    def test_find_nodes_includes_root(self) -> None:
        """find_nodes should include the root if it matches."""
        t = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        root = ASTNode(rule_name="target", children=[t])
        results = find_nodes(root, "target")
        assert len(results) == 1
        assert results[0] is root

    def test_find_nodes_nested(self) -> None:
        """find_nodes should find deeply nested matches."""
        t = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        deep = ASTNode(rule_name="target", children=[t])
        mid = ASTNode(rule_name="mid", children=[deep])
        root = ASTNode(rule_name="root", children=[mid])
        results = find_nodes(root, "target")
        assert len(results) == 1
        assert results[0] is deep


# =============================================================================
# TEST: collect_tokens
# =============================================================================


class TestCollectTokens:
    """Tests for the collect_tokens utility."""

    def test_collect_all_tokens(self) -> None:
        """collect_tokens with no filter should return all tokens."""
        t1 = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        t2 = Token(type=TokenType.PLUS, value="+", line=1, column=3)
        t3 = Token(type=TokenType.NUMBER, value="2", line=1, column=5)
        root = ASTNode(rule_name="expr", children=[t1, t2, t3])

        results = collect_tokens(root)
        assert len(results) == 3
        assert [t.value for t in results] == ["1", "+", "2"]

    def test_collect_tokens_filtered(self) -> None:
        """collect_tokens with a type filter should return only matching tokens."""
        t1 = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        t2 = Token(type=TokenType.PLUS, value="+", line=1, column=3)
        t3 = Token(type=TokenType.NUMBER, value="2", line=1, column=5)
        root = ASTNode(rule_name="expr", children=[t1, t2, t3])

        results = collect_tokens(root, "NUMBER")
        assert len(results) == 2
        assert [t.value for t in results] == ["1", "2"]

    def test_collect_tokens_nested(self) -> None:
        """collect_tokens should recurse into nested ASTNodes."""
        t1 = Token(type=TokenType.NUMBER, value="1", line=1, column=1)
        t2 = Token(type=TokenType.NUMBER, value="2", line=1, column=3)
        inner = ASTNode(rule_name="inner", children=[t2])
        root = ASTNode(rule_name="root", children=[t1, inner])

        results = collect_tokens(root, "NUMBER")
        assert len(results) == 2
        assert [t.value for t in results] == ["1", "2"]

    def test_collect_tokens_string_type(self) -> None:
        """collect_tokens should work with string-based token types."""
        t1 = Token(type="INT", value="42", line=1, column=1)
        t2 = Token(type="FLOAT", value="3.14", line=1, column=4)
        root = ASTNode(rule_name="expr", children=[t1, t2])

        results = collect_tokens(root, "INT")
        assert len(results) == 1
        assert results[0].value == "42"

    def test_collect_tokens_empty_tree(self) -> None:
        """collect_tokens on a node with no tokens should return empty list."""
        root = ASTNode(rule_name="empty", children=[])
        results = collect_tokens(root)
        assert results == []


# =============================================================================
# TEST: is_ast_node
# =============================================================================


class TestIsAstNode:
    """Tests for the is_ast_node utility."""

    def test_ast_node_returns_true(self) -> None:
        """is_ast_node should return True for an ASTNode."""
        node = ASTNode(rule_name="test", children=[])
        assert is_ast_node(node) is True

    def test_token_returns_false(self) -> None:
        """is_ast_node should return False for a Token."""
        token = Token(type=TokenType.NUMBER, value="42", line=1, column=1)
        assert is_ast_node(token) is False


# =============================================================================
# TEST: NEWLINE DETECTION IN NEW ELEMENT TYPES
# =============================================================================


class TestNewlineDetectionNewElements:
    """Tests for _element_references_newline with new grammar element types."""

    def test_newline_in_positive_lookahead(self) -> None:
        """Positive lookahead wrapping NEWLINE should be detected."""
        from grammar_tools import (
            GrammarRule,
            ParserGrammar,
            PositiveLookahead,
            RuleReference,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=PositiveLookahead(
                    element=RuleReference(name="NEWLINE", is_token=True),
                ),
                line_number=1,
            ),
        ])

        tokens = [Token(type=TokenType.EOF, value="", line=1, column=1)]
        parser = GrammarParser(tokens, grammar)
        assert parser._newlines_significant is True

    def test_newline_in_negative_lookahead(self) -> None:
        """Negative lookahead wrapping NEWLINE should be detected."""
        from grammar_tools import (
            GrammarRule,
            NegativeLookahead,
            ParserGrammar,
            RuleReference,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=NegativeLookahead(
                    element=RuleReference(name="NEWLINE", is_token=True),
                ),
                line_number=1,
            ),
        ])

        tokens = [Token(type=TokenType.EOF, value="", line=1, column=1)]
        parser = GrammarParser(tokens, grammar)
        assert parser._newlines_significant is True

    def test_newline_in_one_or_more(self) -> None:
        """OneOrMoreRepetition wrapping NEWLINE should be detected."""
        from grammar_tools import (
            GrammarRule,
            OneOrMoreRepetition,
            ParserGrammar,
            RuleReference,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=OneOrMoreRepetition(
                    element=RuleReference(name="NEWLINE", is_token=True),
                ),
                line_number=1,
            ),
        ])

        tokens = [Token(type=TokenType.EOF, value="", line=1, column=1)]
        parser = GrammarParser(tokens, grammar)
        assert parser._newlines_significant is True

    def test_newline_in_separated_repetition_element(self) -> None:
        """SeparatedRepetition with NEWLINE in element should be detected."""
        from grammar_tools import (
            GrammarRule,
            Literal,
            ParserGrammar,
            RuleReference,
            SeparatedRepetition,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=SeparatedRepetition(
                    element=RuleReference(name="NEWLINE", is_token=True),
                    separator=Literal(value=","),
                    at_least_one=False,
                ),
                line_number=1,
            ),
        ])

        tokens = [Token(type=TokenType.EOF, value="", line=1, column=1)]
        parser = GrammarParser(tokens, grammar)
        assert parser._newlines_significant is True

    def test_newline_in_separated_repetition_separator(self) -> None:
        """SeparatedRepetition with NEWLINE in separator should be detected."""
        from grammar_tools import (
            GrammarRule,
            ParserGrammar,
            RuleReference,
            SeparatedRepetition,
        )

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=SeparatedRepetition(
                    element=RuleReference(name="NUMBER", is_token=True),
                    separator=RuleReference(name="NEWLINE", is_token=True),
                    at_least_one=False,
                ),
                line_number=1,
            ),
        ])

        tokens = [Token(type=TokenType.EOF, value="", line=1, column=1)]
        parser = GrammarParser(tokens, grammar)
        assert parser._newlines_significant is True


# =============================================================================
# TEST: PRE-PARSE AND POST-PARSE HOOKS
# =============================================================================


class TestParseHooks:
    """Tests for the pre-parse and post-parse hook pipeline."""

    def test_pre_parse_hook_transforms_tokens(self) -> None:
        """A pre-parse hook should be able to filter/modify the token list."""
        from grammar_tools import GrammarRule, ParserGrammar, RuleReference

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=RuleReference(name="NUMBER", is_token=True),
                line_number=1,
            ),
        ])

        # Tokens include a leading NAME that we'll filter out via hook.
        tokens = [
            Token(type=TokenType.NAME, value="skip", line=1, column=1),
            Token(type=TokenType.NUMBER, value="42", line=1, column=6),
            Token(type=TokenType.EOF, value="", line=1, column=8),
        ]

        def remove_names(toks: list[Token]) -> list[Token]:
            return [t for t in toks if _type_name(t) != "NAME"]

        parser = GrammarParser(tokens, grammar)
        parser.add_pre_parse(remove_names)
        ast = parser.parse()
        assert ast.children[0].value == "42"  # type: ignore[union-attr]

    def test_post_parse_hook_transforms_ast(self) -> None:
        """A post-parse hook should be able to transform the AST."""
        from grammar_tools import GrammarRule, ParserGrammar, RuleReference

        grammar = ParserGrammar(rules=[
            GrammarRule(
                name="start",
                body=RuleReference(name="NUMBER", is_token=True),
                line_number=1,
            ),
        ])

        tokens = [
            Token(type=TokenType.NUMBER, value="42", line=1, column=1),
            Token(type=TokenType.EOF, value="", line=1, column=3),
        ]

        def rename_root(node: ASTNode) -> ASTNode:
            return ASTNode(rule_name="renamed", children=node.children)

        parser = GrammarParser(tokens, grammar)
        parser.add_post_parse(rename_root)
        ast = parser.parse()
        assert ast.rule_name == "renamed"


# =============================================================================
# TREE TRAVERSAL HELPERS
# =============================================================================
#
# These utilities walk the generic AST tree to find specific nodes or tokens.
# They're used by the tests to verify tree structure without hardcoding
# exact positions.
# =============================================================================


def _find_token_in_tree(
    node: ASTNode | Token, token_type: str, value: str,
) -> bool:
    """Recursively search the tree for a token with the given type and value.

    Args:
        node: The root of the subtree to search.
        token_type: The TokenType name to look for (e.g., "NUMBER").
        value: The token value to look for (e.g., "42").

    Returns:
        True if a matching token was found anywhere in the tree.
    """
    if isinstance(node, Token):
        return _type_name(node) == token_type and node.value == value
    return any(
        _find_token_in_tree(child, token_type, value)
        for child in node.children
    )


def _find_rule(node: ASTNode | Token, rule_name: str) -> ASTNode | None:
    """Find the first ASTNode with the given rule_name in the tree.

    Does a depth-first search. Returns the first match found, or None.

    Args:
        node: The root of the subtree to search.
        rule_name: The rule name to look for (e.g., "expression").

    Returns:
        The first matching ASTNode, or None if not found.
    """
    if isinstance(node, Token):
        return None
    if node.rule_name == rule_name:
        return node
    for child in node.children:
        result = _find_rule(child, rule_name)
        if result is not None:
            return result
    return None


def _collect_tokens(node: ASTNode | Token, token_type: str) -> list[Token]:
    """Collect all tokens of a given type from the tree.

    Does a depth-first traversal and returns all matching tokens in order.

    Args:
        node: The root of the subtree to search.
        token_type: The TokenType name to look for (e.g., "NUMBER").

    Returns:
        A list of matching Token objects in tree-traversal order.
    """
    if isinstance(node, Token):
        if _type_name(node) == token_type:
            return [node]
        return []
    tokens: list[Token] = []
    for child in node.children:
        tokens.extend(_collect_tokens(child, token_type))
    return tokens


def _collect_all_tokens(node: ASTNode | Token) -> list[Token]:
    """Collect all tokens from the tree in depth-first order.

    Args:
        node: The root of the subtree to traverse.

    Returns:
        A list of all Token objects in tree-traversal order.
    """
    if isinstance(node, Token):
        return [node]
    tokens: list[Token] = []
    for child in node.children:
        tokens.extend(_collect_all_tokens(child))
    return tokens
