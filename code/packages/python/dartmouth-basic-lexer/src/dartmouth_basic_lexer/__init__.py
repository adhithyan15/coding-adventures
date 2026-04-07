"""Dartmouth BASIC 1964 Lexer — tokenizes the original BASIC language.

This package is a **thin wrapper** around the generic ``GrammarLexer``. It
demonstrates the grammar-driven architecture applied to Dartmouth BASIC —
the language that introduced computing to a generation of non-scientists and
seeded the personal computer revolution of the 1970s and 1980s.

Dartmouth BASIC (1964) was created by John Kemeny and Thomas Kurtz at
Dartmouth College. Their goal: make programming accessible to every student.
The result was a language with 20 keywords, 11 built-in functions, and an
elegantly simple line-numbered structure that ran on teletypes in real time.

This package loads ``dartmouth_basic.tokens`` from the ``code/grammars/``
directory and uses the same ``GrammarLexer`` engine that powers the JSON,
ALGOL, Python, and JavaScript lexers. Two post-tokenize hooks handle the
language-specific challenges:

  - **LINE_NUM relabeling**: The first integer on each source line is
    relabeled from NUMBER to LINE_NUM, distinguishing line labels from
    numeric literals in expressions.

  - **REM suppression**: Everything after a REM keyword until end-of-line
    is stripped — implementing BASIC's comment syntax.

Usage::

    from dartmouth_basic_lexer import tokenize_dartmouth_basic

    source = "10 LET X = 5\\n20 PRINT X\\n30 END\\n"
    tokens = tokenize_dartmouth_basic(source)
    for token in tokens:
        print(f"{token.type:12} {token.value!r}")

    # LINE_NUM     '10'
    # KEYWORD      'LET'
    # NAME         'X'
    # EQ           '='
    # NUMBER       '5'
    # NEWLINE      '\\n'
    # ... and so on

For direct access to the GrammarLexer (to add custom hooks or inspect the
grammar), use ``create_dartmouth_basic_lexer``::

    from dartmouth_basic_lexer import create_dartmouth_basic_lexer

    lexer = create_dartmouth_basic_lexer("10 PRINT X\\n")
    lexer.add_post_tokenize(my_custom_hook)
    tokens = lexer.tokenize()
"""

from dartmouth_basic_lexer.tokenizer import (
    create_dartmouth_basic_lexer,
    tokenize_dartmouth_basic,
)

__all__ = [
    "create_dartmouth_basic_lexer",
    "tokenize_dartmouth_basic",
]
