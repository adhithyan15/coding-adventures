"""JavaScript Lexer — tokenizes JavaScript source code using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It
demonstrates a core idea of the grammar-driven architecture: the *same*
lexer engine that tokenizes Python can tokenize JavaScript — or any other
language — simply by swapping the ``.tokens`` file.

How the Grammar-Driven Approach Works
--------------------------------------

JavaScript has tokens that Python does not — like ``===`` (strict equality),
``!==`` (strict inequality), ``=>`` (arrow), and delimiters like ``{}``,
``[]``, ``;``, and ``.``. The grammar-driven approach handles all of these
without any new tokenization code: they are declared in the ``.tokens`` file,
and the ``GrammarLexer`` compiles them into regex patterns at runtime.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_javascript_lexer(source)`` — creates a ``GrammarLexer`` configured
  for JavaScript.

- ``tokenize_javascript(source)`` — the all-in-one function. Pass in JavaScript
  source code, get back a list of tokens.

Locating the Grammar File
--------------------------

The ``javascript.tokens`` file lives in the ``code/grammars/`` directory at
the root of the coding-adventures repository. We locate it relative to this
module's file path::

    tokenizer.py
    └── javascript_lexer/   (parent)
        └── src/            (parent)
            └── javascript-lexer/ (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── javascript.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
JS_TOKENS_PATH = GRAMMAR_DIR / "javascript.tokens"


def create_javascript_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for JavaScript source code.

    Args:
        source: The JavaScript source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with JavaScript token definitions.

    Example::

        lexer = create_javascript_lexer('let x = 1 + 2;')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(JS_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_javascript(source: str) -> list[Token]:
    """Tokenize JavaScript source code and return a list of tokens.

    This is the main entry point for the JavaScript lexer. Pass in a string
    of JavaScript source code, and get back a flat list of ``Token`` objects.
    The list always ends with an ``EOF`` token.

    Args:
        source: The JavaScript source code to tokenize.

    Returns:
        A list of ``Token`` objects.

    Example::

        tokens = tokenize_javascript('let x = 1 + 2;')
        # [Token(KEYWORD, 'let', 1:1), Token(NAME, 'x', 1:5),
        #  Token(EQUALS, '=', 1:7), Token(NUMBER, '1', 1:9),
        #  Token(PLUS, '+', 1:11), Token(NUMBER, '2', 1:13),
        #  Token(SEMICOLON, ';', 1:14), Token(EOF, '', 1:15)]
    """
    lexer = create_javascript_lexer(source)
    return lexer.tokenize()
