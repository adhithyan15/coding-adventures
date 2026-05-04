"""Tests for tokenizing SWI-Prolog source."""

from __future__ import annotations

from lexer import Token, TokenType

from swi_prolog_lexer import (
    SWI_PROLOG_TOKENS_PATH,
    __version__,
    create_swi_prolog_lexer,
    tokenize_swi_prolog,
)


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
        assert __version__ == "0.1.0"


class TestSwiTokenization:
    """SWI-Prolog source should tokenize through its dedicated grammar file."""

    def test_uses_swi_token_grammar_path(self) -> None:
        assert SWI_PROLOG_TOKENS_PATH.name == "swi.tokens"
        assert SWI_PROLOG_TOKENS_PATH.parent.name == "prolog"

    def test_fact(self) -> None:
        tokens = tokenize_swi_prolog("parent(homer, bart).\n")

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

    def test_line_and_block_comments_are_skipped(self) -> None:
        tokens = tokenize_swi_prolog(
            "% line comment\nparent(homer, bart). /* block comment */\n",
        )

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

    def test_directive_and_backquoted_string(self) -> None:
        tokens = tokenize_swi_prolog(":- initialization(main).\nmessage(`hello`).\n")

        assert token_types(tokens) == [
            "RULE",
            "ATOM",
            "LPAREN",
            "ATOM",
            "RPAREN",
            "DOT",
            "ATOM",
            "LPAREN",
            "STRING",
            "RPAREN",
            "DOT",
            "EOF",
        ]

    def test_dcg_rule_with_braced_goal(self) -> None:
        tokens = tokenize_swi_prolog("digits(X) --> { X = done }, [X].\n")

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

    def test_clpfd_range_operator_is_one_symbolic_atom(self) -> None:
        tokens = tokenize_swi_prolog("?- X in 1..4, X #=< 3.\n")

        assert [(token.type_name, token.value) for token in tokens] == [
            ("QUERY", "?-"),
            ("VARIABLE", "X"),
            ("ATOM", "in"),
            ("INTEGER", "1"),
            ("ATOM", ".."),
            ("INTEGER", "4"),
            ("COMMA", ","),
            ("VARIABLE", "X"),
            ("ATOM", "#=<"),
            ("INTEGER", "3"),
            ("DOT", "."),
            ("EOF", ""),
        ]

    def test_create_swi_prolog_lexer(self) -> None:
        lexer = create_swi_prolog_lexer("likes(marge, donuts).\n")

        assert token_types(lexer.tokenize()) == [
            "ATOM",
            "LPAREN",
            "ATOM",
            "COMMA",
            "ATOM",
            "RPAREN",
            "DOT",
            "EOF",
        ]
