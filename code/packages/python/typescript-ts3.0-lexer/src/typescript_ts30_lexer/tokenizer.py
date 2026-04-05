"""TypeScript 3.0 (2018) Lexer — tokenizes TypeScript 3.0 using grammar-driven approach.

TypeScript 3.0 shipped in July 2018 with an ES2018 baseline. The headline
syntactic additions were the ``unknown`` top type (a type-safe alternative to
``any``) and first-class rest/spread support in tuple types.

What TypeScript 3.0 adds over TypeScript 2.x:

- ``unknown`` type keyword — the type-safe counterpart of ``any``
- Rest elements in tuple types: ``[string, ...number[]]``
- Spread expressions in tuple types
- Project references (``composite`` flag) for large monorepos
- ``--build`` mode flag (``tsc --build``)
- Generic spread expressions in function calls (``f<...T>``)
- ``@const`` decorator support improvements

At the lexer level, TypeScript is a superset of JavaScript. The token set
extends ECMAScript with TypeScript-specific keywords like ``type``,
``interface``, ``namespace``, ``declare``, ``abstract``, ``readonly``,
``keyof``, ``infer``, ``is``, ``as``, ``satisfies`` etc. Many of these are
*contextual* keywords — they are emitted as NAME tokens and the grammar layer
disambiguates them by position.

Locating the Grammar File
--------------------------

::

    tokenizer.py → typescript_ts30_lexer/ → src/ → typescript-ts3.0-lexer/
    → python/ → packages/ → code/ → grammars/typescript/ts3.0.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "typescript"
)
TS30_TOKENS_PATH = GRAMMAR_DIR / "ts3.0.tokens"


def create_ts30_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for TypeScript 3.0 (2018).

    Args:
        source: The TypeScript 3.0 source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with TS 3.0 token definitions.

    Example::

        lexer = create_ts30_lexer('const x: unknown = 42;')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(TS30_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_ts30(source: str) -> list[Token]:
    """Tokenize TypeScript 3.0 source code and return a list of tokens.

    Args:
        source: The TypeScript 3.0 source code to tokenize.

    Returns:
        A list of ``Token`` objects, always ending with ``EOF``.

    Example::

        tokens = tokenize_ts30('const x: unknown = 42;')
        # [Token(KEYWORD, 'const', 1:1), Token(NAME, 'x', 1:7),
        #  Token(COLON, ':', 1:8), ...]
    """
    lexer = create_ts30_lexer(source)
    return lexer.tokenize()
