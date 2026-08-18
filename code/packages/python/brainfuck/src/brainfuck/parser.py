"""Brainfuck Parser — parses Brainfuck source into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It uses a
pre-compiled ``PARSER_GRAMMAR`` (see ``brainfuck._grammar_parser``), tokenizes
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

The Brainfuck parser grammar is compiled into ``_grammar_parser.py`` by the
grammar-tools compiler from ``code/grammars/brainfuck/brainfuck.grammar``.
Importing the pre-built ``PARSER_GRAMMAR`` constant means no file I/O at
startup and no runtime grammar parsing overhead. (The token grammar lives
in a separate ``_grammar_tokens.py`` in this same package — see
``lexer.py``.)

To regenerate after editing ``code/grammars/brainfuck/brainfuck.grammar``::

    grammar-tools compile-grammar code/grammars/brainfuck/brainfuck.grammar \\
        > code/packages/python/brainfuck/src/brainfuck/_grammar_parser.py
"""

from __future__ import annotations

from lang_parser import ASTNode, GrammarParser

from brainfuck._grammar_parser import PARSER_GRAMMAR
from brainfuck.lexer import tokenize_brainfuck


def create_brainfuck_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for Brainfuck source text.

    This function:

    1. Tokenizes the source text using the Brainfuck lexer. Comment text
       and whitespace are discarded during tokenization; only command tokens
       and EOF reach the parser.
    2. Uses the pre-compiled ``PARSER_GRAMMAR`` (from
       ``brainfuck._grammar_parser``).
    3. Creates a ``GrammarParser`` with those tokens and grammar.

    Args:
        source: The Brainfuck source text to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST root node.

    Example::

        parser = create_brainfuck_parser("++[>+<-]")
        ast = parser.parse()
        print(ast.rule_name)  # "program"
    """
    tokens = tokenize_brainfuck(source)
    return GrammarParser(tokens, PARSER_GRAMMAR)


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
