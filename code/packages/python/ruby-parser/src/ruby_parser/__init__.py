"""Ruby Parser — parses Ruby source code into ASTs using the grammar-driven approach.

This package demonstrates the power of the grammar-driven parser: by simply
providing a different ``.grammar`` file, the same parser engine that parses
Python can parse Ruby. No new parser code needed — just a new grammar.

How It Works
------------

The Ruby parser is a **thin wrapper** around the generic ``GrammarParser``
from the ``lang_parser`` package. It does four things:

1. Tokenizes Ruby source code using the ``ruby_lexer`` package.
2. Locates the ``ruby.grammar`` file in the ``grammars/`` directory.
3. Parses that file into a ``ParserGrammar`` using ``grammar_tools``.
4. Feeds the grammar and tokens to ``GrammarParser``, which builds the AST.

The resulting AST uses generic ``ASTNode`` objects — the same type that the
Python grammar-driven parser produces. Each node has a ``rule_name`` (which
grammar rule matched) and ``children`` (the matched tokens and sub-nodes).

Usage::

    from ruby_parser import parse_ruby

    ast = parse_ruby('x = 1 + 2')
    print(ast.rule_name)  # "program"
    print(ast.children)   # [ASTNode(rule_name="statement", ...)]
"""

from ruby_parser.parser import create_ruby_parser, parse_ruby

__all__ = [
    "create_ruby_parser",
    "parse_ruby",
]
