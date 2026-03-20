"""JSON Parser — parses JSON text into ASTs using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarParser``. It
tokenizes JSON using the ``json-lexer`` package, then parses the token stream
using the EBNF rules defined in ``json.grammar``.

The result is a generic ``ASTNode`` tree — the same type used for Python, Ruby,
JavaScript, and Starlark. This demonstrates the language-agnostic nature of the
grammar-driven approach.

Usage::

    from json_parser import parse_json

    ast = parse_json('{"name": "Ada", "age": 36}')
    print(ast.rule_name)  # "value"
"""

from json_parser.parser import create_json_parser, parse_json

__all__ = [
    "create_json_parser",
    "parse_json",
]
