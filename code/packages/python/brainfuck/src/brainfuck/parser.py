"""Brainfuck Parser — parses Brainfuck source into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It loads
the ``brainfuck.grammar`` file from the ``code/grammars/`` directory, tokenizes
the input using the Brainfuck lexer, and produces a generic ``ASTNode`` tree.

The Brainfuck Grammar
---------------------

The complete grammar is just four rules::

    program     = { instruction } ;
    instruction = loop | command ;
    loop        = LOOP_START { instruction } LOOP_END ;
    command     = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;

Despite its simplicity, the grammar is **recursive**: ``program`` contains
``instruction``s, ``instruction`` can contain a ``loop``, and ``loop``
contains ``instruction``s again. This mutual recursion allows Brainfuck to
represent arbitrarily deep nested loops.

The AST Structure
-----------------

The parser produces a tree of ``ASTNode`` objects where each node records which
grammar rule produced it and what children it matched. For example, parsing
``++[>+<-]`` produces roughly::

    ASTNode(rule_name="program", children=[
        ASTNode(rule_name="instruction", children=[
            ASTNode(rule_name="command", children=[Token(INC, '+')])
        ]),
        ASTNode(rule_name="instruction", children=[
            ASTNode(rule_name="command", children=[Token(INC, '+')])
        ]),
        ASTNode(rule_name="instruction", children=[
            ASTNode(rule_name="loop", children=[
                Token(LOOP_START, '['),
                ASTNode(rule_name="instruction", children=[
                    ASTNode(rule_name="command", children=[Token(RIGHT, '>')])
                ]),
                ASTNode(rule_name="instruction", children=[
                    ASTNode(rule_name="command", children=[Token(INC, '+')])
                ]),
                ASTNode(rule_name="instruction", children=[
                    ASTNode(rule_name="command", children=[Token(LEFT, '<')])
                ]),
                ASTNode(rule_name="instruction", children=[
                    ASTNode(rule_name="command", children=[Token(DEC, '-')])
                ]),
                Token(LOOP_END, ']')
            ])
        ])
    ])

What This Module Provides
--------------------------

Two convenience functions:

- ``create_brainfuck_parser(source)`` — tokenizes the source with the
  Brainfuck lexer and creates a ``GrammarParser`` configured with the
  Brainfuck grammar.
- ``parse_brainfuck(source)`` — the all-in-one function. Pass in Brainfuck
  source text, get back an AST.

Unmatched Brackets
------------------

If the source contains unmatched brackets (e.g., ``[+`` without a matching
``]`` or ``+]`` without a leading ``[``), the generic parser will raise an
exception. This is caught at parse time — before execution — which is a key
advantage of the grammar-driven approach over direct translation.

Locating the Grammar File
--------------------------

The ``brainfuck.grammar`` file lives in ``code/grammars/`` at the repository
root. We locate it relative to this module's file path::

    parser.py
    └── brainfuck/       (parent)
        └── src/         (parent)
            └── brainfuck/ (parent)
                └── python/  (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── brainfuck.grammar
"""

from __future__ import annotations

from pathlib import Path

from brainfuck.lexer import tokenize_brainfuck
from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# Navigate 6 levels up from this file to reach code/, then into grammars/.
# This mirrors the path navigation in lexer.py.
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
BF_GRAMMAR_PATH = GRAMMAR_DIR / "brainfuck.grammar"


def create_brainfuck_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for Brainfuck source text.

    This function:

    1. Tokenizes the source text using the Brainfuck lexer. Comment text
       and whitespace are discarded during tokenization; only command tokens
       and EOF reach the parser.
    2. Reads and parses the ``brainfuck.grammar`` file.
    3. Creates a ``GrammarParser`` with those tokens and grammar.

    Args:
        source: The Brainfuck source text to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST root node.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.

    Example::

        parser = create_brainfuck_parser("++[>+<-]")
        ast = parser.parse()
        print(ast.rule_name)  # "program"
    """
    tokens = tokenize_brainfuck(source)
    grammar = parse_parser_grammar(BF_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_brainfuck(source: str) -> ASTNode:
    """Parse Brainfuck source text and return an AST.

    This is the main entry point for the Brainfuck parser. Pass in a string
    of Brainfuck source, and get back an ``ASTNode`` representing the complete
    parse tree.

    The returned AST always has ``rule_name="program"`` at the root. An empty
    source (or a source containing only comments) produces a program node with
    no instruction children — an empty program is valid Brainfuck.

    Args:
        source: The Brainfuck source text to parse.

    Returns:
        An ``ASTNode`` representing the parse tree. The root node's
        ``rule_name`` is ``"program"``.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        GrammarParseError: If the source has structural errors (e.g.,
            unmatched brackets).

    Example::

        ast = parse_brainfuck("++[>+<-]")
        print(ast.rule_name)  # "program"

    Example::

        # An empty program is valid:
        ast = parse_brainfuck("")
        print(ast.rule_name)  # "program"

    Example::

        # Comments are stripped automatically:
        ast = parse_brainfuck("+ increment")
        # Equivalent to parse_brainfuck("+")

    Example::

        # Unmatched bracket raises an exception:
        try:
            parse_brainfuck("[+")
        except Exception:
            print("parse error: unmatched [")
    """
    parser = create_brainfuck_parser(source)
    return parser.parse()
