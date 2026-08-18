"""JSON Parser — parses JSON text into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It loads
the ``json.grammar`` file from the ``code/grammars/`` directory, tokenizes
the input using the JSON lexer, and produces a generic ``ASTNode`` tree.

JSON (RFC 8259) is the simplest grammar the infrastructure supports. The
entire grammar is just four rules::

    value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;
    object = LBRACE [ pair { COMMA pair } ] RBRACE ;
    pair   = STRING COLON value ;
    array  = LBRACKET [ value { COMMA value } ] RBRACKET ;

The parser produces a tree of ``ASTNode`` objects where each node records
which grammar rule produced it and what children it matched. For example,
parsing ``{"a": 1}`` produces::

    ASTNode(rule_name="value", children=[
        ASTNode(rule_name="object", children=[
            Token(LBRACE, '{'),
            ASTNode(rule_name="pair", children=[
                Token(STRING, 'a'),
                Token(COLON, ':'),
                ASTNode(rule_name="value", children=[
                    Token(NUMBER, '1')
                ])
            ]),
            Token(RBRACE, '}')
        ])
    ])

What This Module Provides
-------------------------

Two convenience functions:

- ``create_json_parser(source)`` — tokenizes the source with ``json_lexer``
  and creates a ``GrammarParser`` configured with the JSON grammar.
- ``parse_json(source)`` — the all-in-one function. Pass in JSON text, get
  back an AST.

Pre-compiled Grammar
---------------------

The ``json.grammar`` file is compiled ahead of time by the
``grammar-tools`` compiler into ``json_parser/_grammar.py``, which embeds
the ``ParserGrammar`` as native Python data structures. This module
imports ``PARSER_GRAMMAR`` from it directly — no file I/O or grammar
parsing happens at runtime, and the package works correctly when
installed as a site-package (it does not depend on the
``code/grammars/`` directory existing on disk).

To regenerate after editing ``code/grammars/json/json.grammar``:

    grammar-tools compile-grammar code/grammars/json/json.grammar \\
        -o code/packages/python/json-parser/src/json_parser/_grammar.py
"""

from __future__ import annotations

from json_lexer import tokenize_json
from lang_parser import ASTNode, GrammarParser

from json_parser._grammar import PARSER_GRAMMAR


def create_json_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for JSON text.

    This function:

    1. Tokenizes the source text using the JSON lexer.
    2. Looks up the pre-compiled ``PARSER_GRAMMAR``.
    3. Creates a ``GrammarParser`` with those tokens and grammar.

    Args:
        source: The JSON text to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST.

    Raises:
        LexerError: If the source contains invalid characters.

    Example::

        parser = create_json_parser('{"key": "value"}')
        ast = parser.parse()
    """
    tokens = tokenize_json(source)
    return GrammarParser(tokens, PARSER_GRAMMAR)


def parse_json(source: str) -> ASTNode:
    """Parse JSON text and return an AST.

    This is the main entry point for the JSON parser. Pass in a string of
    JSON text, and get back an ``ASTNode`` representing the complete parse
    tree.

    The returned AST has the following structure:

    - The root node has ``rule_name="value"`` (JSON's start rule).
    - If the value is an object, the root's only child is an
      ``ASTNode(rule_name="object", ...)``.
    - If the value is an array, the root's only child is an
      ``ASTNode(rule_name="array", ...)``.
    - Primitive values (STRING, NUMBER, TRUE, FALSE, NULL) appear as
      ``Token`` objects directly in the children list.

    Args:
        source: The JSON text to parse.

    Returns:
        An ``ASTNode`` representing the parse tree. The root node's
        ``rule_name`` is ``"value"``.

    Raises:
        LexerError: If the source contains invalid characters.
        GrammarParseError: If the source has syntax errors according
            to the JSON grammar.

    Example::

        ast = parse_json('[1, 2, 3]')
        # ASTNode(rule_name="value", children=[
        #     ASTNode(rule_name="array", children=[
        #         Token(LBRACKET, '['),
        #         Token(NUMBER, '1'), Token(COMMA, ','),
        #         Token(NUMBER, '2'), Token(COMMA, ','),
        #         Token(NUMBER, '3'),
        #         Token(RBRACKET, ']')
        #     ])
        # ])
    """
    parser = create_json_parser(source)
    return parser.parse()
