"""C# Parser — parses C# source code into ASTs using the grammar-driven approach.

This package demonstrates the power of the grammar-driven parser: by simply
providing a different ``.grammar`` file, the same parser engine that parses
Python, JavaScript, or Java can parse C#. No new parser code needed — just
a new grammar.

How It Works
------------

The C# parser is a **thin wrapper** around the generic ``GrammarParser``
from the ``lang_parser`` package. It does four things:

1. Tokenizes the source using ``csharp_lexer.tokenize_csharp()``.
2. Locates the ``csharp{version}.grammar`` file in ``grammars/csharp/``.
3. Parses that file into a ``ParserGrammar`` using ``grammar_tools``.
4. Feeds the tokens and grammar to ``GrammarParser``, which builds the AST.

The parser supports all twelve C# versions from 1.0 (2002) through 12.0
(2023).

Usage::

    from csharp_parser import parse_csharp

    ast = parse_csharp('public class Hello { }')
    print(ast.rule_name)  # "program"

    # Using a specific version
    ast = parse_csharp('record Point(int X, int Y);', '9.0')
"""

from csharp_parser.parser import create_csharp_parser, parse_csharp

__all__ = [
    "create_csharp_parser",
    "parse_csharp",
]
