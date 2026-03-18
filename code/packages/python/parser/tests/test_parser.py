"""Comprehensive tests for the recursive descent parser.

=============================================================================
TESTING STRATEGY
=============================================================================

These tests verify that the parser correctly transforms token streams into
Abstract Syntax Trees. We construct Token lists manually rather than running
the lexer, so these tests are completely self-contained — a failure here
means the parser is broken, not the lexer.

The tests are organized from simple to complex:

    1. Single atoms (numbers, strings, names)
    2. Binary operations (one operator)
    3. Operator precedence (multiple operators)
    4. Parenthesized expressions (precedence override)
    5. Assignment statements
    6. Multiple statements (programs)
    7. Edge cases and error handling

Each test follows the pattern:
    1. Build a list of Token objects (the input)
    2. Create a Parser and call .parse()
    3. Assert the resulting AST matches the expected structure
=============================================================================
"""

from __future__ import annotations

import pytest

from lexer import Token, TokenType

from lang_parser.parser import (
    Assignment,
    BinaryOp,
    Name,
    NumberLiteral,
    ParseError,
    Parser,
    Program,
    StringLiteral,
)


# =============================================================================
# HELPERS
# =============================================================================
#
# Building token lists by hand is verbose, so we provide a few helper
# functions to reduce the boilerplate. These make the tests much more
# readable — you can see the *meaning* of the token stream at a glance.
# =============================================================================


def tok(token_type: TokenType, value: str, line: int = 1, column: int = 1) -> Token:
    """Create a Token with sensible defaults for line and column.

    In most tests we don't care about exact line/column positions — we just
    want to verify the AST structure. This helper lets us focus on token type
    and value.

    Args:
        token_type: The type of the token (e.g., TokenType.NUMBER).
        value:      The text value of the token (e.g., "42").
        line:       Line number (defaults to 1).
        column:     Column number (defaults to 1).

    Returns:
        A Token with the specified type, value, line, and column.
    """
    return Token(type=token_type, value=value, line=line, column=column)


def eof(line: int = 1, column: int = 1) -> Token:
    """Create an EOF token — the standard end-of-input marker.

    Every token list should end with EOF. This helper makes that explicit
    and reduces clutter in the test token lists.
    """
    return Token(type=TokenType.EOF, value="", line=line, column=column)


def nl(line: int = 1, column: int = 1) -> Token:
    """Create a NEWLINE token — the statement terminator."""
    return Token(type=TokenType.NEWLINE, value="\n", line=line, column=column)


def parse_tokens(tokens: list[Token]) -> Program:
    """Parse a token list and return the Program AST.

    This is the most common operation in our tests: create a parser,
    parse the tokens, return the result.

    Args:
        tokens: A list of Token objects (should end with EOF).

    Returns:
        The parsed Program AST.
    """
    parser = Parser(tokens)
    return parser.parse()


# =============================================================================
# TEST: SINGLE ATOMS (FACTORS)
# =============================================================================
#
# The simplest possible inputs — a single token that forms a complete
# expression. These test the _parse_factor() method in isolation.
# =============================================================================


class TestNumberLiteral:
    """Tests for parsing numeric literals."""

    def test_single_number(self) -> None:
        """A bare number like `42` should parse to NumberLiteral(42)."""
        tokens = [tok(TokenType.NUMBER, "42"), eof()]
        program = parse_tokens(tokens)

        assert len(program.statements) == 1
        assert program.statements[0] == NumberLiteral(value=42)

    def test_zero(self) -> None:
        """Zero is a valid number literal."""
        tokens = [tok(TokenType.NUMBER, "0"), eof()]
        program = parse_tokens(tokens)

        assert program.statements[0] == NumberLiteral(value=0)

    def test_large_number(self) -> None:
        """Large numbers should parse correctly."""
        tokens = [tok(TokenType.NUMBER, "999999"), eof()]
        program = parse_tokens(tokens)

        assert program.statements[0] == NumberLiteral(value=999999)


class TestStringLiteral:
    """Tests for parsing string literals."""

    def test_simple_string(self) -> None:
        """A string like `"hello"` should parse to StringLiteral("hello")."""
        tokens = [tok(TokenType.STRING, "hello"), eof()]
        program = parse_tokens(tokens)

        assert len(program.statements) == 1
        assert program.statements[0] == StringLiteral(value="hello")

    def test_empty_string(self) -> None:
        """An empty string `""` should parse to StringLiteral("")."""
        tokens = [tok(TokenType.STRING, ""), eof()]
        program = parse_tokens(tokens)

        assert program.statements[0] == StringLiteral(value="")

    def test_string_with_spaces(self) -> None:
        """Strings can contain spaces."""
        tokens = [tok(TokenType.STRING, "hello world"), eof()]
        program = parse_tokens(tokens)

        assert program.statements[0] == StringLiteral(value="hello world")


class TestName:
    """Tests for parsing variable names (identifiers)."""

    def test_simple_name(self) -> None:
        """A bare name like `x` should parse to Name("x")."""
        tokens = [tok(TokenType.NAME, "x"), eof()]
        program = parse_tokens(tokens)

        assert len(program.statements) == 1
        assert program.statements[0] == Name(name="x")

    def test_longer_name(self) -> None:
        """Multi-character names work too."""
        tokens = [tok(TokenType.NAME, "total"), eof()]
        program = parse_tokens(tokens)

        assert program.statements[0] == Name(name="total")

    def test_underscore_name(self) -> None:
        """Names with underscores should work."""
        tokens = [tok(TokenType.NAME, "my_var"), eof()]
        program = parse_tokens(tokens)

        assert program.statements[0] == Name(name="my_var")


# =============================================================================
# TEST: BINARY OPERATIONS
# =============================================================================
#
# Two operands connected by an operator. These test _parse_expression()
# and _parse_term() with a single operator.
# =============================================================================


class TestBinaryOp:
    """Tests for binary operations (two operands, one operator)."""

    def test_addition(self) -> None:
        """`1 + 2` should parse to BinaryOp(1, "+", 2)."""
        tokens = [
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "2"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = BinaryOp(
            left=NumberLiteral(1),
            op="+",
            right=NumberLiteral(2),
        )
        assert program.statements[0] == expected

    def test_subtraction(self) -> None:
        """`3 - 1` should parse to BinaryOp(3, "-", 1)."""
        tokens = [
            tok(TokenType.NUMBER, "3"),
            tok(TokenType.MINUS, "-"),
            tok(TokenType.NUMBER, "1"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = BinaryOp(
            left=NumberLiteral(3),
            op="-",
            right=NumberLiteral(1),
        )
        assert program.statements[0] == expected

    def test_multiplication(self) -> None:
        """`4 * 5` should parse to BinaryOp(4, "*", 5)."""
        tokens = [
            tok(TokenType.NUMBER, "4"),
            tok(TokenType.STAR, "*"),
            tok(TokenType.NUMBER, "5"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = BinaryOp(
            left=NumberLiteral(4),
            op="*",
            right=NumberLiteral(5),
        )
        assert program.statements[0] == expected

    def test_division(self) -> None:
        """`10 / 2` should parse to BinaryOp(10, "/", 2)."""
        tokens = [
            tok(TokenType.NUMBER, "10"),
            tok(TokenType.SLASH, "/"),
            tok(TokenType.NUMBER, "2"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = BinaryOp(
            left=NumberLiteral(10),
            op="/",
            right=NumberLiteral(2),
        )
        assert program.statements[0] == expected

    def test_name_in_expression(self) -> None:
        """`x + 1` should parse with Name("x") on the left."""
        tokens = [
            tok(TokenType.NAME, "x"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "1"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = BinaryOp(
            left=Name(name="x"),
            op="+",
            right=NumberLiteral(1),
        )
        assert program.statements[0] == expected


# =============================================================================
# TEST: OPERATOR PRECEDENCE
# =============================================================================
#
# These tests verify that * and / bind tighter than + and -, ensuring the
# tree structure correctly encodes precedence.
# =============================================================================


class TestOperatorPrecedence:
    """Tests for correct operator precedence in the AST."""

    def test_multiplication_before_addition(self) -> None:
        """`1 + 2 * 3` should parse as `1 + (2 * 3)`.

        The multiplication becomes a subtree of the addition node,
        meaning it's evaluated first — exactly what we want.
        """
        tokens = [
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "2"),
            tok(TokenType.STAR, "*"),
            tok(TokenType.NUMBER, "3"),
            eof(),
        ]
        program = parse_tokens(tokens)

        # Expected: BinaryOp(1, "+", BinaryOp(2, "*", 3))
        expected = BinaryOp(
            left=NumberLiteral(1),
            op="+",
            right=BinaryOp(
                left=NumberLiteral(2),
                op="*",
                right=NumberLiteral(3),
            ),
        )
        assert program.statements[0] == expected

    def test_division_before_subtraction(self) -> None:
        """`10 - 6 / 2` should parse as `10 - (6 / 2)`."""
        tokens = [
            tok(TokenType.NUMBER, "10"),
            tok(TokenType.MINUS, "-"),
            tok(TokenType.NUMBER, "6"),
            tok(TokenType.SLASH, "/"),
            tok(TokenType.NUMBER, "2"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = BinaryOp(
            left=NumberLiteral(10),
            op="-",
            right=BinaryOp(
                left=NumberLiteral(6),
                op="/",
                right=NumberLiteral(2),
            ),
        )
        assert program.statements[0] == expected

    def test_left_associativity_addition(self) -> None:
        """`1 + 2 + 3` should parse as `(1 + 2) + 3` (left-associative)."""
        tokens = [
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "2"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "3"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = BinaryOp(
            left=BinaryOp(
                left=NumberLiteral(1),
                op="+",
                right=NumberLiteral(2),
            ),
            op="+",
            right=NumberLiteral(3),
        )
        assert program.statements[0] == expected

    def test_left_associativity_multiplication(self) -> None:
        """`2 * 3 * 4` should parse as `(2 * 3) * 4` (left-associative)."""
        tokens = [
            tok(TokenType.NUMBER, "2"),
            tok(TokenType.STAR, "*"),
            tok(TokenType.NUMBER, "3"),
            tok(TokenType.STAR, "*"),
            tok(TokenType.NUMBER, "4"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = BinaryOp(
            left=BinaryOp(
                left=NumberLiteral(2),
                op="*",
                right=NumberLiteral(3),
            ),
            op="*",
            right=NumberLiteral(4),
        )
        assert program.statements[0] == expected

    def test_complex_precedence(self) -> None:
        """`1 + 2 * 3 + 4` should parse as `(1 + (2 * 3)) + 4`."""
        tokens = [
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "2"),
            tok(TokenType.STAR, "*"),
            tok(TokenType.NUMBER, "3"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "4"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = BinaryOp(
            left=BinaryOp(
                left=NumberLiteral(1),
                op="+",
                right=BinaryOp(
                    left=NumberLiteral(2),
                    op="*",
                    right=NumberLiteral(3),
                ),
            ),
            op="+",
            right=NumberLiteral(4),
        )
        assert program.statements[0] == expected


# =============================================================================
# TEST: PARENTHESIZED EXPRESSIONS
# =============================================================================
#
# Parentheses let the programmer override the default precedence.
# The parser handles this by recursively calling _parse_expression()
# when it sees a `(`, then expecting a `)` afterward.
# =============================================================================


class TestParentheses:
    """Tests for parenthesized expressions."""

    def test_simple_parentheses(self) -> None:
        """`(1 + 2)` should parse the same as `1 + 2`.

        Parentheses around a single expression don't change the tree,
        but they should still parse correctly.
        """
        tokens = [
            tok(TokenType.LPAREN, "("),
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "2"),
            tok(TokenType.RPAREN, ")"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = BinaryOp(
            left=NumberLiteral(1),
            op="+",
            right=NumberLiteral(2),
        )
        assert program.statements[0] == expected

    def test_parentheses_override_precedence(self) -> None:
        """`(1 + 2) * 3` should parse as `(1 + 2) * 3`, NOT `1 + (2 * 3)`.

        Without parentheses, multiplication would bind tighter. The parens
        force addition to happen first by making it a deeper subtree.
        """
        tokens = [
            tok(TokenType.LPAREN, "("),
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "2"),
            tok(TokenType.RPAREN, ")"),
            tok(TokenType.STAR, "*"),
            tok(TokenType.NUMBER, "3"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = BinaryOp(
            left=BinaryOp(
                left=NumberLiteral(1),
                op="+",
                right=NumberLiteral(2),
            ),
            op="*",
            right=NumberLiteral(3),
        )
        assert program.statements[0] == expected

    def test_nested_parentheses(self) -> None:
        """`((1 + 2))` should handle nested parens correctly."""
        tokens = [
            tok(TokenType.LPAREN, "("),
            tok(TokenType.LPAREN, "("),
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "2"),
            tok(TokenType.RPAREN, ")"),
            tok(TokenType.RPAREN, ")"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = BinaryOp(
            left=NumberLiteral(1),
            op="+",
            right=NumberLiteral(2),
        )
        assert program.statements[0] == expected

    def test_parentheses_with_name(self) -> None:
        """`(x + 1) * y` mixes names and parentheses."""
        tokens = [
            tok(TokenType.LPAREN, "("),
            tok(TokenType.NAME, "x"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.RPAREN, ")"),
            tok(TokenType.STAR, "*"),
            tok(TokenType.NAME, "y"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = BinaryOp(
            left=BinaryOp(
                left=Name(name="x"),
                op="+",
                right=NumberLiteral(1),
            ),
            op="*",
            right=Name(name="y"),
        )
        assert program.statements[0] == expected

    def test_parenthesized_single_number(self) -> None:
        """`(42)` wrapping a single number should unwrap to NumberLiteral."""
        tokens = [
            tok(TokenType.LPAREN, "("),
            tok(TokenType.NUMBER, "42"),
            tok(TokenType.RPAREN, ")"),
            eof(),
        ]
        program = parse_tokens(tokens)

        assert program.statements[0] == NumberLiteral(value=42)


# =============================================================================
# TEST: ASSIGNMENT STATEMENTS
# =============================================================================
#
# Assignments bind a name to a value. They use the pattern:
#   NAME EQUALS expression NEWLINE
# =============================================================================


class TestAssignment:
    """Tests for assignment statements."""

    def test_simple_assignment(self) -> None:
        """`x = 42\\n` should parse to Assignment(Name("x"), NumberLiteral(42))."""
        tokens = [
            tok(TokenType.NAME, "x"),
            tok(TokenType.EQUALS, "="),
            tok(TokenType.NUMBER, "42"),
            nl(),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = Assignment(
            target=Name(name="x"),
            value=NumberLiteral(value=42),
        )
        assert len(program.statements) == 1
        assert program.statements[0] == expected

    def test_assignment_with_expression(self) -> None:
        """`x = 1 + 2\\n` should parse the right side as a BinaryOp."""
        tokens = [
            tok(TokenType.NAME, "x"),
            tok(TokenType.EQUALS, "="),
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "2"),
            nl(),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = Assignment(
            target=Name(name="x"),
            value=BinaryOp(
                left=NumberLiteral(1),
                op="+",
                right=NumberLiteral(2),
            ),
        )
        assert program.statements[0] == expected

    def test_assignment_with_complex_expression(self) -> None:
        """`result = 1 + 2 * 3\\n` — precedence applies in the value."""
        tokens = [
            tok(TokenType.NAME, "result"),
            tok(TokenType.EQUALS, "="),
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "2"),
            tok(TokenType.STAR, "*"),
            tok(TokenType.NUMBER, "3"),
            nl(),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = Assignment(
            target=Name(name="result"),
            value=BinaryOp(
                left=NumberLiteral(1),
                op="+",
                right=BinaryOp(
                    left=NumberLiteral(2),
                    op="*",
                    right=NumberLiteral(3),
                ),
            ),
        )
        assert program.statements[0] == expected

    def test_assignment_with_string(self) -> None:
        """`name = "hello"\\n` should assign a string literal."""
        tokens = [
            tok(TokenType.NAME, "name"),
            tok(TokenType.EQUALS, "="),
            tok(TokenType.STRING, "hello"),
            nl(),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = Assignment(
            target=Name(name="name"),
            value=StringLiteral(value="hello"),
        )
        assert program.statements[0] == expected

    def test_assignment_without_trailing_newline(self) -> None:
        """`x = 42` at EOF (no trailing newline) should still parse."""
        tokens = [
            tok(TokenType.NAME, "x"),
            tok(TokenType.EQUALS, "="),
            tok(TokenType.NUMBER, "42"),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = Assignment(
            target=Name(name="x"),
            value=NumberLiteral(value=42),
        )
        assert program.statements[0] == expected

    def test_assignment_with_parenthesized_value(self) -> None:
        """`x = (1 + 2) * 3\\n` — parentheses in the value expression."""
        tokens = [
            tok(TokenType.NAME, "x"),
            tok(TokenType.EQUALS, "="),
            tok(TokenType.LPAREN, "("),
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "2"),
            tok(TokenType.RPAREN, ")"),
            tok(TokenType.STAR, "*"),
            tok(TokenType.NUMBER, "3"),
            nl(),
            eof(),
        ]
        program = parse_tokens(tokens)

        expected = Assignment(
            target=Name(name="x"),
            value=BinaryOp(
                left=BinaryOp(
                    left=NumberLiteral(1),
                    op="+",
                    right=NumberLiteral(2),
                ),
                op="*",
                right=NumberLiteral(3),
            ),
        )
        assert program.statements[0] == expected


# =============================================================================
# TEST: MULTIPLE STATEMENTS (PROGRAMS)
# =============================================================================
#
# Real programs have multiple statements. These tests verify that the
# parser correctly handles newline-separated statement sequences.
# =============================================================================


class TestMultipleStatements:
    """Tests for programs with multiple statements."""

    def test_two_assignments(self) -> None:
        """`x = 1\\ny = 2\\n` should produce two assignment nodes."""
        tokens = [
            tok(TokenType.NAME, "x", line=1),
            tok(TokenType.EQUALS, "=", line=1),
            tok(TokenType.NUMBER, "1", line=1),
            nl(line=1),
            tok(TokenType.NAME, "y", line=2),
            tok(TokenType.EQUALS, "=", line=2),
            tok(TokenType.NUMBER, "2", line=2),
            nl(line=2),
            eof(line=3),
        ]
        program = parse_tokens(tokens)

        assert len(program.statements) == 2
        assert program.statements[0] == Assignment(
            target=Name("x"), value=NumberLiteral(1)
        )
        assert program.statements[1] == Assignment(
            target=Name("y"), value=NumberLiteral(2)
        )

    def test_assignment_then_expression(self) -> None:
        """`x = 1\\nx + 2\\n` — assignment followed by expression statement."""
        tokens = [
            tok(TokenType.NAME, "x", line=1),
            tok(TokenType.EQUALS, "=", line=1),
            tok(TokenType.NUMBER, "1", line=1),
            nl(line=1),
            tok(TokenType.NAME, "x", line=2),
            tok(TokenType.PLUS, "+", line=2),
            tok(TokenType.NUMBER, "2", line=2),
            nl(line=2),
            eof(line=3),
        ]
        program = parse_tokens(tokens)

        assert len(program.statements) == 2
        assert program.statements[0] == Assignment(
            target=Name("x"), value=NumberLiteral(1)
        )
        assert program.statements[1] == BinaryOp(
            left=Name("x"), op="+", right=NumberLiteral(2)
        )

    def test_three_statements(self) -> None:
        """Three statements in sequence."""
        tokens = [
            # a = 10
            tok(TokenType.NAME, "a", line=1),
            tok(TokenType.EQUALS, "=", line=1),
            tok(TokenType.NUMBER, "10", line=1),
            nl(line=1),
            # b = 20
            tok(TokenType.NAME, "b", line=2),
            tok(TokenType.EQUALS, "=", line=2),
            tok(TokenType.NUMBER, "20", line=2),
            nl(line=2),
            # a + b
            tok(TokenType.NAME, "a", line=3),
            tok(TokenType.PLUS, "+", line=3),
            tok(TokenType.NAME, "b", line=3),
            nl(line=3),
            eof(line=4),
        ]
        program = parse_tokens(tokens)

        assert len(program.statements) == 3

    def test_blank_lines_between_statements(self) -> None:
        """Blank lines (extra newlines) between statements should be skipped."""
        tokens = [
            tok(TokenType.NUMBER, "1"),
            nl(),
            nl(),  # blank line
            nl(),  # another blank line
            tok(TokenType.NUMBER, "2"),
            nl(),
            eof(),
        ]
        program = parse_tokens(tokens)

        assert len(program.statements) == 2
        assert program.statements[0] == NumberLiteral(1)
        assert program.statements[1] == NumberLiteral(2)

    def test_leading_newlines(self) -> None:
        """Leading blank lines before any statements should be skipped."""
        tokens = [
            nl(),
            nl(),
            tok(TokenType.NUMBER, "42"),
            eof(),
        ]
        program = parse_tokens(tokens)

        assert len(program.statements) == 1
        assert program.statements[0] == NumberLiteral(42)


# =============================================================================
# TEST: EMPTY PROGRAM
# =============================================================================


class TestEmptyProgram:
    """Tests for edge cases with empty or minimal input."""

    def test_empty_program(self) -> None:
        """An empty program (just EOF) should produce an empty statement list."""
        tokens = [eof()]
        program = parse_tokens(tokens)

        assert program == Program(statements=[])

    def test_only_newlines(self) -> None:
        """A program with only newlines should produce an empty statement list."""
        tokens = [nl(), nl(), nl(), eof()]
        program = parse_tokens(tokens)

        assert program == Program(statements=[])


# =============================================================================
# TEST: ERROR HANDLING
# =============================================================================
#
# The parser should raise clear, informative errors when it encounters
# invalid syntax. Good error messages are critical for a good developer
# experience.
# =============================================================================


class TestErrors:
    """Tests for parser error handling."""

    def test_unexpected_token_at_start(self) -> None:
        """A stray operator at the start should raise ParseError."""
        tokens = [
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "1"),
            eof(),
        ]
        with pytest.raises(ParseError, match="Unexpected token"):
            parse_tokens(tokens)

    def test_missing_closing_paren(self) -> None:
        """`(1 + 2` without closing paren should raise ParseError."""
        tokens = [
            tok(TokenType.LPAREN, "("),
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "2"),
            eof(),
        ]
        with pytest.raises(ParseError, match="Expected RPAREN, got EOF"):
            parse_tokens(tokens)

    def test_unexpected_equals(self) -> None:
        """`= 42` (equals without a name on the left) should be an error."""
        tokens = [
            tok(TokenType.EQUALS, "="),
            tok(TokenType.NUMBER, "42"),
            eof(),
        ]
        with pytest.raises(ParseError, match="Unexpected token"):
            parse_tokens(tokens)

    def test_error_includes_token_info(self) -> None:
        """ParseError should include line and column information."""
        bad_token = tok(TokenType.RPAREN, ")", line=3, column=7)
        tokens = [bad_token, eof()]

        with pytest.raises(ParseError) as exc_info:
            parse_tokens(tokens)

        error = exc_info.value
        assert error.token == bad_token
        assert "line 3" in str(error)
        assert "column 7" in str(error)

    def test_missing_operand_after_plus(self) -> None:
        """`1 +` with no right operand should raise ParseError."""
        tokens = [
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            eof(),
        ]
        with pytest.raises(ParseError, match="Unexpected token"):
            parse_tokens(tokens)

    def test_missing_operand_after_star(self) -> None:
        """`2 *` with no right operand should raise ParseError."""
        tokens = [
            tok(TokenType.NUMBER, "2"),
            tok(TokenType.STAR, "*"),
            eof(),
        ]
        with pytest.raises(ParseError, match="Unexpected token"):
            parse_tokens(tokens)

    def test_double_operator(self) -> None:
        """`1 + + 2` — double operator should be an error."""
        tokens = [
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "2"),
            eof(),
        ]
        with pytest.raises(ParseError, match="Unexpected token"):
            parse_tokens(tokens)

    def test_unexpected_rparen(self) -> None:
        """A stray `)` without matching `(` should be caught."""
        tokens = [
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.RPAREN, ")"),
            eof(),
        ]
        # The parser will parse NumberLiteral(1) as an expression statement,
        # then try to consume NEWLINE but find RPAREN instead.
        with pytest.raises(ParseError, match="Expected NEWLINE, got RPAREN"):
            parse_tokens(tokens)


# =============================================================================
# TEST: EXPRESSION STATEMENTS
# =============================================================================
#
# When an expression appears on its own line (not as part of an assignment),
# it's an expression statement. The parser should handle these correctly,
# distinguishing them from assignments.
# =============================================================================


class TestExpressionStatements:
    """Tests for expression statements (expressions used as statements)."""

    def test_number_expression_statement(self) -> None:
        """`42\\n` — a number on its own line."""
        tokens = [
            tok(TokenType.NUMBER, "42"),
            nl(),
            eof(),
        ]
        program = parse_tokens(tokens)

        assert len(program.statements) == 1
        assert program.statements[0] == NumberLiteral(42)

    def test_binary_expression_statement(self) -> None:
        """`1 + 2\\n` — an expression on its own line."""
        tokens = [
            tok(TokenType.NUMBER, "1"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "2"),
            nl(),
            eof(),
        ]
        program = parse_tokens(tokens)

        assert len(program.statements) == 1
        assert program.statements[0] == BinaryOp(
            left=NumberLiteral(1), op="+", right=NumberLiteral(2)
        )

    def test_name_expression_statement(self) -> None:
        """`x\\n` — a name on its own line (not an assignment)."""
        tokens = [
            tok(TokenType.NAME, "x"),
            nl(),
            eof(),
        ]
        program = parse_tokens(tokens)

        assert len(program.statements) == 1
        assert program.statements[0] == Name(name="x")

    def test_name_not_confused_with_assignment(self) -> None:
        """`x + 1\\n` — name followed by operator, not assignment."""
        tokens = [
            tok(TokenType.NAME, "x"),
            tok(TokenType.PLUS, "+"),
            tok(TokenType.NUMBER, "1"),
            nl(),
            eof(),
        ]
        program = parse_tokens(tokens)

        assert isinstance(program.statements[0], BinaryOp)


# =============================================================================
# TEST: END-TO-END (THE TARGET EXPRESSION)
# =============================================================================
#
# This is the "money test" — the expression from the spec that demonstrates
# the parser working end-to-end: `x = 1 + 2`
# =============================================================================


class TestEndToEnd:
    """End-to-end tests for the complete parser pipeline."""

    def test_target_expression(self) -> None:
        """The target expression `x = 1 + 2` should produce the correct AST.

        This is the canonical example from the spec. It exercises:
        - Assignment parsing
        - Expression parsing with a binary operator
        - Number literals
        - Variable names
        """
        tokens = [
            tok(TokenType.NAME, "x", line=1, column=1),
            tok(TokenType.EQUALS, "=", line=1, column=3),
            tok(TokenType.NUMBER, "1", line=1, column=5),
            tok(TokenType.PLUS, "+", line=1, column=7),
            tok(TokenType.NUMBER, "2", line=1, column=9),
            nl(line=1, column=10),
            eof(line=2, column=1),
        ]
        program = parse_tokens(tokens)

        expected = Program(
            statements=[
                Assignment(
                    target=Name(name="x"),
                    value=BinaryOp(
                        left=NumberLiteral(value=1),
                        op="+",
                        right=NumberLiteral(value=2),
                    ),
                )
            ]
        )
        assert program == expected

    def test_full_program(self) -> None:
        """A multi-line program with assignments and expressions.

        Simulates:
            x = 10
            y = 20
            x + y * 2
        """
        tokens = [
            # x = 10
            tok(TokenType.NAME, "x", line=1, column=1),
            tok(TokenType.EQUALS, "=", line=1, column=3),
            tok(TokenType.NUMBER, "10", line=1, column=5),
            nl(line=1),
            # y = 20
            tok(TokenType.NAME, "y", line=2, column=1),
            tok(TokenType.EQUALS, "=", line=2, column=3),
            tok(TokenType.NUMBER, "20", line=2, column=5),
            nl(line=2),
            # x + y * 2
            tok(TokenType.NAME, "x", line=3, column=1),
            tok(TokenType.PLUS, "+", line=3, column=3),
            tok(TokenType.NAME, "y", line=3, column=5),
            tok(TokenType.STAR, "*", line=3, column=7),
            tok(TokenType.NUMBER, "2", line=3, column=9),
            nl(line=3),
            eof(line=4),
        ]
        program = parse_tokens(tokens)

        assert len(program.statements) == 3
        # x = 10
        assert program.statements[0] == Assignment(
            target=Name("x"), value=NumberLiteral(10)
        )
        # y = 20
        assert program.statements[1] == Assignment(
            target=Name("y"), value=NumberLiteral(20)
        )
        # x + y * 2  →  x + (y * 2)
        assert program.statements[2] == BinaryOp(
            left=Name("x"),
            op="+",
            right=BinaryOp(left=Name("y"), op="*", right=NumberLiteral(2)),
        )


# =============================================================================
# TEST: PARSER CLASS API
# =============================================================================


class TestParserAPI:
    """Tests for the Parser class interface."""

    def test_parse_returns_program(self) -> None:
        """parse() should always return a Program node."""
        tokens = [eof()]
        parser = Parser(tokens)
        result = parser.parse()

        assert isinstance(result, Program)

    def test_parse_error_is_exception(self) -> None:
        """ParseError should be a proper Exception subclass."""
        token = tok(TokenType.PLUS, "+")
        error = ParseError("test message", token)

        assert isinstance(error, Exception)
        assert error.message == "test message"
        assert error.token == token

    def test_parse_error_str(self) -> None:
        """ParseError string representation includes location."""
        token = tok(TokenType.PLUS, "+", line=5, column=10)
        error = ParseError("bad syntax", token)

        assert "bad syntax" in str(error)
        assert "line 5" in str(error)
        assert "column 10" in str(error)
