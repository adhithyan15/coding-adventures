"""TypeScript 3.0 (2018) Lexer — tokenizes TypeScript 3.0 source code.

Public API:

- ``create_ts30_lexer(source)`` — creates a ``GrammarLexer`` for TypeScript 3.0.
- ``tokenize_ts30(source)`` — tokenizes source, returns list of ``Token`` objects.
"""

from __future__ import annotations

from typescript_ts30_lexer.tokenizer import create_ts30_lexer, tokenize_ts30

__all__ = [
    "create_ts30_lexer",
    "tokenize_ts30",
]
