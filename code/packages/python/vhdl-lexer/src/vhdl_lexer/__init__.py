"""VHDL Lexer — tokenizes VHDL source code using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarLexer``
from the ``lexer`` package. It loads ``vhdl.tokens`` and delegates
all tokenization to the generic engine.

What makes the VHDL lexer unique among our language wrappers is its
**case normalization** post-processing step. VHDL is case-insensitive:
``ENTITY``, ``Entity``, and ``entity`` are all the same identifier.
After tokenization, any token with type ``NAME`` or ``KEYWORD`` has
its value lowercased. This ensures downstream consumers see a single
canonical form for every identifier and keyword.

Unlike the Verilog lexer, VHDL has **no preprocessor**. All directives
in VHDL (like ``library`` and ``use``) are first-class language
constructs, not text-level transformations.

Usage::

    from vhdl_lexer import tokenize_vhdl

    tokens = tokenize_vhdl('''
        entity and_gate is
            port(a, b : in std_logic; y : out std_logic);
        end entity and_gate;
    ''')
    for token in tokens:
        print(token)
"""

from vhdl_lexer.tokenizer import (
    DEFAULT_VERSION,
    SUPPORTED_VERSIONS,
    create_vhdl_lexer,
    resolve_version,
    tokenize_vhdl,
)

__all__ = [
    "DEFAULT_VERSION",
    "SUPPORTED_VERSIONS",
    "create_vhdl_lexer",
    "resolve_version",
    "tokenize_vhdl",
]
