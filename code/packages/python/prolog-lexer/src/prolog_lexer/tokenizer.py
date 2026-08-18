"""Prolog Lexer — grammar-driven tokenization for Prolog source code.

The implementation is intentionally thin. All language-specific behavior lives
in ``code/grammars/prolog/prolog.tokens``; that grammar is pre-compiled into
``prolog_lexer._grammar`` (see that module's header for regeneration
instructions), and this module just feeds the compiled grammar to the shared
``GrammarLexer``.
"""

from __future__ import annotations

from lexer import GrammarLexer, Token

from prolog_lexer._grammar import TOKEN_GRAMMAR


def create_prolog_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Prolog source code."""

    return GrammarLexer(source, TOKEN_GRAMMAR)


def tokenize_prolog(source: str) -> list[Token]:
    """Tokenize Prolog source code and return the resulting token stream."""

    lexer = create_prolog_lexer(source)
    return lexer.tokenize()
