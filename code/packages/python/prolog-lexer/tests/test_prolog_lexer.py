"""Tests for the Prolog lexer."""

from __future__ import annotations

from lexer import Token, TokenType

from prolog_lexer import create_prolog_lexer, tokenize_prolog


def token_types(tokens: list[Token]) -> list[str]:
    """Return token type names as strings for readable assertions."""

    result: list[str] = []
    for token in tokens:
        if isinstance(token.type, TokenType):
            result.append(token.type.name)
        else:
            result.append(token.type)
    return result


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        from prolog_lexer import __version__

        assert __version__ == "0.1.0"


class TestBasicTokenization:
    """Facts, rules, and queries should tokenize into the expected structure."""

    def test_fact(self) -> None:
        tokens = tokenize_prolog("parent(homer, bart).\n")

        assert token_types(tokens) == [
            "ATOM",
            "LPAREN",
            "ATOM",
            "COMMA",
            "ATOM",
            "RPAREN",
            "DOT",
            "EOF",
        ]

    def test_rule(self) -> None:
        tokens = tokenize_prolog("ancestor(X, Y) :- parent(X, Y).\n")

        assert token_types(tokens) == [
            "ATOM",
            "LPAREN",
            "VARIABLE",
            "COMMA",
            "VARIABLE",
            "RPAREN",
            "RULE",
            "ATOM",
            "LPAREN",
            "VARIABLE",
            "COMMA",
            "VARIABLE",
            "RPAREN",
            "DOT",
            "EOF",
        ]

    def test_query(self) -> None:
        tokens = tokenize_prolog("?- member(X, [a, b | T]).\n")

        assert token_types(tokens) == [
            "QUERY",
            "ATOM",
            "LPAREN",
            "VARIABLE",
            "COMMA",
            "LBRACKET",
            "ATOM",
            "COMMA",
            "ATOM",
            "BAR",
            "VARIABLE",
            "RBRACKET",
            "RPAREN",
            "DOT",
            "EOF",
        ]

    def test_dcg_rule(self) -> None:
        tokens = tokenize_prolog("digits([D|Ds]) --> digit(D), digits(Ds).\n")

        assert token_types(tokens) == [
            "ATOM",
            "LPAREN",
            "LBRACKET",
            "VARIABLE",
            "BAR",
            "VARIABLE",
            "RBRACKET",
            "RPAREN",
            "DCG",
            "ATOM",
            "LPAREN",
            "VARIABLE",
            "RPAREN",
            "COMMA",
            "ATOM",
            "LPAREN",
            "VARIABLE",
            "RPAREN",
            "DOT",
            "EOF",
        ]

    def test_dcg_rule_with_braced_goal(self) -> None:
        tokens = tokenize_prolog("digits(X) --> { X = done }, [X].\n")

        assert token_types(tokens) == [
            "ATOM",
            "LPAREN",
            "VARIABLE",
            "RPAREN",
            "DCG",
            "LCURLY",
            "VARIABLE",
            "ATOM",
            "ATOM",
            "RCURLY",
            "COMMA",
            "LBRACKET",
            "VARIABLE",
            "RBRACKET",
            "DOT",
            "EOF",
        ]


class TestAtomsAndVariables:
    """Prolog's core lexical distinction is atoms vs variables."""

    def test_lowercase_atom_and_uppercase_variable(self) -> None:
        tokens = tokenize_prolog("parent(X).\n")

        assert token_types(tokens)[:4] == ["ATOM", "LPAREN", "VARIABLE", "RPAREN"]
        assert tokens[0].value == "parent"
        assert tokens[2].value == "X"

    def test_anonymous_vs_named_underscore_variable(self) -> None:
        tokens = tokenize_prolog("pair(_, _Tmp).\n")

        assert token_types(tokens) == [
            "ATOM",
            "LPAREN",
            "ANON_VAR",
            "COMMA",
            "VARIABLE",
            "RPAREN",
            "DOT",
            "EOF",
        ]

    def test_quoted_atom_is_still_atom(self) -> None:
        tokens = tokenize_prolog("'Hello world'.\n")

        assert token_types(tokens) == ["ATOM", "DOT", "EOF"]
        assert tokens[0].value == "'Hello world'"

    def test_symbolic_operator_atoms(self) -> None:
        tokens = tokenize_prolog("X \\= Y.\n")

        assert token_types(tokens) == ["VARIABLE", "ATOM", "VARIABLE", "DOT", "EOF"]
        assert tokens[1].value == "\\="


class TestLiteralsAndComments:
    """Numbers, strings, and comments should behave predictably."""

    def test_integer_and_float(self) -> None:
        tokens = tokenize_prolog("point(1, 3.14).\n")

        assert token_types(tokens) == [
            "ATOM",
            "LPAREN",
            "INTEGER",
            "COMMA",
            "FLOAT",
            "RPAREN",
            "DOT",
            "EOF",
        ]

    def test_string_literal(self) -> None:
        tokens = tokenize_prolog('say("hello").\n')

        assert token_types(tokens) == [
            "ATOM",
            "LPAREN",
            "STRING",
            "RPAREN",
            "DOT",
            "EOF",
        ]
        assert tokens[2].value == "hello"

    def test_percent_comments_are_skipped(self) -> None:
        tokens = tokenize_prolog("parent(homer, bart). % trailing comment\n")

        assert token_types(tokens) == [
            "ATOM",
            "LPAREN",
            "ATOM",
            "COMMA",
            "ATOM",
            "RPAREN",
            "DOT",
            "EOF",
        ]


class TestHelpersAndPositions:
    """Factory helpers and source positions should be stable."""

    def test_create_prolog_lexer(self) -> None:
        lexer = create_prolog_lexer("likes(marge, donuts).\n")
        tokens = lexer.tokenize()

        assert token_types(tokens) == [
            "ATOM",
            "LPAREN",
            "ATOM",
            "COMMA",
            "ATOM",
            "RPAREN",
            "DOT",
            "EOF",
        ]

    def test_positions_across_lines(self) -> None:
        tokens = tokenize_prolog("parent(homer, bart).\nancestor(X, Y).\n")

        assert tokens[0].line == 1
        assert tokens[0].column == 1
        assert tokens[7].line == 2
        assert tokens[7].column == 1
