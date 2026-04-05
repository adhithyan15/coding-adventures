"""mosaic-parser — Parses .mosaic source into an ASTNode tree.

This package is the second stage of the Mosaic compiler pipeline:

    Source text → Lexer → Tokens → **Parser** → ASTNode → Analyzer → IR

The Mosaic parser is a thin wrapper around the generic ``GrammarParser`` from
the ``lang_parser`` package. It provides two convenience functions:

- ``parse(source)`` — tokenizes and parses Mosaic source, returns the AST root.
- ``create_parser(source)`` — returns the ``GrammarParser`` instance for
  advanced use cases.

Mosaic Grammar Overview
-----------------------

A ``.mosaic`` file has this structure::

    import Button from "./button.mosaic";

    component ProfileCard {
        slot avatar-url: image;
        slot display-name: text;
        slot items: list<text>;

        Column {
            Image { source: @avatar-url; }
            Text  { content: @display-name; font-size: 18sp; }
            each @items as item {
                Text { content: @item; }
            }
        }
    }

Grammar Rules
-------------

- ``file`` — top-level: imports + one component
- ``import_decl`` — ``import X [as Y] from "path";``
- ``component_decl`` — ``component Name { slots... tree }``
- ``slot_decl`` — ``slot name: type [= default];``
- ``slot_type`` — ``list_type | KEYWORD | NAME``
- ``list_type`` — ``list<slot_type>``
- ``default_value`` — literal value for optional slots
- ``node_tree`` — root node element
- ``node_element`` — ``Name { contents... }``
- ``node_content`` — property, child node, slot ref, when/each block
- ``property_assignment`` — ``name: value;``
- ``property_value`` — slot ref, string, number, dimension, color, enum, name
- ``slot_ref`` — ``@name``
- ``enum_value`` — ``namespace.member``
- ``slot_reference`` — ``@name;`` (child slot reference)
- ``when_block`` — ``when @flag { ... }``
- ``each_block`` — ``each @list as item { ... }``

Usage::

    from mosaic_parser import parse

    ast = parse('''
        component Label {
            slot text: text;
            Text { content: @text; }
        }
    ''')
    print(ast.rule_name)  # "file"
"""

from __future__ import annotations

from lang_parser import ASTNode, GrammarParser
from mosaic_lexer import tokenize as tokenize_mosaic

from mosaic_parser._grammar import PARSER_GRAMMAR

__version__ = "0.1.0"

__all__ = [
    "parse",
    "create_parser",
    "PARSER_GRAMMAR",
]


def create_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for Mosaic source code.

    Tokenizes the source using the Mosaic lexer, then constructs a
    ``GrammarParser`` with the embedded Mosaic grammar.

    Use this when you need direct access to the parser object — for example,
    to inspect parse state or integrate with a custom pipeline.
    For most use cases, ``parse()`` is simpler.

    Args:
        source: The Mosaic source text.

    Returns:
        A ``GrammarParser`` ready to parse Mosaic. Call ``.parse()`` to get
        the AST root.

    Raises:
        LexerError: If the source contains invalid tokens.

    Example::

        parser = create_parser('component Label {}')
        ast = parser.parse()
    """
    tokens = tokenize_mosaic(source)
    return GrammarParser(tokens, PARSER_GRAMMAR)


def parse(source: str) -> ASTNode:
    """Parse Mosaic source code and return the AST root.

    This is the main entry point for the Mosaic parser. Pass in a string of
    Mosaic source code and get back an ``ASTNode`` with ``rule_name="file"``.

    The AST faithfully mirrors the grammar: every keyword, brace, semicolon,
    and slot-type token appears as a child of the appropriate rule node.
    The downstream analyzer strips the syntax noise and produces a typed IR.

    Args:
        source: The Mosaic source text.

    Returns:
        An ``ASTNode`` with ``rule_name="file"`` representing the complete
        parse tree.

    Raises:
        LexerError: If the source contains invalid tokens.
        GrammarParseError: If the source does not match the Mosaic grammar.

    Example::

        ast = parse('''
            component Label {
                slot title: text;
                Text { content: @title; }
            }
        ''')
        print(ast.rule_name)  # "file"
    """
    return create_parser(source).parse()
