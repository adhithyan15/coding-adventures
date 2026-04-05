"""TypeScript 5.0 (2023) Lexer — tokenizes TypeScript 5.0 source code.

Public API:

- ``create_ts50_lexer(source)`` — creates a ``GrammarLexer`` for TypeScript 5.0.
- ``tokenize_ts50(source)`` — tokenizes source, returns list of ``Token`` objects.
"""

from __future__ import annotations

from typescript_ts50_lexer.tokenizer import create_ts50_lexer, tokenize_ts50

__all__ = [
    "create_ts50_lexer",
    "tokenize_ts50",
]
