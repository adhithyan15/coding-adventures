"""Lattice Parser — parses Lattice source into ASTs.

This module is a thin wrapper around the generic ``GrammarParser``. It loads
the ``lattice.grammar`` file from the ``code/grammars/`` directory, tokenizes
the input using the Lattice lexer, and produces a generic ``ASTNode`` tree.

The AST contains both CSS nodes (``qualified_rule``, ``declaration``,
``selector_list``) and Lattice nodes (``variable_declaration``,
``mixin_definition``, ``if_directive``, etc.). The AST-to-CSS compiler
(separate package) removes Lattice nodes by expanding them into pure CSS.

Pre-compiled Grammar
---------------------

The ``lattice.grammar`` file is compiled ahead of time by the
``grammar-tools`` compiler into ``lattice_parser/_grammar.py``, which
embeds the ``ParserGrammar`` as native Python data structures. This module
imports ``PARSER_GRAMMAR`` from it directly — no file I/O or grammar
parsing happens at runtime, and the package works correctly when
installed as a site-package (it does not depend on the
``code/grammars/`` directory existing on disk).

To regenerate after editing ``code/grammars/lattice/lattice.grammar``:

    grammar-tools compile-grammar code/grammars/lattice/lattice.grammar \\
        -o code/packages/python/lattice-parser/src/lattice_parser/_grammar.py
"""

from __future__ import annotations

from lang_parser import ASTNode, GrammarParser
from lattice_lexer import tokenize_lattice

from lattice_parser._grammar import PARSER_GRAMMAR


def create_lattice_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for Lattice source text.

    This function:

    1. Tokenizes the source text using the Lattice lexer.
    2. Looks up the pre-compiled ``PARSER_GRAMMAR``.
    3. Creates a ``GrammarParser`` with those tokens and grammar.

    Args:
        source: The Lattice source text to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST.

    Raises:
        LexerError: If the source contains invalid characters.

    Example::

        parser = create_lattice_parser('$color: red;')
        ast = parser.parse()
    """
    tokens = tokenize_lattice(source)
    return GrammarParser(tokens, PARSER_GRAMMAR)


def parse_lattice(source: str) -> ASTNode:
    """Parse Lattice source text and return an AST.

    This is the main entry point for the Lattice parser. Pass in a string
    of Lattice source, get back an ``ASTNode`` representing the complete
    parse tree.

    The returned AST has ``rule_name="stylesheet"`` at the root, with
    children that are ``rule`` nodes containing Lattice constructs
    (``variable_declaration``, ``mixin_definition``, etc.) and CSS
    constructs (``qualified_rule``, ``at_rule``).

    Args:
        source: The Lattice source text to parse.

    Returns:
        An ``ASTNode`` representing the parse tree.

    Raises:
        LexerError: If the source contains invalid characters.
        GrammarParseError: If the source has syntax errors.

    Example::

        ast = parse_lattice('$color: red; h1 { color: $color; }')
    """
    parser = create_lattice_parser(source)
    return parser.parse()
