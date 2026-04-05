"""TypeScript 1.0 (April 2014) Lexer — tokenizes TS 1.0 source code
using grammar-driven approach.

TypeScript 1.0 was the first public release of the TypeScript language,
announced at Microsoft's Build developer conference in April 2014. It was a
strict superset of ECMAScript 5, adding a static type system to JavaScript.

What TypeScript 1.0 adds over ES5:

- Type annotations: ``var x: number = 1;``
- Interfaces: ``interface Foo { x: string; }``
- Classes: ``class Animal { name: string; }``
- Enums: ``enum Color { Red, Green, Blue }``
- Generics (angle-bracket syntax): ``Array<string>``
- Type aliases: ``type Alias = string;``
- Namespaces (called "modules" at the time): ``namespace MyNS { }``
- Ambient declarations: ``declare var x: number;``
- Type assertions: ``<string>x`` and ``x as string``
- Decorators: ``@Component`` (experimental at TS 1.0)
- Non-null assertion operator: ``x!``

New Tokens for TypeScript 1.0
------------------------------

The grammar adds several tokens not in ES5:

- ``AT`` (``@``) — decorator prefix
- ``COLON`` (already existed in ES5 for object literals, repurposed for
  type annotations)
- ``LESS_THAN`` / ``GREATER_THAN`` (``<`` and ``>``) — generic type parameters
- ``QUESTION_MARK`` (``?``) — optional parameter marker
- ``EXCLAMATION`` (``!``) — non-null assertion

Context Keywords
-----------------

TypeScript 1.0 has many *context keywords* — identifiers that are only
keywords in certain positions. For example:

- ``interface`` — not a keyword in ES5, but TS treats it specially
- ``type`` — only a keyword in certain positions
- ``namespace`` — only a keyword at statement start
- ``declare`` — only a keyword at the beginning of ambient declarations
- ``abstract`` — only a keyword before ``class``
- ``readonly`` — only a keyword before a property
- ``from`` / ``of`` — only keywords in import/for..of statements

The lexer emits these as ``NAME`` tokens. The parser resolves their meaning
from context.

Locating the Grammar File
--------------------------

::

    tokenizer.py → typescript_ts10_lexer/ → src/ → typescript-ts1.0-lexer/
    → python/ → packages/ → code/ → grammars/typescript/ts1.0.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "typescript"
)
TS10_TOKENS_PATH = GRAMMAR_DIR / "ts1.0.tokens"


def create_ts10_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for TypeScript 1.0 (April 2014).

    TypeScript 1.0 was the first public release of TypeScript, a statically
    typed superset of JavaScript. It added type annotations, interfaces,
    classes, enums, and generics on top of ECMAScript 5.

    Args:
        source: The TypeScript 1.0 source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with TS1.0 token definitions.

    Example::

        lexer = create_ts10_lexer('var x: number = 1;')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(TS10_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_ts10(source: str) -> list[Token]:
    """Tokenize TypeScript 1.0 source code and return a list of tokens.

    TypeScript 1.0 is a superset of ES5 — all valid ES5 is valid TS 1.0.
    The lexer produces the same tokens as ES5 for JavaScript constructs,
    plus additional tokens for TypeScript-specific syntax.

    Args:
        source: The TypeScript 1.0 source code to tokenize.

    Returns:
        A list of ``Token`` objects, always ending with ``EOF``.

    Example::

        tokens = tokenize_ts10('var x: number = 1;')
        # [Token(KEYWORD, 'var', 1:1), Token(NAME, 'x', 1:5),
        #  Token(COLON, ':', 1:6), Token(NAME, 'number', 1:8),
        #  Token(EQUALS, '=', 1:15), Token(NUMBER, '1', 1:17),
        #  Token(SEMICOLON, ';', 1:18), Token(EOF, '', 1:19)]
    """
    lexer = create_ts10_lexer(source)
    return lexer.tokenize()
