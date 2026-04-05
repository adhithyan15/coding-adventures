"""ECMAScript 5 (2009) Lexer — tokenizes ES5 JavaScript using grammar-driven approach.

ES5 (ECMA-262 5th Edition, December 2009) landed a decade after ES3. ES4 was
abandoned after years of committee disagreement. The syntactic changes in ES5
are modest — the big innovations were strict mode and property descriptors.

What ES5 adds over ES3:

- ``debugger`` keyword (promoted from future-reserved)
- Getter/setter syntax in object literals (parsed by the grammar, not the lexer)
- String line continuation (backslash before newline)

The token set is nearly identical to ES3. The real difference is in the keyword
list (``debugger`` is now a keyword) and the future-reserved list (significantly
reduced from ES3's large set).

Locating the Grammar File
--------------------------

::

    tokenizer.py → ecmascript_es5_lexer/ → src/ → ecmascript-es5-lexer/
    → python/ → packages/ → code/ → grammars/ecmascript/es5.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "ecmascript"
)
ES5_TOKENS_PATH = GRAMMAR_DIR / "es5.tokens"


def create_es5_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for ECMAScript 5 (2009).

    Args:
        source: The ES5 JavaScript source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with ES5 token definitions.

    Example::

        lexer = create_es5_lexer('debugger; var x = 1;')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(ES5_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_es5(source: str) -> list[Token]:
    """Tokenize ECMAScript 5 source code and return a list of tokens.

    Args:
        source: The ES5 JavaScript source code to tokenize.

    Returns:
        A list of ``Token`` objects, always ending with ``EOF``.

    Example::

        tokens = tokenize_es5('debugger;')
        # [Token(KEYWORD, 'debugger', 1:1), Token(SEMICOLON, ';', 1:9),
        #  Token(EOF, '', 1:10)]
    """
    lexer = create_es5_lexer(source)
    return lexer.tokenize()
