"""caesar-cipher -- the oldest substitution cipher.

Includes brute-force and frequency analysis for breaking it.

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.

Public API
----------
- ``encrypt(text, shift)`` -- encrypt plaintext using a Caesar shift
- ``decrypt(text, shift)`` -- decrypt ciphertext using a Caesar shift
- ``rot13(text)`` -- apply ROT13 (shift=13, self-inverse)
- ``brute_force(ciphertext)`` -- try all 25 possible shifts
- ``frequency_analysis(ciphertext)`` -- guess shift via English letter frequencies
- ``ENGLISH_FREQUENCIES`` -- expected letter frequency distribution for English
"""

from __future__ import annotations

from caesar_cipher.analysis import (
    ENGLISH_FREQUENCIES,
    brute_force,
    frequency_analysis,
)
from caesar_cipher.cipher import decrypt, encrypt, rot13

__version__ = "0.1.0"

__all__ = [
    "ENGLISH_FREQUENCIES",
    "__version__",
    "brute_force",
    "decrypt",
    "encrypt",
    "frequency_analysis",
    "rot13",
]
