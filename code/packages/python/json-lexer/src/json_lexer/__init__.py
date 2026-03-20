"""JSON Lexer — tokenizes JSON text using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarLexer``. It
demonstrates the grammar-driven architecture at its simplest: the same lexer
engine that tokenizes Python, Ruby, JavaScript, and Starlark can tokenize
JSON — just by loading a different ``.tokens`` file.

JSON is the ideal validation case because it has no keywords, no identifiers,
no operators, no comments, and no indentation. Every token is either a value
literal (STRING, NUMBER, TRUE, FALSE, NULL) or a structural delimiter
({, }, [, ], :, ,). This makes it the smallest possible grammar that is
still practically useful.

Usage::

    from json_lexer import tokenize_json

    tokens = tokenize_json('{"name": "Ada", "age": 36}')
    for token in tokens:
        print(token)
"""

from json_lexer.tokenizer import create_json_lexer, tokenize_json

__all__ = [
    "create_json_lexer",
    "tokenize_json",
]
