"""ECMAScript 1 (1997) Lexer — tokenizes ES1 JavaScript using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It loads
the ``es1.tokens`` grammar file from ``code/grammars/ecmascript/`` and creates
a lexer configured for the very first version of standardized JavaScript.

ES1 (ECMA-262 1st Edition, June 1997) is the foundation of JavaScript:

- 26 keywords: ``break``, ``case``, ``continue``, ``default``, ``delete``,
  ``do``, ``else``, ``for``, ``function``, ``if``, ``in``, ``new``,
  ``return``, ``switch``, ``this``, ``typeof``, ``var``, ``void``,
  ``while``, ``with``, ``true``, ``false``, ``null``
- No ``===`` or ``!==`` (strict equality — that's ES3)
- No ``try``/``catch``/``finally``/``throw`` (error handling — that's ES3)
- No regex literals (formalized in ES3)
- No ``let``/``const``/``class``/arrow functions (that's ES2015)

Locating the Grammar File
--------------------------

The ``es1.tokens`` file lives in ``code/grammars/ecmascript/`` at the
repository root. We locate it relative to this module's file path::

    tokenizer.py
    └── ecmascript_es1_lexer/  (parent)
        └── src/               (parent)
            └── ecmascript-es1-lexer/ (parent)
                └── python/           (parent)
                    └── packages/     (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── ecmascript/
                                    └── es1.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "ecmascript"
)
ES1_TOKENS_PATH = GRAMMAR_DIR / "es1.tokens"


def create_es1_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for ECMAScript 1 (1997).

    Args:
        source: The ES1 JavaScript source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with ES1 token definitions.

    Example::

        lexer = create_es1_lexer('var x = 1 + 2;')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(ES1_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_es1(source: str) -> list[Token]:
    """Tokenize ECMAScript 1 source code and return a list of tokens.

    This is the main entry point for the ES1 lexer. Pass in a string of
    JavaScript source code (ES1-era), and get back a flat list of ``Token``
    objects. The list always ends with an ``EOF`` token.

    Args:
        source: The ES1 JavaScript source code to tokenize.

    Returns:
        A list of ``Token`` objects.

    Example::

        tokens = tokenize_es1('var x = 1 + 2;')
        # [Token(KEYWORD, 'var', 1:1), Token(NAME, 'x', 1:5),
        #  Token(EQUALS, '=', 1:7), Token(NUMBER, '1', 1:9),
        #  Token(PLUS, '+', 1:11), Token(NUMBER, '2', 1:13),
        #  Token(SEMICOLON, ';', 1:14), Token(EOF, '', 1:15)]
    """
    lexer = create_es1_lexer(source)
    return lexer.tokenize()
