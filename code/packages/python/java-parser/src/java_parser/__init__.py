"""Java Parser — parses Java source code into ASTs using the grammar-driven approach.

This package demonstrates the power of the grammar-driven parser: by simply
providing a different ``.grammar`` file, the same parser engine that parses
Python or JavaScript can parse Java. No new parser code needed — just a new
grammar.

Usage::

    from java_parser import parse_java

    ast = parse_java('public class Hello { }')
    print(ast.rule_name)  # "program"
"""

from java_parser.parser import create_java_parser, parse_java

__all__ = [
    "create_java_parser",
    "parse_java",
]
