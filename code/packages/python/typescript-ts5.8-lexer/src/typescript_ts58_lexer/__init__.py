"""TypeScript 5.8 (2025) Lexer — tokenizes TypeScript 5.8 source code.

Public API:

- ``create_ts58_lexer(source)`` — creates a ``GrammarLexer`` for TypeScript 5.8.
- ``tokenize_ts58(source)`` — tokenizes source, returns list of ``Token`` objects.
"""

from __future__ import annotations

from typescript_ts58_lexer.tokenizer import create_ts58_lexer, tokenize_ts58

__all__ = [
    "create_ts58_lexer",
    "tokenize_ts58",
]
