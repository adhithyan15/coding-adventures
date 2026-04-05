r"""TypeScript 4.0 (2020) Lexer — tokenizes TypeScript 4.0
using grammar-driven approach.

TypeScript 4.0 shipped in August 2020 with an ES2020 baseline. The headline
syntactic additions were variadic tuple types, template literal types, and
labeled tuple elements — all significant grammar-level changes.

What TypeScript 4.0 adds over TypeScript 3.x:

- **Variadic tuple types**:
  ``type Concat<T extends unknown[], U extends unknown[]> = [...T, ...U]``
  Generic spread in tuple positions, not just fixed-size spreads from TS 3.0.
- **Labeled tuple elements**: ``type Range = [start: number, end: number]``
  Named positions in tuples for better tooling support.
- **Template literal types**: ``type Greeting = \`Hello, ${string}!\```
  Types can be constructed by string interpolation at the type level.
- **Class property inference from constructors** (``--noUncheckedIndexedAccess``)
- Short-circuit assignment operators: ``&&=``, ``||=``, ``??=``
- ``catch`` variable type narrowing (catch-all as ``unknown``)

At the lexer level, TypeScript is a superset of JavaScript. The new operators
``&&=``, ``||=``, ``??=`` require new multi-character tokens. Template literal
types reuse the same backtick grammar as template literal values.

Locating the Grammar File
--------------------------

::

    tokenizer.py → typescript_ts40_lexer/ → src/ → typescript-ts4.0-lexer/
    → python/ → packages/ → code/ → grammars/typescript/ts4.0.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "typescript"
)
TS40_TOKENS_PATH = GRAMMAR_DIR / "ts4.0.tokens"


def create_ts40_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for TypeScript 4.0 (2020).

    Args:
        source: The TypeScript 4.0 source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with TS 4.0 token definitions.

    Example::

        lexer = create_ts40_lexer('type Pair = [first: string, second: number];')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(TS40_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_ts40(source: str) -> list[Token]:
    """Tokenize TypeScript 4.0 source code and return a list of tokens.

    Args:
        source: The TypeScript 4.0 source code to tokenize.

    Returns:
        A list of ``Token`` objects, always ending with ``EOF``.

    Example::

        tokens = tokenize_ts40('type Pair = [first: string, second: number];')
        # [Token(NAME, 'type', 1:1), Token(NAME, 'Pair', 1:6),
        #  Token(EQUALS, '=', 1:11), ...]
    """
    lexer = create_ts40_lexer(source)
    return lexer.tokenize()
