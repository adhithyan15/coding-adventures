"""vigenere-cipher -- Vigenere polyalphabetic substitution cipher.

The Vigenere cipher is one of the most famous historical ciphers. Invented
by Giovan Battista Bellaso in 1553 (and misattributed to Blaise de Vigenere),
it was considered "le chiffre indechiffrable" (the undecipherable cipher) for
nearly 300 years until Friedrich Kasiski published a general attack in 1863.

How It Works
------------

Unlike the Caesar cipher, which shifts every letter by the SAME amount,
the Vigenere cipher shifts each letter by a DIFFERENT amount determined by
a repeating keyword:

    Plaintext:  A T T A C K A T D A W N
    Keyword:    L E M O N L E M O N L E
    Shift:      11 4 12 14 13 11 4 12 14 13 11 4
    Ciphertext: L X F O P V E F R N H R

Each letter of the keyword maps to a shift value: A=0, B=1, ..., Z=25.
The keyword repeats cyclically, but ONLY advances on alphabetic characters
(spaces and punctuation don't consume a keyword position).

Why It Matters
--------------

The Vigenere cipher bridges single-key substitution (Caesar) and modern
polyalphabetic systems. Its cryptanalysis -- using Index of Coincidence
and chi-squared frequency analysis -- introduces statistical techniques
that remain foundational in modern cryptography.

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.
"""

__version__ = "0.1.0"

from vigenere_cipher.analysis import break_cipher, find_key, find_key_length
from vigenere_cipher.cipher import decrypt, encrypt

__all__ = [
    "encrypt",
    "decrypt",
    "find_key_length",
    "find_key",
    "break_cipher",
    "__version__",
]
