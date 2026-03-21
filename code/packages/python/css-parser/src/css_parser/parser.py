"""CSS Parser — parses CSS text into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It loads
the ``css.grammar`` file from the ``code/grammars/`` directory, tokenizes
the input using the CSS lexer, and produces a generic ``ASTNode`` tree.

CSS is the Most Complex Grammar
-------------------------------

CSS (Cascading Style Sheets, Level 3) is the most complex grammar the
infrastructure supports — a deliberate stress test for the parser engine.
Compared to JSON (4 rules) and Starlark (~25 rules), the CSS grammar has
36 rules covering:

- **Selectors**: type, class, ID, attribute, pseudo-class, pseudo-element,
  combinators (child ``>``, sibling ``+``/``~``), nesting (``&``)
- **At-rules**: ``@media``, ``@import``, ``@keyframes``, ``@font-face``,
  ``@charset`` — all sharing a unified ``at_rule`` structure
- **Declaration blocks**: property-value pairs with ``!important`` support
- **Values**: dimensions (``10px``), percentages (``50%``), colors (``#fff``),
  functions (``rgb()``, ``calc()``, ``var()``), custom properties (``--var``)
- **CSS Nesting**: nested rules inside declaration blocks using ``&``

The grammar exercises several parser features:

- **Backtracking**: The ``declaration_or_nested`` rule tries to parse a
  declaration first, and falls back to a qualified rule if that fails.
  Both start with ``IDENT``, so the parser must backtrack.
- **Literal matching**: ``priority = BANG "important"`` matches the IDENT
  token only if its text value is ``"important"``.
- **Deep nesting**: Media queries can contain rule sets which contain
  declarations and nested rules.
- **Repetition with optional separators**: Selector lists use
  ``{ COMMA complex_selector }`` for comma-separated selectors.

Example Parse Tree
------------------

Parsing ``h1 { color: red; }`` produces::

    ASTNode(rule_name="stylesheet", children=[
        ASTNode(rule_name="rule", children=[
            ASTNode(rule_name="qualified_rule", children=[
                ASTNode(rule_name="selector_list", children=[
                    ASTNode(rule_name="complex_selector", children=[
                        ASTNode(rule_name="compound_selector", children=[
                            ASTNode(rule_name="simple_selector", children=[
                                Token(IDENT, 'h1')
                            ])
                        ])
                    ])
                ]),
                ASTNode(rule_name="block", children=[
                    Token(LBRACE, '{'),
                    ASTNode(rule_name="block_contents", children=[
                        ASTNode(rule_name="block_item", children=[
                            ASTNode(rule_name="declaration_or_nested", children=[
                                ASTNode(rule_name="declaration", children=[
                                    ASTNode(rule_name="property", children=[
                                        Token(IDENT, 'color')
                                    ]),
                                    Token(COLON, ':'),
                                    ASTNode(rule_name="value_list", children=[
                                        ASTNode(rule_name="value", children=[
                                            Token(IDENT, 'red')
                                        ])
                                    ]),
                                    Token(SEMICOLON, ';')
                                ])
                            ])
                        ])
                    ]),
                    Token(RBRACE, '}')
                ])
            ])
        ])
    ])

What This Module Provides
-------------------------

Two convenience functions:

- ``create_css_parser(source)`` — tokenizes the source with ``css_lexer``
  and creates a ``GrammarParser`` configured with the CSS grammar.
- ``parse_css(source)`` — the all-in-one function. Pass in CSS text, get
  back an AST.

Locating the Grammar File
--------------------------

The ``css.grammar`` file lives in ``code/grammars/`` at the repository
root. We locate it relative to this module's file path::

    parser.py
    └── css_parser/        (parent)
        └── src/           (parent)
            └── css-parser/  (parent)
                └── python/    (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── css.grammar
"""

from __future__ import annotations

from pathlib import Path

from css_lexer import tokenize_css
from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# Navigate from this file's location up to the repository root's grammars/
# directory. The path is:
#   src/css_parser/parser.py -> src/css_parser -> src -> css-parser
#   -> python -> packages -> code -> code/grammars
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
CSS_GRAMMAR_PATH = GRAMMAR_DIR / "css.grammar"


def create_css_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for CSS text.

    This function:

    1. Tokenizes the source text using the CSS lexer.
    2. Reads and parses the ``css.grammar`` file.
    3. Creates a ``GrammarParser`` with those tokens and grammar.

    Args:
        source: The CSS text to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains invalid characters.

    Example::

        parser = create_css_parser('h1 { color: red; }')
        ast = parser.parse()
    """
    tokens = tokenize_css(source)
    grammar = parse_parser_grammar(CSS_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_css(source: str) -> ASTNode:
    """Parse CSS text and return an AST.

    This is the main entry point for the CSS parser. Pass in a string of
    CSS text, and get back an ``ASTNode`` representing the complete parse
    tree.

    The returned AST has the following structure:

    - The root node has ``rule_name="stylesheet"`` (CSS's start rule).
    - Each child is a ``rule`` node containing either an ``at_rule`` or
      a ``qualified_rule``.
    - Qualified rules have a ``selector_list`` and a ``block``.
    - Blocks contain ``declaration`` nodes and potentially nested rules.

    Args:
        source: The CSS text to parse.

    Returns:
        An ``ASTNode`` representing the parse tree. The root node's
        ``rule_name`` is ``"stylesheet"``.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains invalid characters.
        GrammarParseError: If the source has syntax errors according
            to the CSS grammar.

    Example::

        ast = parse_css('h1 { color: red; }')
        # ASTNode(rule_name="stylesheet", children=[
        #     ASTNode(rule_name="rule", children=[
        #         ASTNode(rule_name="qualified_rule", children=[...])
        #     ])
        # ])
    """
    parser = create_css_parser(source)
    return parser.parse()
