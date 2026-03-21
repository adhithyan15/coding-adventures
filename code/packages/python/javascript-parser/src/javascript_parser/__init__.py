"""JavaScript Parser — parses JavaScript source code into ASTs using the grammar-driven approach.

This package demonstrates the power of the grammar-driven parser: by simply
providing a different ``.grammar`` file, the same parser engine that parses
Python can parse JavaScript. No new parser code needed — just a new grammar.

Usage::

    from javascript_parser import parse_javascript

    ast = parse_javascript('let x = 1 + 2;')
    print(ast.rule_name)  # "program"
"""

from javascript_parser.parser import create_javascript_parser, parse_javascript

__all__ = [
    "create_javascript_parser",
    "parse_javascript",
]
