"""TypeScript Lexer — tokenizes TypeScript source code using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It
demonstrates a core idea of the grammar-driven architecture: the *same*
lexer engine that tokenizes Python and JavaScript can tokenize TypeScript
— or any other language — simply by swapping the ``.tokens`` file.

How the Grammar-Driven Approach Works
--------------------------------------

TypeScript has tokens that JavaScript does not — like ``interface``, ``type``,
``enum``, ``namespace``, and type-annotation keywords like ``number``,
``string``, ``boolean``. The grammar-driven approach handles all of these
without any new tokenization code: they are declared in the ``.tokens`` file,
and the ``GrammarLexer`` compiles them into regex patterns at runtime.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_typescript_lexer(source)`` — creates a ``GrammarLexer`` configured
  for TypeScript.

- ``tokenize_typescript(source)`` — the all-in-one function. Pass in TypeScript
  source code, get back a list of tokens.

Locating the Grammar File
--------------------------

The ``typescript.tokens`` file lives in the ``code/grammars/`` directory at
the root of the coding-adventures repository. We locate it relative to this
module's file path::

    tokenizer.py
    └── typescript_lexer/   (parent)
        └── src/            (parent)
            └── typescript-lexer/ (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── typescript.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
TS_TOKENS_PATH = GRAMMAR_DIR / "typescript.tokens"


def create_typescript_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for TypeScript source code.

    Args:
        source: The TypeScript source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with TypeScript token definitions.

    Example::

        lexer = create_typescript_lexer('let x: number = 1 + 2;')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(TS_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_typescript(source: str) -> list[Token]:
    """Tokenize TypeScript source code and return a list of tokens.

    This is the main entry point for the TypeScript lexer. Pass in a string
    of TypeScript source code, and get back a flat list of ``Token`` objects.
    The list always ends with an ``EOF`` token.

    Args:
        source: The TypeScript source code to tokenize.

    Returns:
        A list of ``Token`` objects.

    Example::

        tokens = tokenize_typescript('let x: number = 1 + 2;')
        # [Token(KEYWORD, 'let', 1:1), Token(NAME, 'x', 1:5),
        #  Token(COLON, ':', 1:6), Token(KEYWORD, 'number', 1:8),
        #  Token(EQUALS, '=', 1:15), Token(NUMBER, '1', 1:17),
        #  Token(PLUS, '+', 1:19), Token(NUMBER, '2', 1:21),
        #  Token(SEMICOLON, ';', 1:22), Token(EOF, '', 1:23)]
    """
    lexer = create_typescript_lexer(source)
    return lexer.tokenize()
