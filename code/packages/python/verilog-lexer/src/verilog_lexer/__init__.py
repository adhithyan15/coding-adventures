"""Verilog Lexer — tokenizes Verilog HDL source code using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarLexer``
from the ``lexer`` package. It loads ``verilog.tokens`` and delegates
all tokenization to the generic engine.

What makes the Verilog lexer unique among our language wrappers is its
**preprocessor hook**. Verilog has a C-like preprocessor with directives
like `` `define ``, `` `ifdef ``, and `` `include ``. These are processed
as a ``pre_tokenize`` hook (str → str) that runs before tokenization,
expanding macros and evaluating conditionals.

Usage::

    from verilog_lexer import tokenize_verilog

    tokens = tokenize_verilog('''
        module and_gate(input a, input b, output y);
            assign y = a & b;
        endmodule
    ''')
    for token in tokens:
        print(token)
"""

from verilog_lexer.tokenizer import (
    DEFAULT_VERSION,
    SUPPORTED_VERSIONS,
    create_verilog_lexer,
    resolve_version,
    tokenize_verilog,
)
from verilog_lexer.preprocessor import verilog_preprocess

__all__ = [
    "create_verilog_lexer",
    "tokenize_verilog",
    "DEFAULT_VERSION",
    "SUPPORTED_VERSIONS",
    "resolve_version",
    "verilog_preprocess",
]
