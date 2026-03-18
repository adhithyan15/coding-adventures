"""Ruby Lexer — tokenizes Ruby source code using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It
demonstrates a core idea of the grammar-driven architecture: the *same*
lexer engine that tokenizes Python can tokenize Ruby — or any other language
— simply by swapping the ``.tokens`` file.

How the Grammar-Driven Approach Works
--------------------------------------

Consider the traditional approach to supporting a new language. You would
need to write a brand-new lexer with custom character-dispatching logic
for every new token type. Ruby has operators like ``..`` (range) and ``=>``
(hash rocket) that Python does not. A hand-written approach would require
adding new methods to handle these.

The grammar-driven approach sidesteps all of that. The ``ruby.tokens`` file
declares what tokens Ruby has, including ``..`` and ``=>``. The
``GrammarLexer`` reads those declarations and compiles them into regex
patterns at runtime. No new Python code is needed for the lexer itself.

What This Module Provides
-------------------------

Two convenience functions:

- ``create_ruby_lexer(source)`` — creates a ``GrammarLexer`` configured
  for Ruby. Use this when you want to control the tokenization process
  yourself (e.g., for streaming or incremental tokenization).

- ``tokenize_ruby(source)`` — the all-in-one function. Pass in Ruby source
  code, get back a list of tokens. This is the function most callers want.

Both functions handle locating and parsing the ``ruby.tokens`` file
automatically.

Locating the Grammar File
--------------------------

The ``ruby.tokens`` file lives in the ``code/grammars/`` directory at the
root of the coding-adventures repository. We locate it relative to this
module's file path using ``pathlib.Path``. This works regardless of where
the package is installed, as long as the repository structure is intact.

The path traversal is::

    tokenizer.py
    └── ruby_lexer/        (parent)
        └── src/           (parent)
            └── ruby-lexer/  (parent)
                └── python/    (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── ruby.tokens
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
#   src/ruby_lexer/tokenizer.py -> src/ruby_lexer -> src -> ruby-lexer
#   -> python -> packages -> code -> code/grammars
#
# Using Path(__file__) makes this work regardless of the current working
# directory, which is important for testing and for use as an installed
# package.
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
RUBY_TOKENS_PATH = GRAMMAR_DIR / "ruby.tokens"


def create_ruby_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Ruby source code.

    This function reads the ``ruby.tokens`` file, parses it into a
    ``TokenGrammar``, and creates a ``GrammarLexer`` ready to tokenize
    the given source code.

    Use this when you want access to the lexer object itself — for example,
    to inspect its internal state or to integrate with a custom pipeline.
    For most use cases, ``tokenize_ruby()`` is simpler.

    Args:
        source: The Ruby source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with Ruby token definitions.
        Call ``.tokenize()`` on it to get the token list.

    Raises:
        FileNotFoundError: If the ``ruby.tokens`` file cannot be found.
        TokenGrammarError: If the ``.tokens`` file has syntax errors.

    Example::

        lexer = create_ruby_lexer('puts("hello")')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(RUBY_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_ruby(source: str) -> list[Token]:
    """Tokenize Ruby source code and return a list of tokens.

    This is the main entry point for the Ruby lexer. Pass in a string of
    Ruby source code, and get back a flat list of ``Token`` objects. The
    list always ends with an ``EOF`` token.

    The function handles all the setup internally: locating the grammar
    file, parsing it, creating the lexer, and running the tokenization.

    Args:
        source: The Ruby source code to tokenize.

    Returns:
        A list of ``Token`` objects representing the lexical structure
        of the input. The last token is always ``Token(EOF, ...)``.

    Raises:
        FileNotFoundError: If the ``ruby.tokens`` file cannot be found.
        LexerError: If the source contains characters that don't match
            any token pattern in the Ruby grammar.

    Example::

        tokens = tokenize_ruby('x = 1 + 2')
        # [Token(NAME, 'x', 1:1), Token(EQUALS, '=', 1:3),
        #  Token(NUMBER, '1', 1:5), Token(PLUS, '+', 1:7),
        #  Token(NUMBER, '2', 1:9), Token(EOF, '', 1:10)]
    """
    lexer = create_ruby_lexer(source)
    return lexer.tokenize()
