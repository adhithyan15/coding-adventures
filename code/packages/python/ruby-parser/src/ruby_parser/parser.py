"""Ruby Parser — parses Ruby source code into ASTs using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It
demonstrates the same core idea as the Ruby lexer: the *same* parser
engine that handles Python can handle Ruby — just swap the ``.grammar`` file.

How the Grammar-Driven Parser Works (Brief Recap)
--------------------------------------------------

The ``GrammarParser`` interprets EBNF grammar rules at runtime. For each
rule, it tries to match the token stream against the rule's body using
a recursive descent approach with backtracking:

- **Sequences** (``A B C``) must match all elements in order.
- **Alternations** (``A | B``) try each choice; first match wins.
- **Repetitions** (``{ A }``) match zero or more times.
- **Optionals** (``[ A ]``) match zero or one time.
- **Token references** (``NUMBER``, ``NAME``) match tokens by type.
- **Literals** (``"puts"``) match tokens by exact text value.
- **Rule references** (``expression``) recursively parse another rule.

The parser produces a tree of generic ``ASTNode`` objects, where each
node records which grammar rule produced it and what children it matched.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_ruby_parser(source)`` — tokenizes the source with
  ``ruby_lexer`` and creates a ``GrammarParser`` configured with the
  Ruby grammar. Use this when you want to control the parse process.

- ``parse_ruby(source)`` — the all-in-one function. Pass in Ruby source
  code, get back an AST. This is the function most callers want.

Locating the Grammar File
--------------------------

The ``ruby.grammar`` file lives in ``code/grammars/`` at the repository
root. We locate it relative to this module's file path, similar to how
the Ruby lexer locates ``ruby.tokens``.
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from ruby_lexer import tokenize_ruby

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# We navigate from this file's location up to the repository root's
# grammars/ directory. The path traversal is:
#   src/ruby_parser/parser.py -> src/ruby_parser -> src -> ruby-parser
#   -> python -> packages -> code -> code/grammars
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
RUBY_GRAMMAR_PATH = GRAMMAR_DIR / "ruby.grammar"


def create_ruby_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for Ruby source code.

    This function:

    1. Tokenizes the source code using the Ruby lexer.
    2. Reads and parses the ``ruby.grammar`` file.
    3. Creates a ``GrammarParser`` with those tokens and grammar.

    Use this when you want access to the parser object itself — for
    example, to inspect its internal state or integrate with a custom
    pipeline. For most use cases, ``parse_ruby()`` is simpler.

    Args:
        source: The Ruby source code to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains invalid characters.

    Example::

        parser = create_ruby_parser('x = 1 + 2')
        ast = parser.parse()
    """
    tokens = tokenize_ruby(source)
    grammar = parse_parser_grammar(RUBY_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_ruby(source: str) -> ASTNode:
    """Parse Ruby source code and return an AST.

    This is the main entry point for the Ruby parser. Pass in a string
    of Ruby source code, and get back an ``ASTNode`` representing the
    complete parse tree.

    The returned AST has the following structure:

    - The root node has ``rule_name="program"`` and its children are
      the statements in the program.
    - Each statement is an ``ASTNode`` with ``rule_name="statement"``
      whose children show what kind of statement it is (assignment,
      method call, or expression).
    - Expressions follow the standard precedence hierarchy:
      ``expression`` > ``term`` > ``factor``.

    Args:
        source: The Ruby source code to parse.

    Returns:
        An ``ASTNode`` representing the parse tree. The root node's
        ``rule_name`` is ``"program"``.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains invalid characters.
        GrammarParseError: If the source has syntax errors according
            to the Ruby grammar.

    Example::

        ast = parse_ruby('x = 1 + 2')
        # ASTNode(rule_name="program", children=[
        #     ASTNode(rule_name="statement", children=[
        #         ASTNode(rule_name="assignment", children=[
        #             Token(NAME, 'x'), Token(EQUALS, '='),
        #             ASTNode(rule_name="expression", ...)
        #         ])
        #     ])
        # ])
    """
    parser = create_ruby_parser(source)
    return parser.parse()
