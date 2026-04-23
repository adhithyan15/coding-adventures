"""SWI-Prolog lexer backed by ``code/grammars/prolog/swi.tokens``."""

from __future__ import annotations

from functools import cache
from pathlib import Path

from grammar_tools import TokenGrammar, parse_token_grammar
from lexer import GrammarLexer, Token

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
SWI_PROLOG_TOKENS_PATH = GRAMMAR_DIR / "prolog" / "swi.tokens"


@cache
def _swi_token_grammar() -> TokenGrammar:
    """Load and cache the SWI-Prolog token grammar."""

    return parse_token_grammar(SWI_PROLOG_TOKENS_PATH.read_text())


def create_swi_prolog_lexer(source: str) -> GrammarLexer:
    """Create a grammar-driven lexer configured for SWI-Prolog."""

    return GrammarLexer(source, _swi_token_grammar())


def tokenize_swi_prolog(source: str) -> list[Token]:
    """Tokenize SWI-Prolog source code."""

    return create_swi_prolog_lexer(source).tokenize()
