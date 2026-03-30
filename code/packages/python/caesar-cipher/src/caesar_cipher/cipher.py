"""cipher.py -- Caesar Cipher Encryption and Decryption
=====================================================

The Caesar cipher is the simplest and oldest known substitution cipher.
Named after Julius Caesar, who reportedly used it with a shift of 3 to
communicate with his generals, it works by shifting every letter in the
alphabet by a fixed number of positions.

How It Works
------------

Imagine the alphabet laid out in a circle (a "ring"):

    A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
    |                                                   |
    +---------------------------------------------------+
                    (wraps around)

To encrypt with shift=3, slide every letter 3 positions to the right:

    Plaintext alphabet:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
    Ciphertext alphabet: D E F G H I J K L M N O P Q R S T U V W X Y Z A B C

So 'A' -> 'D', 'B' -> 'E', ..., 'X' -> 'A', 'Y' -> 'B', 'Z' -> 'C'.

Worked Example
--------------

Encrypting "HELLO" with shift=3:

    H -> K  (H is position 7, 7+3=10, which is K)
    E -> H  (E is position 4, 4+3=7,  which is H)
    L -> O  (L is position 11, 11+3=14, which is O)
    L -> O  (same as above)
    O -> R  (O is position 14, 14+3=17, which is R)

    Result: "KHOOR"

The Math
--------

For a letter at position ``p`` (where A=0, B=1, ..., Z=25):

    encrypted_position = (p + shift) mod 26
    decrypted_position = (p - shift) mod 26

The ``mod 26`` operation is what makes the alphabet "wrap around".

Truth Table (shift=3, first 8 letters)
--------------------------------------

    +-----------+----------+------------+-----------+
    | Plaintext | Position | +3 mod 26  | Ciphertext|
    +-----------+----------+------------+-----------+
    | A         | 0        | 3          | D         |
    | B         | 1        | 4          | E         |
    | C         | 2        | 5          | F         |
    | D         | 3        | 6          | G         |
    | E         | 4        | 7          | H         |
    | F         | 5        | 8          | I         |
    | G         | 6        | 9          | J         |
    | H         | 7        | 10         | K         |
    +-----------+----------+------------+-----------+

Non-alphabetic characters (digits, punctuation, spaces) are passed through
unchanged. This preserves the structure of the message while only
transforming the letters.

Case Preservation
-----------------

Uppercase letters stay uppercase, lowercase letters stay lowercase.
We handle this by detecting case, converting to a 0-25 position,
applying the shift, then converting back with the original case.
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# The number of letters in the English alphabet. Every shift operation is
# performed modulo this value so the alphabet "wraps around".
_ALPHABET_SIZE: int = 26


# ---------------------------------------------------------------------------
# Core shift function (the heart of the cipher)
# ---------------------------------------------------------------------------


def _shift_char(ch: str, shift: int) -> str:
    """Shift a single character by *shift* positions in the alphabet.

    This is the fundamental building block of the Caesar cipher.  It handles
    three cases:

    1. **Uppercase letter** (A-Z) -- shift within the uppercase range.
    2. **Lowercase letter** (a-z) -- shift within the lowercase range.
    3. **Everything else** -- return unchanged (digits, spaces, punctuation).

    The algorithm:
        1. Find the character's position (0-25) by subtracting the ASCII code
           of 'A' (for uppercase) or 'a' (for lowercase).
        2. Add the shift and take modulo 26 to wrap around.
        3. Convert back to a character by adding the base ASCII code.

    Parameters
    ----------
    ch : str
        A single character to shift.
    shift : int
        Number of positions to shift (can be negative for decryption).

    Returns
    -------
    str
        The shifted character, or the original if non-alphabetic.

    Examples
    --------
    >>> _shift_char('A', 3)
    'D'
    >>> _shift_char('z', 1)
    'a'
    >>> _shift_char('5', 10)
    '5'
    """
    if ch.isupper():
        # ord('A') = 65.  Subtract to get position 0-25, shift, wrap, add back.
        return chr((ord(ch) - ord("A") + shift) % _ALPHABET_SIZE + ord("A"))
    if ch.islower():
        # ord('a') = 97.  Same logic, lowercase range.
        return chr((ord(ch) - ord("a") + shift) % _ALPHABET_SIZE + ord("a"))
    # Non-alphabetic characters pass through unchanged.
    return ch


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def encrypt(text: str, shift: int) -> str:
    """Encrypt *text* using the Caesar cipher with the given *shift*.

    Each letter in the text is replaced by a letter a fixed number of
    positions further along in the alphabet.  Non-alphabetic characters
    (digits, punctuation, whitespace) are left unchanged.

    Parameters
    ----------
    text : str
        The plaintext message to encrypt.
    shift : int
        The number of positions to shift each letter.  Can be any integer;
        negative values shift left, values >= 26 wrap around.

    Returns
    -------
    str
        The encrypted ciphertext.

    Examples
    --------
    >>> encrypt("HELLO", 3)
    'KHOOR'

    >>> encrypt("attack at dawn", 13)
    'nggnpx ng qnja'

    >>> encrypt("abc XYZ 123!", 1)
    'bcd YZA 123!'

    Shift wrapping -- a shift of 26 is the same as no shift at all:

    >>> encrypt("test", 26)
    'test'
    """
    return "".join(_shift_char(ch, shift) for ch in text)


def decrypt(text: str, shift: int) -> str:
    """Decrypt *text* that was encrypted with the Caesar cipher.

    Decryption is simply encryption with the *negated* shift.  If a message
    was encrypted with shift=3, we decrypt with shift=3 by internally
    applying shift=-3.

    This works because:

        encrypt(decrypt(text, s), s) == text
        (p + s - s) mod 26 == p mod 26

    Parameters
    ----------
    text : str
        The ciphertext to decrypt.
    shift : int
        The shift that was used during encryption.

    Returns
    -------
    str
        The decrypted plaintext.

    Examples
    --------
    >>> decrypt("KHOOR", 3)
    'HELLO'

    >>> decrypt(encrypt("round trip", 7), 7)
    'round trip'
    """
    return encrypt(text, -shift)


def rot13(text: str) -> str:
    """Apply ROT13 to *text*.

    ROT13 is a special case of the Caesar cipher where the shift is exactly
    13 -- half the alphabet.  This makes it *self-inverse*: applying ROT13
    twice returns the original text.

    Why is shift=13 special?
    ~~~~~~~~~~~~~~~~~~~~~~~~

    The alphabet has 26 letters.  26 / 2 = 13.  So shifting by 13 and then
    shifting by 13 again gives a total shift of 26, which is the same as
    no shift at all:

        ROT13(ROT13(text)) == text
        (p + 13 + 13) mod 26 == (p + 26) mod 26 == p

    ROT13 Substitution Table
    ~~~~~~~~~~~~~~~~~~~~~~~~

        Input:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
        Output: N O P Q R S T U V W X Y Z A B C D E F G H I J K L M

    Notice that the first half (A-M) maps to the second half (N-Z) and
    vice versa.  This symmetry is what makes ROT13 self-inverse.

    Parameters
    ----------
    text : str
        The text to transform.

    Returns
    -------
    str
        The ROT13-transformed text.

    Examples
    --------
    >>> rot13("Hello, World!")
    'Uryyb, Jbeyq!'

    >>> rot13(rot13("Hello, World!"))
    'Hello, World!'
    """
    return encrypt(text, 13)
