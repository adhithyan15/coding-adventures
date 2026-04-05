"""TypeScript 4.0 (2020) Lexer — tokenizes TypeScript 4.0 source code.

Public API:

- ``create_ts40_lexer(source)`` — creates a ``GrammarLexer`` for TypeScript 4.0.
- ``tokenize_ts40(source)`` — tokenizes source, returns list of ``Token`` objects.
"""

from __future__ import annotations

from typescript_ts40_lexer.tokenizer import create_ts40_lexer, tokenize_ts40

__all__ = [
    "create_ts40_lexer",
    "tokenize_ts40",
]
