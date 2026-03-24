"""Lattice Parser — parses Lattice source into ASTs.

This module is a thin wrapper around the generic ``GrammarParser``. It loads
the ``lattice.grammar`` file from the ``code/grammars/`` directory, tokenizes
the input using the Lattice lexer, and produces a generic ``ASTNode`` tree.

The AST contains both CSS nodes (``qualified_rule``, ``declaration``,
``selector_list``) and Lattice nodes (``variable_declaration``,
``mixin_definition``, ``if_directive``, etc.). The AST-to-CSS compiler
(separate package) removes Lattice nodes by expanding them into pure CSS.

Locating the Grammar File
--------------------------

Same path strategy as the lexer — navigate from this file up to
``code/grammars/lattice.grammar``::

    parser.py
    └── lattice_parser/      (parent)
        └── src/             (parent)
            └── lattice-parser/  (parent)
                └── python/      (parent)
                    └── packages/ (parent)
                        └── code/ (parent)
                            └── grammars/
                                └── lattice.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from lattice_lexer import tokenize_lattice

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
LATTICE_GRAMMAR_PATH = GRAMMAR_DIR / "lattice.grammar"


def create_lattice_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for Lattice source text.

    This function:

    1. Tokenizes the source text using the Lattice lexer.
    2. Reads and parses the ``lattice.grammar`` file.
    3. Creates a ``GrammarParser`` with those tokens and grammar.

    Args:
        source: The Lattice source text to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains invalid characters.

    Example::

        parser = create_lattice_parser('$color: red;')
        ast = parser.parse()
    """
    tokens = tokenize_lattice(source)
    grammar = parse_parser_grammar(LATTICE_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


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
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains invalid characters.
        GrammarParseError: If the source has syntax errors.

    Example::

        ast = parse_lattice('$color: red; h1 { color: $color; }')
    """
    parser = create_lattice_parser(source)
    return parser.parse()
