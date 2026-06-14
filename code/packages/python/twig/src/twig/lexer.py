"""Twig lexer — thin wrapper around the generic ``GrammarLexer``.

The Twig token grammar lives in ``code/grammars/twig.tokens``; this
module just locates the file, hands it to ``parse_token_grammar``,
and constructs a ``GrammarLexer`` over the resulting
``TokenGrammar``.

Mirrors the pattern already used by every other language in the
repo (Brainfuck, Dartmouth BASIC, ALGOL, Prolog…) — a single
source-of-truth grammar file feeds every implementation, and the
language-specific package is the thin shim that loads it.
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

# ---------------------------------------------------------------------------
# Grammar file location
# ---------------------------------------------------------------------------
#
# Walk up from this module to ``code/``, then into ``grammars/``:
#
#   src/twig/lexer.py
#   ├─ src/twig/        (parent)
#   ├─ src/             (parent)
#   ├─ twig/            (parent — package dir)
#   ├─ python/          (parent)
#   ├─ packages/        (parent)
#   └─ code/            (parent) → ``grammars/twig.tokens``

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
TWIG_TOKENS_PATH = GRAMMAR_DIR / "twig.tokens"


def create_twig_lexer(source: str) -> GrammarLexer:
    """Build a ``GrammarLexer`` configured for Twig source.

    Reads ``twig.tokens`` from the grammars directory and returns a
    lexer ready to call ``.tokenize()``.  The resulting token stream
    contains ``LPAREN`` / ``RPAREN`` / ``QUOTE`` / ``BOOL_TRUE`` /
    ``BOOL_FALSE`` / ``INTEGER`` / ``KEYWORD`` / ``NAME`` tokens, with
    whitespace and ``;`` comments already discarded.
    """
    grammar = parse_token_grammar(TWIG_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_twig(source: str) -> list[Token]:
    """Tokenise Twig source text into a flat list of ``Token``.

    The terminating ``EOF`` token is included by the GrammarLexer.
    Position tracking (``line`` / ``column``) on each token enables
    LSP-style error messages in the parser and compiler.
    """
    return create_twig_lexer(source).tokenize()
