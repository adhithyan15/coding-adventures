"""Verilog Lexer — tokenizes Verilog HDL source code using grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It
loads the ``verilog.tokens`` grammar file and optionally registers the
Verilog preprocessor as a ``pre_tokenize`` hook.

The Preprocessor Hook
---------------------

Verilog has a C-like preprocessor with directives like:

- `` `define WIDTH 8 ``     — macro definition
- `` `ifdef USE_CACHE ``    — conditional compilation
- `` `include "types.v" ``  — file inclusion (stubbed)
- `` `undef WIDTH ``        — undefine a macro

These directives operate on raw text *before* tokenization. The
preprocessor is registered as a ``pre_tokenize`` hook on the
``GrammarLexer``, following the same pattern described in
``lexer-parser-hooks.md`` for C preprocessor handling.

By default, ``create_verilog_lexer()`` registers the preprocessor.
Pass ``preprocess=False`` to disable it (useful for testing the
raw tokenizer without preprocessing).

Locating the Grammar File
--------------------------

The ``verilog.tokens`` file lives in the ``code/grammars/`` directory::

    tokenizer.py
    └── verilog_lexer/   (parent)
        └── src/            (parent)
            └── verilog-lexer/ (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── verilog.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

from verilog_lexer.preprocessor import verilog_preprocess

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
VERILOG_TOKENS_PATH = GRAMMAR_DIR / "verilog.tokens"


def create_verilog_lexer(
    source: str, *, preprocess: bool = True
) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Verilog source code.

    Args:
        source: The Verilog source code to tokenize.
        preprocess: Whether to run the Verilog preprocessor before
            tokenization. Defaults to True. Set to False to tokenize
            raw source without expanding macros or evaluating conditionals.

    Returns:
        A ``GrammarLexer`` instance configured with Verilog token definitions.

    Example::

        lexer = create_verilog_lexer('module m; endmodule')
        tokens = lexer.tokenize()

    Implementation Note
    -------------------

    The preprocessor is applied directly to the source text before creating
    the ``GrammarLexer``, rather than as a ``pre_tokenize`` hook. This is
    because the hook API (``add_pre_tokenize``) is specified in
    ``lexer-parser-hooks.md`` but not yet implemented in the base lexer
    package. Once the hooks are implemented, this can be refactored to use
    ``lexer.add_pre_tokenize(verilog_preprocess)`` instead.
    """
    if preprocess:
        source = verilog_preprocess(source)
    grammar = parse_token_grammar(VERILOG_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_verilog(source: str, *, preprocess: bool = True) -> list[Token]:
    """Tokenize Verilog source code and return a list of tokens.

    This is the main entry point for the Verilog lexer. Pass in a string
    of Verilog source code, and get back a flat list of ``Token`` objects.
    The list always ends with an ``EOF`` token.

    Args:
        source: The Verilog source code to tokenize.
        preprocess: Whether to run the Verilog preprocessor. Defaults to True.

    Returns:
        A list of ``Token`` objects.

    Example::

        tokens = tokenize_verilog('''
            module and_gate(input a, input b, output y);
                assign y = a & b;
            endmodule
        ''')
        # [Token(KEYWORD, 'module', ...), Token(NAME, 'and_gate', ...),
        #  Token(LPAREN, '(', ...), ...]
    """
    lexer = create_verilog_lexer(source, preprocess=preprocess)
    return lexer.tokenize()
