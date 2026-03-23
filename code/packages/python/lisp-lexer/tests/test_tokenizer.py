"""Tests for the Lisp lexer.

These tests verify that the Lisp lexer correctly tokenizes:
- Atoms: numbers, symbols, strings
- Delimiters: parentheses, quote, dot
- Whitespace and comments are skipped
- Symbol characters: +, -, *, /, =, <, >, !, ?, &
- Edge cases: negative numbers, single-char symbols
"""

from __future__ import annotations

import pytest

from lisp_lexer import tokenize_lisp


def _type_name(t: object) -> str:
    """Get the type name of a token, handling both enum and string types."""
    token_type = t.type  # type: ignore[union-attr]
    return token_type if isinstance(token_type, str) else token_type.name


def _types(source: str) -> list[str]:
    """Tokenize and return just the token type names (excluding EOF)."""
    return [_type_name(t) for t in tokenize_lisp(source) if _type_name(t) != "EOF"]


def _values(source: str) -> list[str]:
    """Tokenize and return just the token values (excluding EOF)."""
    return [t.value for t in tokenize_lisp(source) if _type_name(t) != "EOF"]


# -------------------------------------------------------------------------
# Basic atoms
# -------------------------------------------------------------------------


class TestAtoms:
    """Tests for tokenizing atomic values."""

    def test_number(self) -> None:
        """Positive integers should tokenize as NUMBER."""
        assert _types("42") == ["NUMBER"]
        assert _values("42") == ["42"]

    def test_negative_number(self) -> None:
        """Negative integers should tokenize as a single NUMBER."""
        assert _types("-7") == ["NUMBER"]
        assert _values("-7") == ["-7"]

    def test_zero(self) -> None:
        """Zero should tokenize as NUMBER."""
        assert _types("0") == ["NUMBER"]

    def test_symbol(self) -> None:
        """Identifiers should tokenize as SYMBOL."""
        assert _types("define") == ["SYMBOL"]
        assert _values("define") == ["define"]

    def test_string(self) -> None:
        """Double-quoted strings should tokenize as STRING."""
        tokens = tokenize_lisp('"hello world"')
        non_eof = [t for t in tokens if _type_name(t) != "EOF"]
        assert len(non_eof) == 1
        assert _type_name(non_eof[0]) == "STRING"

    def test_string_with_escape(self) -> None:
        """Strings with escape sequences should tokenize correctly."""
        tokens = tokenize_lisp(r'"hello \"world\""')
        non_eof = [t for t in tokens if _type_name(t) != "EOF"]
        assert len(non_eof) == 1
        assert _type_name(non_eof[0]) == "STRING"


# -------------------------------------------------------------------------
# Operator symbols
# -------------------------------------------------------------------------


class TestOperatorSymbols:
    """Lisp symbols can contain operator characters like +, -, *, /."""

    def test_plus(self) -> None:
        """The + symbol should tokenize as SYMBOL."""
        assert _types("+") == ["SYMBOL"]
        assert _values("+") == ["+"]

    def test_minus(self) -> None:
        """The - symbol (not followed by digit) should tokenize as SYMBOL."""
        # In isolation, '-' is a SYMBOL
        # But '-42' is a NUMBER (priority ordering)
        assert _types("(- 3 1)") == ["LPAREN", "SYMBOL", "NUMBER", "NUMBER", "RPAREN"]

    def test_star(self) -> None:
        """The * symbol should tokenize as SYMBOL."""
        assert _types("*") == ["SYMBOL"]

    def test_slash(self) -> None:
        """The / symbol should tokenize as SYMBOL."""
        assert _types("/") == ["SYMBOL"]

    def test_equals(self) -> None:
        """The = symbol should tokenize as SYMBOL."""
        assert _types("=") == ["SYMBOL"]

    def test_comparison(self) -> None:
        """Comparison operators should tokenize as SYMBOL."""
        assert _types("< > <= >=") == ["SYMBOL", "SYMBOL", "SYMBOL", "SYMBOL"]

    def test_multi_char_symbol(self) -> None:
        """Multi-character symbols with operators should work."""
        assert _types("set!") == ["SYMBOL"]
        assert _values("set!") == ["set!"]
        assert _types("null?") == ["SYMBOL"]
        assert _values("null?") == ["null?"]


# -------------------------------------------------------------------------
# Delimiters
# -------------------------------------------------------------------------


class TestDelimiters:
    """Tests for parentheses, quote, and dot."""

    def test_parentheses(self) -> None:
        """Parentheses should tokenize as LPAREN and RPAREN."""
        assert _types("()") == ["LPAREN", "RPAREN"]

    def test_quote(self) -> None:
        """Single quote should tokenize as QUOTE."""
        assert _types("'x") == ["QUOTE", "SYMBOL"]

    def test_dot(self) -> None:
        """Dot should tokenize as DOT."""
        assert _types("(a . b)") == ["LPAREN", "SYMBOL", "DOT", "SYMBOL", "RPAREN"]


# -------------------------------------------------------------------------
# Whitespace and comments
# -------------------------------------------------------------------------


class TestSkipping:
    """Tests for skipping whitespace and comments."""

    def test_whitespace_skipped(self) -> None:
        """Spaces, tabs, and newlines should be skipped."""
        assert _types("  42  ") == ["NUMBER"]
        assert _types("a\tb") == ["SYMBOL", "SYMBOL"]
        assert _types("a\nb") == ["SYMBOL", "SYMBOL"]

    def test_comment_skipped(self) -> None:
        """Comments (starting with ;) should be skipped."""
        assert _types("; this is a comment\n42") == ["NUMBER"]

    def test_inline_comment(self) -> None:
        """Comments after code should be skipped."""
        assert _types("(+ 1 2) ; add them") == [
            "LPAREN", "SYMBOL", "NUMBER", "NUMBER", "RPAREN"
        ]


# -------------------------------------------------------------------------
# Full expressions
# -------------------------------------------------------------------------


class TestExpressions:
    """Tests for tokenizing complete Lisp expressions."""

    def test_simple_call(self) -> None:
        """A simple function call should tokenize correctly."""
        assert _types("(+ 1 2)") == [
            "LPAREN", "SYMBOL", "NUMBER", "NUMBER", "RPAREN"
        ]

    def test_nested_call(self) -> None:
        """Nested function calls should tokenize correctly."""
        assert _types("(+ (* 2 3) 4)") == [
            "LPAREN", "SYMBOL",
            "LPAREN", "SYMBOL", "NUMBER", "NUMBER", "RPAREN",
            "NUMBER", "RPAREN"
        ]

    def test_define(self) -> None:
        """A define expression should tokenize correctly."""
        result = _types("(define x 42)")
        assert result == ["LPAREN", "SYMBOL", "SYMBOL", "NUMBER", "RPAREN"]

    def test_lambda(self) -> None:
        """A lambda expression should tokenize correctly."""
        result = _types("(lambda (x) (* x x))")
        assert result == [
            "LPAREN", "SYMBOL",
            "LPAREN", "SYMBOL", "RPAREN",
            "LPAREN", "SYMBOL", "SYMBOL", "SYMBOL", "RPAREN",
            "RPAREN"
        ]

    def test_quoted_symbol(self) -> None:
        """A quoted symbol should tokenize as QUOTE + SYMBOL."""
        assert _types("'foo") == ["QUOTE", "SYMBOL"]

    def test_quoted_list(self) -> None:
        """A quoted list should tokenize correctly."""
        assert _types("'(1 2 3)") == [
            "QUOTE", "LPAREN", "NUMBER", "NUMBER", "NUMBER", "RPAREN"
        ]

    def test_dotted_pair(self) -> None:
        """A dotted pair should tokenize correctly."""
        assert _types("(1 . 2)") == [
            "LPAREN", "NUMBER", "DOT", "NUMBER", "RPAREN"
        ]

    def test_cond_expression(self) -> None:
        """A cond expression should tokenize correctly."""
        source = "(cond ((eq x 0) 1) (t x))"
        types = _types(source)
        assert types == [
            "LPAREN", "SYMBOL",
            "LPAREN", "LPAREN", "SYMBOL", "SYMBOL", "NUMBER", "RPAREN",
            "NUMBER", "RPAREN",
            "LPAREN", "SYMBOL", "SYMBOL", "RPAREN",
            "RPAREN"
        ]

    def test_factorial(self) -> None:
        """The factorial definition should tokenize correctly."""
        source = """
        (define factorial
          (lambda (n)
            (cond ((eq n 0) 1)
                  (t (* n (factorial (- n 1)))))))
        """
        tokens = [t for t in tokenize_lisp(source) if _type_name(t) != "EOF"]
        # Just verify it tokenizes without error and has reasonable count
        assert len(tokens) > 20
        # First meaningful tokens should be: ( define factorial
        assert _type_name(tokens[0]) == "LPAREN"
        assert tokens[1].value == "define"
        assert tokens[2].value == "factorial"

    def test_empty_input(self) -> None:
        """Empty input should produce only EOF."""
        tokens = tokenize_lisp("")
        assert len(tokens) == 1
        assert _type_name(tokens[0]) == "EOF"

    def test_only_comments(self) -> None:
        """Input with only comments should produce only EOF."""
        tokens = tokenize_lisp("; just a comment\n; another one")
        assert len(tokens) == 1
        assert _type_name(tokens[0]) == "EOF"

    def test_eof_always_present(self) -> None:
        """Every token list should end with EOF."""
        tokens = tokenize_lisp("(+ 1 2)")
        assert _type_name(tokens[-1]) == "EOF"


# -------------------------------------------------------------------------
# Number vs Symbol disambiguation
# -------------------------------------------------------------------------


class TestNumberSymbolDisambiguation:
    """Tests for the NUMBER vs SYMBOL priority ordering."""

    def test_negative_number_in_context(self) -> None:
        """Negative numbers should be NUMBER, not SYMBOL + NUMBER."""
        # -42 should be a single NUMBER token
        assert _types("-42") == ["NUMBER"]
        assert _values("-42") == ["-42"]

    def test_subtraction_expression(self) -> None:
        """In (- 3 1), the - is a SYMBOL (followed by space, not digit)."""
        types = _types("(- 3 1)")
        assert types == ["LPAREN", "SYMBOL", "NUMBER", "NUMBER", "RPAREN"]
        values = _values("(- 3 1)")
        assert values == ["(", "-", "3", "1", ")"]
