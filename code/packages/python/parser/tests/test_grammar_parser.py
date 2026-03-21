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

from lang_parser.grammar_parser import ASTNode, GrammarParseError, GrammarParser

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
