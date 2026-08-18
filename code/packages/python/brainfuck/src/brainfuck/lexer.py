"""Brainfuck Lexer — tokenizes Brainfuck source using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It uses a
pre-compiled ``TOKEN_GRAMMAR`` (see ``brainfuck._grammar_tokens``) and creates
a lexer configured for Brainfuck tokenization.

What Is Brainfuck?
------------------

Brainfuck is a minimalist esoteric programming language created by Urban Müller
in 1993. It has exactly **eight** meaningful characters:

    >   RIGHT      — Move the data pointer one cell to the right
    <   LEFT       — Move the data pointer one cell to the left
    +   INC        — Increment the byte at the data pointer
    -   DEC        — Decrement the byte at the data pointer
    .   OUTPUT     — Output the byte at the data pointer as ASCII
    ,   INPUT      — Read one byte from input into the current cell
    [   LOOP_START — Jump past matching ] if current cell is zero
    ]   LOOP_END   — Jump back to matching [ if current cell is nonzero

Every other character is silently ignored — it is a **comment**. Brainfuck has
no dedicated comment syntax; programmers annotate their code by writing natural
language prose directly, knowing the 8 command characters are unambiguous.

Comment Handling
----------------

The ``brainfuck.tokens`` grammar defines two skip patterns:

- ``WHITESPACE = /[ \\t\\r\\n]+/`` — matches whitespace including newlines.
  The lexer engine uses this pattern to advance its line counter, so line
  and column numbers in tokens remain accurate.
- ``COMMENT = /[^><+\\-.,[\\] \\t\\r\\n]+/`` — matches runs of non-command,
  non-whitespace characters. These are discarded silently.

The two-pattern design is deliberate: if COMMENT consumed newlines, the lexer's
internal line counter would lose track of line boundaries. Separating whitespace
from non-whitespace non-command characters ensures both are skipped **and** that
line/column tracking stays correct.

What This Module Provides
--------------------------

Two convenience functions:

- ``create_brainfuck_lexer(source)`` — creates a ``GrammarLexer`` configured
  for Brainfuck. Use this when you want to control the tokenization process
  yourself (e.g., to tokenize lazily or inspect the grammar object).
- ``tokenize_brainfuck(source)`` — the all-in-one function. Pass in Brainfuck
  source text, get back a list of tokens. This is the function most callers want.

The Brainfuck token grammar is compiled into ``_grammar_tokens.py`` by the
grammar-tools compiler from ``code/grammars/brainfuck/brainfuck.tokens``.
Importing the pre-built ``TOKEN_GRAMMAR`` constant means no file I/O at
startup and no runtime grammar parsing overhead. (The parser grammar lives
in a separate ``_grammar_parser.py`` in this same package — see
``parser.py`` — since both a lexer and a parser share this ``brainfuck``
package.)

To regenerate after editing ``code/grammars/brainfuck/brainfuck.tokens``::

    grammar-tools compile-tokens code/grammars/brainfuck/brainfuck.tokens \\
        > code/packages/python/brainfuck/src/brainfuck/_grammar_tokens.py
"""

from __future__ import annotations

from lexer import GrammarLexer, Token

from brainfuck._grammar_tokens import TOKEN_GRAMMAR


def create_brainfuck_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Brainfuck source text.

    This function uses the pre-compiled ``TOKEN_GRAMMAR`` (from
    ``brainfuck._grammar_tokens``) to create a ``GrammarLexer`` ready to
    tokenize the given source text. No file I/O is performed.

    The resulting lexer recognises the 8 Brainfuck command characters and
    silently discards all other characters (comments and whitespace).

    Args:
        source: The Brainfuck source text to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with the Brainfuck token
        definitions. Call ``.tokenize()`` on it to get the token list.

    Example::

        lexer = create_brainfuck_lexer("++[>+<-]")
        tokens = lexer.tokenize()
        # [Token(INC, '+'), Token(INC, '+'), Token(LOOP_START, '['), ...]
    """
    return GrammarLexer(source, TOKEN_GRAMMAR)


def tokenize_brainfuck(source: str) -> list[Token]:
    """Tokenize Brainfuck source text and return a list of tokens.

    This is the main entry point for the Brainfuck lexer. Pass in a string
    of Brainfuck source, and get back a flat list of ``Token`` objects. The
    list always ends with an ``EOF`` token.

    Only command characters produce tokens. Everything else is discarded:

    - **Whitespace** (spaces, tabs, newlines, carriage returns) is consumed
      by the WHITESPACE skip pattern. Newlines advance the line counter.
    - **All other non-command characters** (letters, digits, punctuation) are
      consumed by the COMMENT skip pattern.

    The 8 token types you will see (besides EOF) are:

    - **RIGHT** — ``>`` (move data pointer right)
    - **LEFT** — ``<`` (move data pointer left)
    - **INC** — ``+`` (increment current cell)
    - **DEC** — ``-`` (decrement current cell)
    - **OUTPUT** — ``.`` (output current cell as ASCII)
    - **INPUT** — ``,`` (read input into current cell)
    - **LOOP_START** — ``[`` (begin loop)
    - **LOOP_END** — ``]`` (end loop)
    - **EOF** — end of input

    Args:
        source: The Brainfuck source text to tokenize.

    Returns:
        A list of ``Token`` objects. The last token is always EOF.

    Example::

        tokens = tokenize_brainfuck("++[>+<-]")
        # Token types: INC INC LOOP_START RIGHT INC LEFT DEC LOOP_END EOF

    Example::

        # Comments are silently discarded:
        tokens = tokenize_brainfuck("+ increment the cell")
        # Token types: INC EOF
        # "increment the cell" produces no tokens

    Example::

        # All 8 commands in one pass:
        tokens = tokenize_brainfuck("><+-.,[]")
        # Token types: RIGHT LEFT INC DEC OUTPUT INPUT LOOP_START LOOP_END EOF
    """
    lexer = create_brainfuck_lexer(source)
    return lexer.tokenize()
