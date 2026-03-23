"""TypeScript Parser — parses TypeScript source code into ASTs using the grammar-driven approach.

This package demonstrates the power of the grammar-driven parser: by simply
providing a different ``.grammar`` file, the same parser engine that parses
Python and JavaScript can parse TypeScript. No new parser code needed — just
a new grammar.

Usage::

    from typescript_parser import parse_typescript

    ast = parse_typescript('let x = 1 + 2;')
    print(ast.rule_name)  # "program"
"""

from typescript_parser.parser import create_typescript_parser, parse_typescript

__all__ = [
    "create_typescript_parser",
    "parse_typescript",
]
