"""TOML Lexer — tokenizes TOML text using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarLexer``. It
demonstrates that the same lexer engine used for Python, Ruby, JavaScript,
and JSON can tokenize TOML — just by loading a different ``.tokens`` file.

TOML is more complex than JSON but simpler than a full programming language.
It occupies a sweet spot that exercises several lexer features that JSON
does not:

- **Newline sensitivity** — TOML uses newlines to delimit key-value pairs,
  so the lexer emits NEWLINE tokens instead of skipping all whitespace.
- **Multiple string types** — basic, literal, multi-line basic, and
  multi-line literal strings each have different quoting and escape rules.
- **Date/time literals** — TOML natively supports ISO 8601 dates and times,
  which must be matched before bare keys and integers to avoid ambiguity.
- **Comments** — hash-to-end-of-line comments are skipped by the lexer.
- **Bare keys** — unquoted key names like ``server`` or ``my-key``.

Usage::

    from toml_lexer import tokenize_toml

    tokens = tokenize_toml('name = "TOML"\\nversion = "1.0.0"')
    for token in tokens:
        print(token)
"""

from lexer import LexerError, Token

from toml_lexer.tokenizer import create_toml_lexer, tokenize_toml

__all__ = [
    "LexerError",
    "Token",
    "create_toml_lexer",
    "tokenize_toml",
]
