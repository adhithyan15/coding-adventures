"""TypeScript 2.0 (September 2016) Lexer — tokenizes TypeScript 2.0 source code.

TypeScript 2.0 was released in September 2016. It upgraded the JavaScript
baseline from ECMAScript 5 to ECMAScript 2015, adding non-nullable types,
the ``never`` type, strict null checks, and tagged template types.

Public API:

- ``create_ts20_lexer(source)`` — creates a ``GrammarLexer`` for TS 2.0.
- ``tokenize_ts20(source)`` — tokenizes source, returns list of ``Token`` objects.
"""

from __future__ import annotations

from typescript_ts20_lexer.tokenizer import create_ts20_lexer, tokenize_ts20

__all__ = [
    "create_ts20_lexer",
    "tokenize_ts20",
]
