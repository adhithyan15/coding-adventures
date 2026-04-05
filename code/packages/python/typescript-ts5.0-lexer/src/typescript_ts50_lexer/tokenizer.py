"""TypeScript 5.0 (2023) Lexer — tokenizes TypeScript using grammar-driven approach.

TypeScript 5.0, released March 2023, is a major release with several significant
improvements. It targets ES2022 as its baseline, bringing class fields, private
class members (``#name``), and static initialization blocks.

What TypeScript 5.0 adds over TypeScript 4.x:

- Standard TC39 decorators (replaces ``--experimentalDecorators`` default)
- ``const`` type parameters: ``function identity<const T>(arg: T): T``
  Preserves the literal type rather than widening to the base type.
- Multiple ``tsconfig.json`` extends (array form)
- ``--verbatimModuleSyntax`` flag: controls how import/export of types is emitted
- ``accessor`` keyword for auto-accessor class members (with decorators)
- ``satisfies`` operator (from TS 4.9, but fully integrated in 5.0 tooling)
- ``using`` context keyword (TS 5.2 explicit resource management, lexically present)

ES2022 Baseline (What Class Fields Mean)
-----------------------------------------

Before ES2022, class bodies could only hold methods. After ES2022::

    class Counter {
        count = 0;                   # public field
        #secret = "hidden";          # truly private field (runtime enforced)
        static instances = 0;        # static field
        static { Counter.instances = 0; }  # static init block

    }

TypeScript integrates these with its type system — fields can have type
annotations, access modifiers, and readonly qualifiers.

Standard Decorators (TS 5.0 Default)
--------------------------------------

TS 5.0 adopts the TC39 stage-3 decorator proposal by default.
The ``@`` token is the same; the semantics changed::

    @sealed
    class BugReport {
        @logged
        accessor title: string = "";
    }

Standard decorators are functions that receive the decorated value and a
context object (kind, name, access). They can return a replacement.

Locating the Grammar File
--------------------------

::

    tokenizer.py → typescript_ts50_lexer/ → src/ → typescript-ts5.0-lexer/
    → python/ → packages/ → code/ → grammars/typescript/ts5.0.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars" / "typescript"
)
TS50_TOKENS_PATH = GRAMMAR_DIR / "ts5.0.tokens"


def create_ts50_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for TypeScript 5.0 (2023).

    Args:
        source: The TypeScript 5.0 source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with TS 5.0 token definitions.

    Example::

        lexer = create_ts50_lexer('@decorator class Foo {}')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(TS50_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_ts50(source: str) -> list[Token]:
    """Tokenize TypeScript 5.0 source code and return a list of tokens.

    Args:
        source: The TypeScript 5.0 source code to tokenize.

    Returns:
        A list of ``Token`` objects, always ending with ``EOF``.

    Example::

        tokens = tokenize_ts50('const x: number = 1;')
        # [Token(KEYWORD, 'const', 1:1), Token(NAME, 'x', 1:7),
        #  Token(COLON, ':', 1:8), Token(NAME, 'number', 1:10),
        #  Token(EQUALS, '=', 1:17), Token(NUMBER, '1', 1:19),
        #  Token(SEMICOLON, ';', 1:20), Token(EOF, '', 1:21)]
    """
    lexer = create_ts50_lexer(source)
    return lexer.tokenize()
