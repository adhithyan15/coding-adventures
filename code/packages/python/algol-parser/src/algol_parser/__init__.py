"""ALGOL 60 Parser — parses ALGOL 60 source into ASTs using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarParser``. It
tokenizes ALGOL 60 source using the ``algol-lexer`` package, then parses the
token stream using the compiled EBNF rules from ``algol/algol60.grammar``.

ALGOL 60 (1960) was the first programming language whose grammar was formally
specified in BNF (Backus-Naur Form). It introduced block structure, lexical
scoping, recursion, the call stack, and free-format source layout — the
foundations of every modern programming language.

The result of parsing is a generic ``ASTNode`` tree — the same type used for
JSON, Python, Ruby, and JavaScript. This demonstrates the language-agnostic
nature of the grammar-driven approach: the same parser engine that handles
modern languages can also handle a language from 1960.

Usage::

    from algol_parser import parse_algol

    ast = parse_algol('begin integer x; x := 42 end')
    print(ast.rule_name)  # "program"
"""

from algol_parser.parser import (
    DEFAULT_VERSION,
    SUPPORTED_VERSIONS,
    create_algol_parser,
    parse_algol,
    resolve_version,
)

__all__ = [
    "DEFAULT_VERSION",
    "SUPPORTED_VERSIONS",
    "create_algol_parser",
    "parse_algol",
    "resolve_version",
]
