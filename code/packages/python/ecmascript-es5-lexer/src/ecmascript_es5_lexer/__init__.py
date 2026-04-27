"""ECMAScript 5 (2009) Lexer — tokenizes ES5 JavaScript source code.

Public API:

- ``create_es5_lexer(source)`` — creates a ``GrammarLexer`` for ES5.
- ``tokenize_es5(source)`` — tokenizes source, returns list of ``Token`` objects.
"""

from __future__ import annotations

from ecmascript_es5_lexer.tokenizer import create_es5_lexer, tokenize_es5

__all__ = [
    "create_es5_lexer",
    "tokenize_es5",
]
