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

Pre-compiled Grammar
---------------------

The ``ruby.tokens`` file is compiled ahead of time by the ``grammar-tools``
compiler into ``ruby_lexer/_grammar.py``, which embeds the ``TokenGrammar``
as native Python data structures. This module imports ``TOKEN_GRAMMAR``
from it directly — no file I/O or grammar parsing happens at runtime, and
the package works correctly when installed as a site-package (it does not
depend on the ``code/grammars/`` directory existing on disk).

To regenerate after editing ``code/grammars/ruby/ruby.tokens``:

    grammar-tools compile-tokens code/grammars/ruby/ruby.tokens \\
        -o code/packages/python/ruby-lexer/src/ruby_lexer/_grammar.py
"""

from __future__ import annotations

from lexer import GrammarLexer, Token

from ruby_lexer._grammar import TOKEN_GRAMMAR


def create_ruby_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Ruby source code.

    This function uses the pre-compiled ``TOKEN_GRAMMAR`` (from
    ``ruby_lexer._grammar``) to create a ``GrammarLexer`` ready to tokenize
    the given source code. No file I/O is performed.

    Use this when you want access to the lexer object itself — for example,
    to inspect its internal state or to integrate with a custom pipeline.
    For most use cases, ``tokenize_ruby()`` is simpler.

    Args:
        source: The Ruby source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with Ruby token definitions.
        Call ``.tokenize()`` on it to get the token list.

    Example::

        lexer = create_ruby_lexer('puts("hello")')
        tokens = lexer.tokenize()
    """
    return GrammarLexer(source, TOKEN_GRAMMAR)


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
