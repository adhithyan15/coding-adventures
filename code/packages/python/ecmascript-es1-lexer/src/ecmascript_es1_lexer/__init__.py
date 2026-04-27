"""ECMAScript 1 (1997) Lexer — tokenizes ES1 JavaScript source code.

Public API:

- ``create_es1_lexer(source)`` — creates a ``GrammarLexer`` for ES1.
- ``tokenize_es1(source)`` — tokenizes source, returns list of ``Token`` objects.
"""

from __future__ import annotations

from ecmascript_es1_lexer.tokenizer import create_es1_lexer, tokenize_es1

__all__ = [
    "create_es1_lexer",
    "tokenize_es1",
]
