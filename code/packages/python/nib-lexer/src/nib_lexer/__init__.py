"""Nib Lexer — tokenizes Nib source text using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarLexer``. It
demonstrates the grammar-driven architecture applied to Nib — a safe toy
language targeting the Intel 4004 (the world's first commercial microprocessor,
released by Intel in November 1971).

Nib (named after *nibble*, the 4-bit unit) targets the 4004's extreme hardware
constraints: 4-bit registers, 160 bytes of RAM, a 3-level hardware call stack,
and no multiply or divide instructions in hardware.

This package loads ``nib.tokens`` from the ``code/grammars/`` directory and
uses the same ``GrammarLexer`` engine that powers the ALGOL 60, JSON, Python,
Ruby, and Starlark lexers. One lexer engine handles radically different
languages — from 1960 ALGOL to a 2024 embedded toy language.

Usage::

    from nib_lexer import tokenize_nib

    tokens = tokenize_nib('let x: u4 = 0xF;')
    for token in tokens:
        print(token)
"""

from __future__ import annotations

from nib_lexer.tokenizer import create_nib_lexer, tokenize_nib

__all__ = [
    "create_nib_lexer",
    "tokenize_nib",
]
