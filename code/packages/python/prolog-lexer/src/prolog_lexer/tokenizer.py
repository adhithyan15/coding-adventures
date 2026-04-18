"""Prolog Lexer — grammar-driven tokenization for Prolog source code.

The implementation is intentionally thin. All language-specific behavior lives
in ``code/grammars/prolog.tokens``; this module just locates that grammar,
parses it, and feeds it to the shared ``GrammarLexer``.
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
PROLOG_TOKENS_PATH = GRAMMAR_DIR / "prolog.tokens"


def create_prolog_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Prolog source code."""

    grammar = parse_token_grammar(PROLOG_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_prolog(source: str) -> list[Token]:
    """Tokenize Prolog source code and return the resulting token stream."""

    lexer = create_prolog_lexer(source)
    return lexer.tokenize()
