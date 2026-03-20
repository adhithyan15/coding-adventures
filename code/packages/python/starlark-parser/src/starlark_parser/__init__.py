"""Starlark Parser — parses Starlark source code into ASTs using the grammar-driven approach.

This package demonstrates the power of the grammar-driven parser: by simply
providing a different ``.grammar`` file, the same parser engine that parses
Python can parse Starlark. No new parser code needed — just a new grammar.

How It Works
------------

The Starlark parser is a **thin wrapper** around the generic ``GrammarParser``
from the ``lang_parser`` package. It does four things:

1. Tokenizes Starlark source code using the ``starlark_lexer`` package.
2. Locates the ``starlark.grammar`` file in the ``grammars/`` directory.
3. Parses that file into a ``ParserGrammar`` using ``grammar_tools``.
4. Feeds the grammar and tokens to ``GrammarParser``, which builds the AST.

The resulting AST uses generic ``ASTNode`` objects — the same type that the
Python and Ruby grammar-driven parsers produce. Each node has a ``rule_name``
(which grammar rule matched) and ``children`` (the matched tokens and
sub-nodes).

Why Starlark Matters
--------------------

Starlark is the configuration language used by Bazel, Buck, and other build
systems. Being able to parse Starlark means being able to read, analyze,
and transform BUILD files programmatically. The grammar-driven parser makes
this straightforward: define the grammar in ``starlark.grammar``, and the
generic parser engine handles the rest.

Usage::

    from starlark_parser import parse_starlark

    ast = parse_starlark('x = 1 + 2\\n')
    print(ast.rule_name)  # "file"
    print(ast.children)   # [ASTNode(rule_name="statement", ...)]
"""

from starlark_parser.parser import create_starlark_parser, parse_starlark

__all__ = [
    "create_starlark_parser",
    "parse_starlark",
]
