"""TOML Parser — parses TOML text into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It
tokenizes the input using the TOML lexer, reads the EBNF rules from
``toml.grammar``, and produces a generic ``ASTNode`` tree.

TOML's grammar has 11 rules — more than JSON (4 rules) but fewer than CSS
(36 rules). The full grammar::

    document           = { NEWLINE | expression } ;
    expression         = array_table_header | table_header | keyval ;
    keyval             = key EQUALS value ;
    key                = simple_key { DOT simple_key } ;
    simple_key         = BARE_KEY | BASIC_STRING | LITERAL_STRING
                       | TRUE | FALSE | INTEGER | FLOAT
                       | OFFSET_DATETIME | LOCAL_DATETIME | LOCAL_DATE
                       | LOCAL_TIME ;
    table_header       = LBRACKET key RBRACKET ;
    array_table_header = LBRACKET LBRACKET key RBRACKET RBRACKET ;
    value              = BASIC_STRING | ML_BASIC_STRING | LITERAL_STRING
                       | ML_LITERAL_STRING
                       | INTEGER | FLOAT | TRUE | FALSE
                       | OFFSET_DATETIME | LOCAL_DATETIME | LOCAL_DATE
                       | LOCAL_TIME
                       | array | inline_table ;
    array              = LBRACKET array_values RBRACKET ;
    array_values       = { NEWLINE }
                         [ value { NEWLINE }
                           { COMMA { NEWLINE } value { NEWLINE } }
                           [ COMMA ]
                           { NEWLINE } ] ;
    inline_table       = LBRACE [ keyval { COMMA keyval } ] RBRACE ;

Two-Phase Parsing
-----------------

The AST this module produces is *syntactic* — it captures the structure of the
TOML source but does not enforce semantic constraints. TOML has several rules
that are context-sensitive and cannot be expressed in a context-free grammar:

1. **Key uniqueness** — the same key cannot be defined twice in the same table.
2. **Table path consistency** — ``[a.b]`` cannot appear if ``a.b`` is already
   a value (not a table).
3. **Inline table immutability** — once defined, no keys can be added to an
   inline table.
4. **Array-of-tables consistency** — ``[[a]]`` and ``[a]`` cannot coexist
   for the same path.

These are enforced in the **converter** (``converter.py``), which walks the
AST and builds a ``TOMLDocument`` (a Python dict). This two-phase approach
(parse → validate + convert) keeps the grammar clean and matches how
real-world TOML parsers work.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_toml_parser(source)`` — tokenizes the source with the TOML lexer
  and creates a ``GrammarParser`` configured with the TOML grammar.
- ``parse_toml_ast(source)`` — the all-in-one function. Pass in TOML text,
  get back an AST.

Grammar Source
--------------

The ``toml.grammar`` file lives in ``code/grammars/toml/`` at the
repository root, but this package does not read it at runtime. It is
pre-compiled into ``toml_parser._grammar`` (see that module's header for
regeneration instructions), so parsing TOML requires no file I/O.
"""

from __future__ import annotations

from lang_parser import ASTNode, GrammarParser
from toml_lexer import tokenize_toml

from toml_parser._grammar import PARSER_GRAMMAR

# ---------------------------------------------------------------------------
# Grammar Source
# ---------------------------------------------------------------------------
#
# The TOML parser grammar (``code/grammars/toml/toml.grammar``) is
# pre-compiled into ``toml_parser._grammar`` — see that module's header for
# regeneration instructions.
# ---------------------------------------------------------------------------


def create_toml_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for TOML text.

    This function:

    1. Tokenizes the source text using the TOML lexer.
    2. Creates a ``GrammarParser`` with those tokens and the pre-compiled
       TOML parser grammar.

    Args:
        source: The TOML text to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST.

    Raises:
        LexerError: If the source contains invalid characters.

    Example::

        parser = create_toml_parser('name = "TOML"')
        ast = parser.parse()
    """
    tokens = tokenize_toml(source)
    return GrammarParser(tokens, PARSER_GRAMMAR)


def parse_toml_ast(source: str) -> ASTNode:
    """Parse TOML text and return an AST.

    This is the syntax-only entry point. It returns a generic ``ASTNode``
    tree that captures the structure of the TOML source, but does not
    enforce semantic constraints like key uniqueness or table consistency.

    For most use cases, prefer ``parse_toml()`` from the package's
    ``__init__`` module — it parses AND validates, returning a Python dict.

    The returned AST has the following structure:

    - The root node has ``rule_name="document"``.
    - Children are NEWLINE tokens and ``expression`` nodes.
    - Each ``expression`` is a ``keyval``, ``table_header``, or
      ``array_table_header``.

    Args:
        source: The TOML text to parse.

    Returns:
        An ``ASTNode`` representing the parse tree.

    Raises:
        LexerError: If the source contains invalid characters.
        GrammarParseError: If the source has syntax errors.

    Example::

        ast = parse_toml_ast('name = "TOML"')
        # ASTNode(rule_name="document", children=[
        #     ASTNode(rule_name="expression", children=[
        #         ASTNode(rule_name="keyval", children=[...])
        #     ])
        # ])
    """
    parser = create_toml_parser(source)
    return parser.parse()
