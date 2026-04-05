"""Python Lexer — tokenizes Python source code using versioned grammar files.

This package supports multiple Python versions (2.7, 3.0, 3.6, 3.8, 3.10,
3.12), each with its own ``.tokens`` grammar file that captures the exact
token set for that version. The grammar files live at
``code/grammars/python/pythonX.Y.tokens`` and are loaded at runtime by
the grammar-driven lexer.

Why Versioned Grammars?
-----------------------

Python's lexical grammar has changed significantly over the years. Consider
just a few examples:

- **Python 2.7 vs 3.0**: The ``print`` keyword became a function. ``exec``
  stopped being a keyword. The ``<>`` operator was removed. Unicode string
  prefixes changed.

- **Python 3.6**: f-strings (``f"..."``) were introduced, adding a new
  string prefix.

- **Python 3.8**: The walrus operator (``:=``) was added.

- **Python 3.10**: ``match`` and ``case`` became soft keywords (they are
  keywords only inside match statements, but remain valid identifiers
  elsewhere).

- **Python 3.12**: ``type`` became a soft keyword for type alias statements.

By maintaining a separate ``.tokens`` file per version, each grammar
precisely captures the lexical rules for that version. A Python 2.7 grammar
does not know about f-strings; a Python 3.0 grammar does not have the
walrus operator.

How It Works
------------

The Python lexer is a **thin wrapper** around the generic ``GrammarLexer``
from the ``lexer`` package. It does three things:

1. Resolves the version string to a grammar file path.
2. Parses that file into a ``TokenGrammar`` using ``grammar_tools``.
3. Feeds the grammar to ``GrammarLexer``, which handles the actual
   tokenization.

Parsed grammars are cached per version, so the file is only read and
parsed once regardless of how many times you call ``tokenize_python()``.

Usage::

    from python_lexer import tokenize_python

    # Default version (3.12)
    tokens = tokenize_python('x = 1 + 2')

    # Specific version
    tokens = tokenize_python('print "hello"', version="2.7")
"""

from python_lexer.tokenizer import (
    DEFAULT_VERSION,
    SUPPORTED_VERSIONS,
    create_python_lexer,
    tokenize_python,
)

__all__ = [
    "DEFAULT_VERSION",
    "SUPPORTED_VERSIONS",
    "create_python_lexer",
    "tokenize_python",
]
