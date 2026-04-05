"""TypeScript 1.0 (April 2014) Lexer — tokenizes TypeScript 1.0 source code.

TypeScript 1.0 was the first public release of TypeScript, announced at
Microsoft's Build conference in April 2014. It added a static type system
to JavaScript, building on ECMAScript 5 as its foundation.

Public API:

- ``create_ts10_lexer(source)`` — creates a ``GrammarLexer`` for TS 1.0.
- ``tokenize_ts10(source)`` — tokenizes source, returns list of ``Token`` objects.
"""

from __future__ import annotations

from typescript_ts10_lexer.tokenizer import create_ts10_lexer, tokenize_ts10

__all__ = [
    "create_ts10_lexer",
    "tokenize_ts10",
]
