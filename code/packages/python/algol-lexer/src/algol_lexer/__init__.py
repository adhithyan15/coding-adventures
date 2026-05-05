"""ALGOL 60 Lexer — tokenizes ALGOL 60 source using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarLexer``. It
demonstrates the grammar-driven architecture applied to ALGOL 60 — the
language that introduced formal grammar specification (BNF), block structure,
lexical scoping, and recursion to the programming world.

ALGOL 60 (ALGOrithmic Language, 1960) was designed by an international
committee chaired by Peter Naur. It is the common ancestor of Pascal, C,
Simula (the first OOP language), and through them virtually every modern
programming language. Its grammar was the first ever written in BNF notation.

This package imports the compiled ``algol/algol60.tokens`` grammar as native
Python data and uses the same ``GrammarLexer`` engine that powers the JSON,
Python, Ruby, JavaScript, and Starlark lexers. This demonstrates that one lexer
engine can handle radically different languages — including a language from
1960.

Usage::

    from algol_lexer import tokenize_algol

    tokens = tokenize_algol('begin integer x; x := 42 end')
    for token in tokens:
        print(token)
"""

from algol_lexer.tokenizer import (
    DEFAULT_VERSION,
    SUPPORTED_VERSIONS,
    create_algol_lexer,
    resolve_version,
    tokenize_algol,
)

__all__ = [
    "DEFAULT_VERSION",
    "SUPPORTED_VERSIONS",
    "create_algol_lexer",
    "resolve_version",
    "tokenize_algol",
]
