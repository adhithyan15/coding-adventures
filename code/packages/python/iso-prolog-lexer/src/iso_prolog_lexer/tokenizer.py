"""ISO/Core Prolog lexer backed by ``code/grammars/prolog/iso.tokens``."""

from __future__ import annotations

from functools import cache
from pathlib import Path

from grammar_tools import TokenGrammar, parse_token_grammar
from lexer import GrammarLexer, Token

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
ISO_PROLOG_TOKENS_PATH = GRAMMAR_DIR / "prolog" / "iso.tokens"


@cache
def _iso_token_grammar() -> TokenGrammar:
    """Load and cache the ISO/Core Prolog token grammar."""

    return parse_token_grammar(ISO_PROLOG_TOKENS_PATH.read_text())


def create_iso_prolog_lexer(source: str) -> GrammarLexer:
    """Create a grammar-driven lexer configured for ISO/Core Prolog."""

    return GrammarLexer(source, _iso_token_grammar())


def tokenize_iso_prolog(source: str) -> list[Token]:
    """Tokenize ISO/Core Prolog source code."""

    return create_iso_prolog_lexer(source).tokenize()
