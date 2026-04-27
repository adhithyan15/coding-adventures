"""Oct Lexer — tokenizes Oct source text using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarLexer``. It
demonstrates the grammar-driven architecture applied to Oct — a safe, statically-
typed toy language targeting the Intel 8008 (the world's first commercial 8-bit
microprocessor, released by Intel in 1972).

Oct (named after *octet*, the networking term for exactly 8 bits) targets the
8008's distinctive hardware:

- **8-bit words** — ``u8`` is the native ALU type, values 0–255
- **7 usable registers** — A (accumulator), B, C, D, E (general-purpose), H:L (pointer)
- **4 GP registers for locals** — B, C, D, E only; H:L are reserved for memory access
- **8-level push-down stack** — 7 usable call levels (one always occupied by the PC)
- **Port I/O** — 8 input ports (INP), 24 output ports (OUT) encoded in opcode
- **Carry arithmetic** — CY flag exposed via adc(), sbb(), carry() intrinsics
- **Byte rotations** — RLC, RRC, RAL, RAR exposed as rlc(), rrc(), ral(), rar()

Oct is the sister language to Nib (which targets the 4-bit Intel 4004). Where
Nib operates in nibbles, Oct operates in full bytes — the step up in word size
that the 8008 brought over its predecessor.

This package loads ``oct.tokens`` from the ``code/grammars/`` directory and
uses the same ``GrammarLexer`` engine that powers the Nib, ALGOL 60, JSON,
Python, and Starlark lexers. One lexer engine handles radically different
languages — from 1960 ALGOL to a 2024 embedded toy language.

Usage::

    from oct_lexer import tokenize_oct

    tokens = tokenize_oct('let x: u8 = 0xFF;')
    for token in tokens:
        print(token)
"""

from __future__ import annotations

from oct_lexer.tokenizer import create_oct_lexer, tokenize_oct

__all__ = [
    "create_oct_lexer",
    "tokenize_oct",
]
