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

Locating the Grammar File
--------------------------

The ``json.grammar`` file lives in ``code/grammars/`` at the repository
root. We locate it relative to this module's file path::

    parser.py
    └── json_parser/       (parent)
        └── src/           (parent)
            └── json-parser/ (parent)
                └── python/    (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── json.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from json_lexer import tokenize_json
from lang_parser import ASTNode, GrammarParser

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
JSON_GRAMMAR_PATH = GRAMMAR_DIR / "json.grammar"


def create_json_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for JSON text.

    This function:

    1. Tokenizes the source text using the JSON lexer.
    2. Reads and parses the ``json.grammar`` file.
    3. Creates a ``GrammarParser`` with those tokens and grammar.

    Args:
        source: The JSON text to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains invalid characters.

    Example::

        parser = create_json_parser('{"key": "value"}')
        ast = parser.parse()
    """
    tokens = tokenize_json(source)
    grammar = parse_parser_grammar(JSON_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


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
        FileNotFoundError: If the grammar files cannot be found.
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
