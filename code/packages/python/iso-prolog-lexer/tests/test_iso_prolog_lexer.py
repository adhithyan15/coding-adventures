"""Tests for tokenizing ISO/Core Prolog source."""

from __future__ import annotations

from lexer import Token, TokenType

from iso_prolog_lexer import (
    ISO_PROLOG_TOKENS_PATH,
    __version__,
    create_iso_prolog_lexer,
    tokenize_iso_prolog,
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


class TestIsoTokenization:
    """ISO/Core source should tokenize through its dedicated grammar file."""

    def test_uses_iso_token_grammar_path(self) -> None:
        assert ISO_PROLOG_TOKENS_PATH.name == "iso.tokens"
        assert ISO_PROLOG_TOKENS_PATH.parent.name == "prolog"

    def test_fact(self) -> None:
        tokens = tokenize_iso_prolog("parent(homer, bart).\n")

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

    def test_rule_query_and_list(self) -> None:
        tokens = tokenize_iso_prolog("?- member(X, [a, b | T]).\n")

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

    def test_dcg_rule_with_braced_goal(self) -> None:
        tokens = tokenize_iso_prolog("digits(X) --> { X = done }, [X].\n")

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

    def test_create_iso_prolog_lexer(self) -> None:
        lexer = create_iso_prolog_lexer("likes(marge, donuts).\n")

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
