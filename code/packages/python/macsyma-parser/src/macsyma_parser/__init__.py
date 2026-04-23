"""MACSYMA Parser — parses MACSYMA tokens into a generic AST.

A thin wrapper around the repo's grammar-driven ``GrammarParser``. It
loads ``macsyma.grammar``, tokenizes via ``macsyma-lexer``, and produces
an ``ASTNode`` tree — the same generic AST type used by every other
language parser in the repo.

The resulting AST is consumed by ``macsyma-compiler`` which converts it
to the universal symbolic IR.

Usage::

    from macsyma_parser import parse_macsyma

    ast = parse_macsyma("f(x) := x^2; diff(f(x), x);")
    print(ast.rule_name)  # "program"
"""

from macsyma_parser.parser import create_macsyma_parser, parse_macsyma

__all__ = [
    "create_macsyma_parser",
    "parse_macsyma",
]
