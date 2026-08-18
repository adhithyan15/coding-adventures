"""Twig lexer — thin wrapper around the generic ``GrammarLexer``.

The Twig token grammar is compiled into ``_grammar_tokens.py`` by the
grammar-tools compiler from ``code/grammars/twig/twig.tokens``; this
module imports the pre-built ``TOKEN_GRAMMAR`` constant and constructs a
``GrammarLexer`` over it — no file I/O at startup, no runtime grammar
parsing overhead. (The parser grammar lives in a separate
``_grammar_parser.py`` in this same package — see ``parser.py`` — since
both a lexer and a parser share this ``twig`` package.)

Mirrors the pattern already used by every other language in the
repo (Brainfuck, Dartmouth BASIC, ALGOL, Prolog…) — a single
source-of-truth grammar file feeds every implementation, and the
language-specific package is the thin shim that loads it.

To regenerate after editing ``code/grammars/twig/twig.tokens``::

    grammar-tools compile-tokens code/grammars/twig/twig.tokens \\
        > code/packages/python/twig/src/twig/_grammar_tokens.py
"""

from __future__ import annotations

from lexer import GrammarLexer, Token

from twig._grammar_tokens import TOKEN_GRAMMAR


def create_twig_lexer(source: str) -> GrammarLexer:
    """Build a ``GrammarLexer`` configured for Twig source.

    Uses the pre-compiled ``TOKEN_GRAMMAR`` (from ``twig._grammar_tokens``)
    and returns a lexer ready to call ``.tokenize()``.  The resulting
    token stream contains ``LPAREN`` / ``RPAREN`` / ``QUOTE`` /
    ``BOOL_TRUE`` / ``BOOL_FALSE`` / ``INTEGER`` / ``KEYWORD`` / ``NAME``
    tokens, with whitespace and ``;`` comments already discarded.
    """
    return GrammarLexer(source, TOKEN_GRAMMAR)


def tokenize_twig(source: str) -> list[Token]:
    """Tokenise Twig source text into a flat list of ``Token``.

    The terminating ``EOF`` token is included by the GrammarLexer.
    Position tracking (``line`` / ``column``) on each token enables
    LSP-style error messages in the parser and compiler.
    """
    return create_twig_lexer(source).tokenize()
