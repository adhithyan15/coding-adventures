"""scytale-cipher -- Scytale cipher: ancient Spartan transposition cipher.

The Scytale (pronounced "SKIT-ah-lee") cipher is one of the earliest known
transposition ciphers, used by the Spartans around 700 BCE for military
communication. Unlike substitution ciphers (which replace characters),
transposition ciphers rearrange the positions of characters in the message.

The physical Scytale was a wooden rod of specific diameter. A strip of
leather or parchment was wrapped around the rod, and the message was written
along the length. When unwrapped, the letters appeared scrambled. Only
someone with a rod of the same diameter (the "key") could read the message.

Mathematically, the Scytale is equivalent to a columnar transposition:

    Encrypt: write text row-by-row into a grid, read column-by-column.
    Decrypt: write text column-by-column into a grid, read row-by-row.

The key is the number of columns in the grid (equivalently, the
circumference of the rod).

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.
"""

__version__ = "0.1.0"

from scytale_cipher.cipher import brute_force, decrypt, encrypt

__all__ = ["encrypt", "decrypt", "brute_force", "__version__"]
