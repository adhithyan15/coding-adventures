"""ISO/Core Prolog lexer backed by a pre-compiled ``iso.tokens`` grammar."""

from __future__ import annotations

from lexer import GrammarLexer, Token

from iso_prolog_lexer._grammar import TOKEN_GRAMMAR


def create_iso_prolog_lexer(source: str) -> GrammarLexer:
    """Create a grammar-driven lexer configured for ISO/Core Prolog."""

    return GrammarLexer(source, TOKEN_GRAMMAR)


def tokenize_iso_prolog(source: str) -> list[Token]:
    """Tokenize ISO/Core Prolog source code."""

    return create_iso_prolog_lexer(source).tokenize()
