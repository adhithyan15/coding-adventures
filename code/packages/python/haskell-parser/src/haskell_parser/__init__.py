"""Haskell Parser — parses Haskell source code into ASTs using the grammar-driven approach.

This package demonstrates the power of the grammar-driven parser: by simply
providing a different ``.grammar`` file, the same parser engine that parses
Python or HaskellScript can parse Haskell. No new parser code needed — just a new
grammar.

Usage::

    from haskell_parser import parse_haskell

    ast = parse_haskell('public class Hello { }')
    print(ast.rule_name)  # "program"
"""

from haskell_parser.parser import create_haskell_parser, parse_haskell

__all__ = [
    "create_haskell_parser",
    "parse_haskell",
]
