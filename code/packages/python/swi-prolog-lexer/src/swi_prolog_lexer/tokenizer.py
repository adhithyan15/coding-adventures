"""SWI-Prolog lexer backed by a pre-compiled ``swi.tokens`` grammar."""

from __future__ import annotations

from lexer import GrammarLexer, Token

from swi_prolog_lexer._grammar import TOKEN_GRAMMAR


def create_swi_prolog_lexer(source: str) -> GrammarLexer:
    """Create a grammar-driven lexer configured for SWI-Prolog."""

    return GrammarLexer(source, TOKEN_GRAMMAR)


def tokenize_swi_prolog(source: str) -> list[Token]:
    """Tokenize SWI-Prolog source code."""

    return create_swi_prolog_lexer(source).tokenize()
