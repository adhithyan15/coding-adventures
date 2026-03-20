"""Starlark Lexer — tokenizes Starlark source code using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It
demonstrates a core idea of the grammar-driven architecture: the *same*
lexer engine that tokenizes Python can tokenize Starlark — or any other
language — simply by swapping the ``.tokens`` file.

How the Grammar-Driven Approach Works
--------------------------------------

Consider the traditional approach to supporting a new language. You would
need to write a brand-new lexer with custom logic for every new token type.
Starlark has operators like ``**`` (exponentiation) and ``//`` (floor
division), augmented assignments like ``+=`` and ``**=``, and four different
string quoting styles with optional prefixes (``r``, ``b``, ``rb``). A
hand-written approach would require adding new methods for each of these.

The grammar-driven approach sidesteps all of that. The ``starlark.tokens``
file declares what tokens Starlark has, and the ``GrammarLexer`` reads those
declarations and compiles them into regex patterns at runtime. No new Python
code is needed for the lexer itself.

Starlark-Specific Features
---------------------------

The ``starlark.tokens`` file uses several features that go beyond basic
token definitions:

1. **Indentation mode** (``mode: indentation``): The lexer tracks leading
   whitespace and emits ``INDENT``, ``DEDENT``, and ``NEWLINE`` tokens
   automatically. This is how Python-style block structure is handled
   without curly braces.

2. **Skip patterns** (``skip:``): Comments (``# ...``) and inline
   whitespace are matched and discarded without producing tokens. This
   keeps the token stream clean — the parser never sees whitespace.

3. **Type aliases** (``-> TYPE``): Multiple regex patterns can emit the
   same token type. For example, all eight string quoting styles
   (``STRING_DQ``, ``STRING_SQ``, ``STRING_RAW_DQ``, etc.) emit
   ``STRING``. This simplifies the grammar — it just says ``STRING``
   instead of listing every variant.

4. **Reserved keywords** (``reserved:``): Words like ``class``, ``import``,
   and ``while`` are legal Python identifiers but illegal in Starlark.
   If the lexer encounters one, it raises an error immediately instead
   of silently misinterpreting the code.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_starlark_lexer(source)`` — creates a ``GrammarLexer`` configured
  for Starlark. Use this when you want to control the tokenization process
  yourself (e.g., for streaming or incremental tokenization).

- ``tokenize_starlark(source)`` — the all-in-one function. Pass in Starlark
  source code, get back a list of tokens. This is the function most callers
  want.

Both functions handle locating and parsing the ``starlark.tokens`` file
automatically.

Locating the Grammar File
--------------------------

The ``starlark.tokens`` file lives in the ``code/grammars/`` directory at the
root of the coding-adventures repository. We locate it relative to this
module's file path using ``pathlib.Path``. This works regardless of where
the package is installed, as long as the repository structure is intact.

The path traversal is::

    tokenizer.py
    └── starlark_lexer/     (parent)
        └── src/            (parent)
            └── starlark-lexer/ (parent)
                └── python/     (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── starlark.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# We navigate from this file's location up to the repository root's
# grammars/ directory. The path is:
#   src/starlark_lexer/tokenizer.py -> src/starlark_lexer -> src ->
#   starlark-lexer -> python -> packages -> code -> code/grammars
#
# Using Path(__file__) makes this work regardless of the current working
# directory, which is important for testing and for use as an installed
# package.
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
STARLARK_TOKENS_PATH = GRAMMAR_DIR / "starlark.tokens"


def create_starlark_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Starlark source code.

    This function reads the ``starlark.tokens`` file, parses it into a
    ``TokenGrammar``, and creates a ``GrammarLexer`` ready to tokenize
    the given source code.

    Use this when you want access to the lexer object itself — for example,
    to inspect its internal state or to integrate with a custom pipeline.
    For most use cases, ``tokenize_starlark()`` is simpler.

    Args:
        source: The Starlark source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with Starlark token
        definitions. Call ``.tokenize()`` on it to get the token list.

    Raises:
        FileNotFoundError: If the ``starlark.tokens`` file cannot be found.
        TokenGrammarError: If the ``.tokens`` file has syntax errors.

    Example::

        lexer = create_starlark_lexer('print("hello")')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(STARLARK_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_starlark(source: str) -> list[Token]:
    """Tokenize Starlark source code and return a list of tokens.

    This is the main entry point for the Starlark lexer. Pass in a string
    of Starlark source code, and get back a flat list of ``Token`` objects.
    The list always ends with an ``EOF`` token.

    Because Starlark uses indentation mode, the returned token list will
    include synthetic tokens that do not correspond to literal characters
    in the source:

    - ``NEWLINE`` — marks the end of a logical line
    - ``INDENT`` — marks an increase in indentation level
    - ``DEDENT`` — marks a decrease in indentation level

    These tokens are suppressed inside brackets (``()``, ``[]``, ``{}``),
    which allows multi-line expressions like::

        result = func(
            arg1,
            arg2,
        )

    to be tokenized without spurious INDENT/DEDENT/NEWLINE tokens.

    The function handles all the setup internally: locating the grammar
    file, parsing it, creating the lexer, and running the tokenization.

    Args:
        source: The Starlark source code to tokenize.

    Returns:
        A list of ``Token`` objects representing the lexical structure
        of the input. The last token is always ``Token(EOF, ...)``.

    Raises:
        FileNotFoundError: If the ``starlark.tokens`` file cannot be found.
        LexerError: If the source contains characters that don't match
            any token pattern in the Starlark grammar, or if a reserved
            keyword is encountered.

    Example::

        tokens = tokenize_starlark('x = 1 + 2')
        # [Token(NAME, 'x', 1:1), Token(EQUALS, '=', 1:3),
        #  Token(INT, '1', 1:5), Token(PLUS, '+', 1:7),
        #  Token(INT, '2', 1:9), Token(NEWLINE, ...), Token(EOF, '', ...)]
    """
    lexer = create_starlark_lexer(source)
    return lexer.tokenize()
