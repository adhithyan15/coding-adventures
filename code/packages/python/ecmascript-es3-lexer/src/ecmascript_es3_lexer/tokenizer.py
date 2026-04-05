"""ECMAScript 3 (1999) Lexer — tokenizes ES3 JavaScript using grammar-driven approach.

ES3 (ECMA-262 3rd Edition, December 1999) was the version that made JavaScript
a real, complete language. It adds several critical features over ES1:

- ``===`` and ``!==`` (strict equality — no type coercion)
- ``try``/``catch``/``finally``/``throw`` (structured error handling)
- Regular expression literals (``/pattern/flags``)
- ``instanceof`` operator (prototype chain check)
- 5 new keywords: ``catch``, ``finally``, ``instanceof``, ``throw``, ``try``

Regex vs Division Disambiguation
---------------------------------

The ``/`` character is ambiguous: it could start a regex or be division.
The ``GrammarLexer`` resolves this using context from the previous token.
After expression-ending tokens (``)`, ``]``, ``NAME``, ``NUMBER``, etc.),
``/`` is division. After operators and statement-starting tokens, ``/``
starts a regex.

Locating the Grammar File
--------------------------

::

    tokenizer.py → ecmascript_es3_lexer/ → src/ → ecmascript-es3-lexer/
    → python/ → packages/ → code/ → grammars/ecmascript/es3.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "ecmascript"
)
ES3_TOKENS_PATH = GRAMMAR_DIR / "es3.tokens"


def create_es3_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for ECMAScript 3 (1999).

    Args:
        source: The ES3 JavaScript source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with ES3 token definitions.

    Example::

        lexer = create_es3_lexer('try { x === 1; } catch (e) { }')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(ES3_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_es3(source: str) -> list[Token]:
    """Tokenize ECMAScript 3 source code and return a list of tokens.

    Args:
        source: The ES3 JavaScript source code to tokenize.

    Returns:
        A list of ``Token`` objects, always ending with ``EOF``.

    Example::

        tokens = tokenize_es3('x === 1')
        # [Token(NAME, 'x', 1:1), Token(STRICT_EQUALS, '===', 1:3),
        #  Token(NUMBER, '1', 1:7), Token(EOF, '', 1:8)]
    """
    lexer = create_es3_lexer(source)
    return lexer.tokenize()
