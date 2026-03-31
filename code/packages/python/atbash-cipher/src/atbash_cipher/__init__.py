"""atbash-cipher -- Atbash cipher: fixed reverse-alphabet substitution, self-inverse.

The Atbash cipher is one of the oldest known ciphers, originally used with
the Hebrew alphabet. The name "Atbash" comes from the first, last, second,
and second-to-last letters of the Hebrew alphabet: Aleph-Tav-Beth-Shin.

The core idea is beautifully simple: reverse the alphabet. A maps to Z,
B maps to Y, C maps to X, and so on. Mathematically, if we assign each
letter a position (A=0, B=1, ..., Z=25), the encrypted position is:

    encrypted_position = 25 - original_position

This gives us the mapping:

    A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
    Z Y X W V U T S R Q P O N M L K J I H G F E D C B A

A remarkable property of the Atbash cipher is that it is *self-inverse*:
applying the cipher twice returns the original text. This is because
reversing the alphabet twice gets you back to where you started:

    encrypt(encrypt("HELLO")) == "HELLO"

This means the same function works for both encryption and decryption.

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.
"""

__version__ = "0.1.0"

from atbash_cipher.cipher import decrypt, encrypt

__all__ = ["encrypt", "decrypt", "__version__"]
