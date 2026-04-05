"""TypeScript 2.0 (September 2016) Lexer — tokenizes TS 2.0 source code.

TypeScript 2.0 was released in September 2016, exactly two years after
TypeScript 1.0. It upgraded the JavaScript baseline from ECMAScript 5 to
ECMAScript 2015 (ES6), bringing many new syntactic constructs to the lexer.

What TypeScript 2.0 Adds over TS 1.0
---------------------------------------

TypeScript 2.0 is primarily a *type system* release. The major lexical change
is the promotion of ECMAScript 2015 (ES2015) tokens:

New Type System Tokens:
- ``never`` type — a type that never occurs (e.g., a function that always throws)
- ``object`` type — represents non-primitive types
- ``undefined`` — was already a NAME, but now has dedicated type semantics

New ES2015 Syntax (baseline upgrade from ES5 → ES2015):
- ``let`` and ``const`` — block-scoped variable declarations
- ``class`` — class syntax with extends, implements
- Template literals — `` `Hello ${name}` `` using backticks
- Arrow functions — ``(x) => x + 1``
- Destructuring — ``const { x, y } = obj``
- Default parameters — ``function foo(x = 1)``
- Rest parameters — ``function foo(...args)``
- Spread operator — ``[...arr]``
- ``import`` / ``export`` — ES2015 module system
- ``from`` / ``as`` in import statements — ``import { Foo } from "./foo"``
- ``of`` in for..of loops — ``for (const x of arr)``
- ``Symbol`` — new primitive type
- ``Promise`` — built-in async type
- Generator syntax — ``function*`` and ``yield``

Context Keywords in TS 2.0
----------------------------

TS 2.0 adds more context keywords beyond TS 1.0:

- ``never`` — only a keyword in type positions
- ``object`` — only a keyword in type positions
- ``readonly`` — property modifier
- ``is`` — in return type position (type predicates)
- ``infer`` — in conditional types (TS 2.8, but grammar includes it)
- ``unique`` — in ``unique symbol`` types
- ``global`` — in ``declare global`` augmentation

Locating the Grammar File
--------------------------

::

    tokenizer.py → typescript_ts20_lexer/ → src/ → typescript-ts2.0-lexer/
    → python/ → packages/ → code/ → grammars/typescript/ts2.0.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "typescript"
)
TS20_TOKENS_PATH = GRAMMAR_DIR / "ts2.0.tokens"


def create_ts20_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for TypeScript 2.0 (September 2016).

    TypeScript 2.0 upgraded the JavaScript baseline to ECMAScript 2015 (ES6),
    adding non-nullable types, the ``never`` type, strict null checks, and
    tagged template types to the TS 1.0 feature set.

    Args:
        source: The TypeScript 2.0 source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with TS2.0 token definitions.

    Example::

        lexer = create_ts20_lexer('let x: string | null = null;')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(TS20_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_ts20(source: str) -> list[Token]:
    """Tokenize TypeScript 2.0 source code and return a list of tokens.

    TypeScript 2.0 is a superset of TS 1.0, which is a superset of ES5.
    All valid TS 1.0 and ES5 programs are valid TS 2.0. The lexer adds
    ES2015 tokens and new TS 2.0 type system tokens.

    Args:
        source: The TypeScript 2.0 source code to tokenize.

    Returns:
        A list of ``Token`` objects, always ending with ``EOF``.

    Example::

        tokens = tokenize_ts20('const x: never = undefined as never;')
        # Includes NAME tokens for 'never', 'undefined', 'as'
    """
    lexer = create_ts20_lexer(source)
    return lexer.tokenize()
