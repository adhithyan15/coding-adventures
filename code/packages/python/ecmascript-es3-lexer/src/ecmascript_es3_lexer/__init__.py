"""ECMAScript 3 (1999) Lexer — tokenizes ES3 JavaScript source code.

Public API:

- ``create_es3_lexer(source)`` — creates a ``GrammarLexer`` for ES3.
- ``tokenize_es3(source)`` — tokenizes source, returns list of ``Token`` objects.
"""

from __future__ import annotations

from ecmascript_es3_lexer.tokenizer import create_es3_lexer, tokenize_es3

__all__ = [
    "create_es3_lexer",
    "tokenize_es3",
]
