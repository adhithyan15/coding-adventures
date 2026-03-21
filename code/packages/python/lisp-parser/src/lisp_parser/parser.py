"""Lisp Parser — parses Lisp text into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It loads
the ``lisp.grammar`` file from the ``code/grammars/`` directory, tokenizes
the input using the Lisp lexer, and produces a generic ``ASTNode`` tree.

Lisp's Grammar is Simple
-------------------------

Lisp has one of the simplest grammars of any programming language — just
6 rules. Everything is either an atom (number, symbol, string) or a list
(parenthesized sequence of s-expressions). This simplicity is the genius
of Lisp: the syntax is so uniform that code and data have the same structure.

Compare to CSS (36 rules) or Starlark (~25 rules). Lisp's grammar is
a refreshing exercise in minimalism.

The 6 Rules
-----------

::

    program   = { sexpr } ;          # A program is zero or more s-expressions
    sexpr     = atom | list | quoted ;   # An s-expression is an atom, list, or quoted form
    atom      = NUMBER | SYMBOL | STRING ;   # Atoms are terminal values
    list      = LPAREN list_body RPAREN ;    # Lists are parenthesized
    list_body = [ sexpr { sexpr } [ DOT sexpr ] ] ;  # List contents (may be dotted pair)
    quoted    = QUOTE sexpr ;            # 'x is sugar for (quote x)

Example Parse Tree
------------------

Parsing ``(define x 42)`` produces::

    ASTNode(rule_name="program", children=[
        ASTNode(rule_name="sexpr", children=[
            ASTNode(rule_name="list", children=[
                Token(LPAREN, '('),
                ASTNode(rule_name="list_body", children=[
                    ASTNode(rule_name="sexpr", children=[
                        ASTNode(rule_name="atom", children=[Token(SYMBOL, 'define')])
                    ]),
                    ASTNode(rule_name="sexpr", children=[
                        ASTNode(rule_name="atom", children=[Token(SYMBOL, 'x')])
                    ]),
                    ASTNode(rule_name="sexpr", children=[
                        ASTNode(rule_name="atom", children=[Token(NUMBER, '42')])
                    ]),
                ]),
                Token(RPAREN, ')')
            ])
        ])
    ])

Locating the Grammar File
--------------------------

The ``lisp.grammar`` file lives in ``code/grammars/`` at the repository
root. We locate it relative to this module's file path::

    parser.py
    └── lisp_parser/       (parent)
        └── src/           (parent)
            └── lisp-parser/ (parent)
                └── python/    (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── lisp.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from lisp_lexer import tokenize_lisp

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# Navigate from this file's location up to the repository root's grammars/
# directory. The path is:
#   src/lisp_parser/parser.py -> src/lisp_parser -> src -> lisp-parser
#   -> python -> packages -> code -> code/grammars
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
LISP_GRAMMAR_PATH = GRAMMAR_DIR / "lisp.grammar"


def create_lisp_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for Lisp text.

    This function:

    1. Tokenizes the source text using the Lisp lexer.
    2. Reads and parses the ``lisp.grammar`` file.
    3. Creates a ``GrammarParser`` with those tokens and grammar.

    Args:
        source: The Lisp text to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains invalid characters.

    Example::

        parser = create_lisp_parser('(+ 1 2)')
        ast = parser.parse()
    """
    tokens = tokenize_lisp(source)
    grammar = parse_parser_grammar(LISP_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_lisp(source: str) -> ASTNode:
    """Parse Lisp text and return an AST.

    This is the main entry point for the Lisp parser. Pass in a string of
    Lisp text, and get back an ``ASTNode`` representing the complete parse
    tree.

    The returned AST has the following structure:

    - The root node has ``rule_name="program"`` (Lisp's start rule).
    - Each child is a ``sexpr`` node.
    - ``sexpr`` nodes contain either ``atom``, ``list``, or ``quoted``.
    - ``list`` nodes contain ``LPAREN``, ``list_body``, and ``RPAREN``.
    - ``atom`` nodes contain a single ``NUMBER``, ``SYMBOL``, or ``STRING``.

    Args:
        source: The Lisp text to parse.

    Returns:
        An ``ASTNode`` representing the parse tree.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains invalid characters.
        GrammarParseError: If the source has syntax errors.

    Example::

        ast = parse_lisp('(+ 1 2)')
        assert ast.rule_name == "program"
    """
    parser = create_lisp_parser(source)
    return parser.parse()
