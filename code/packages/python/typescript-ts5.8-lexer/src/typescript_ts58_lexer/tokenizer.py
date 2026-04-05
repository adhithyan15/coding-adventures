"""TypeScript 5.8 (2025) Lexer — tokenizes TypeScript using grammar-driven approach.

TypeScript 5.8, released February 2025, aligns with the ES2025 specification.
ES2025 is a landmark release that standardizes three major features: decorators,
import attributes, and explicit resource management (``using``).

What TypeScript 5.8 adds over TypeScript 5.0:

- ES2025 as the baseline (standardizes decorators, import attributes, ``using``)
- ``HASHBANG`` token — ``#!/usr/bin/env node`` at the start of scripts
- Regex ``v`` flag support (ES2024 Unicode Sets mode)
- ``--erasableSyntaxOnly`` mode — only emit JavaScript-erasable syntax
- ``import type`` from computed module specifiers
- Conditional types improvements (deferred/lazy evaluation)
- ``export type *`` re-exports

ES2025 Baseline Features
-------------------------

Three landmark features reach standardization in ES2025:

1. **Standard TC39 Decorators** (Stage 4 in 2023)
   Decorators are functions receiving the decorated value and a context object.
   They can replace the value or register metadata. Predictable and composable::

       @logged
       class Foo {
           @memoize
           greet() { return "hello"; }
       }

2. **Import Attributes** — ``with { type: "json" }``
   Provides metadata to the module loader. Uses ``with`` (already reserved)
   instead of the withdrawn ``assert`` keyword::

       import data from "./config.json" with { type: "json" };
       import styles from "./main.css" with { type: "css" };

3. **Explicit Resource Management** — ``using`` and ``await using``
   Deterministic cleanup. Resources implement ``Symbol.dispose`` (sync) or
   ``Symbol.asyncDispose`` (async)::

       {
           using conn = openConnection();
           // ... conn[Symbol.dispose]() called automatically on exit
       }

       async function example() {
           await using db = await connect();
           // ... await db[Symbol.asyncDispose]() called automatically
       }

HASHBANG Token
---------------

ES2025 standardizes hashbang comments (``#!`` on the first line). Node.js
has supported them for years; now JavaScript parsers must also handle them::

    #!/usr/bin/env node
    console.log("Running with Node.js");

The hashbang becomes a comment token (similar to ``//``). TypeScript supports
this in scripts that will be run directly.

Locating the Grammar File
--------------------------

::

    tokenizer.py → typescript_ts58_lexer/ → src/ → typescript-ts5.8-lexer/
    → python/ → packages/ → code/ → grammars/typescript/ts5.8.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "typescript"
)
TS58_TOKENS_PATH = GRAMMAR_DIR / "ts5.8.tokens"


def create_ts58_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for TypeScript 5.8 (2025).

    Args:
        source: The TypeScript 5.8 source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with TS 5.8 token definitions.

    Example::

        lexer = create_ts58_lexer('using x = getResource();')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(TS58_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_ts58(source: str) -> list[Token]:
    """Tokenize TypeScript 5.8 source code and return a list of tokens.

    Args:
        source: The TypeScript 5.8 source code to tokenize.

    Returns:
        A list of ``Token`` objects, always ending with ``EOF``.

    Example::

        tokens = tokenize_ts58('await using db = await connect();')
        # [Token(NAME, 'await', ...), Token(NAME, 'using', ...),
        #  Token(NAME, 'db', ...), Token(EQUALS, '=', ...),
        #  Token(KEYWORD, 'await', ...), ...]
    """
    lexer = create_ts58_lexer(source)
    return lexer.tokenize()
