"""cipher.py -- Core Atbash cipher implementation.

The Atbash Cipher
=================

The Atbash cipher works by reversing the position of each letter in the
alphabet. Think of it like reading the alphabet backwards:

    Forward:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
    Reversed: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A

So 'A' (position 0) maps to 'Z' (position 25), 'B' (position 1) maps to
'Y' (position 24), and so on.

The Formula
-----------

For any letter at position `p` (where A=0, B=1, ..., Z=25):

    new_position = 25 - p

For example:
- H is at position 7.  25 - 7  = 18, which is S.
- E is at position 4.  25 - 4  = 21, which is V.
- L is at position 11. 25 - 11 = 14, which is O.
- O is at position 14. 25 - 14 = 11, which is L.

So "HELLO" becomes "SVOOL".

Why It's Self-Inverse
---------------------

If we encrypt 'S' (position 18): 25 - 18 = 7, which is 'H'.
If we encrypt 'V' (position 21): 25 - 21 = 4, which is 'E'.

Encrypting "SVOOL" gives back "HELLO". The cipher undoes itself!
This happens because f(f(x)) = 25 - (25 - x) = x.

Case Preservation
-----------------

We preserve the case of each letter. If the input is 'h' (lowercase),
we compute the Atbash of 'h' and return the result as lowercase 's'.
Non-alphabetic characters (digits, punctuation, spaces) pass through
unchanged.
"""


def _atbash_char(char: str) -> str:
    """Apply the Atbash substitution to a single character.

    The algorithm:
    1. Check if the character is a letter (A-Z or a-z).
    2. If not, return it unchanged (digits, spaces, punctuation pass through).
    3. If it is a letter, find its position in the alphabet (0-25).
    4. Compute the reversed position: 25 - position.
    5. Convert back to a character, preserving the original case.

    Examples:
        >>> _atbash_char('A')
        'Z'
        >>> _atbash_char('z')
        'a'
        >>> _atbash_char('5')
        '5'
    """
    # --- Uppercase letters: A=65, B=66, ..., Z=90 in ASCII ---
    if "A" <= char <= "Z":
        position = ord(char) - ord("A")  # Convert 'A'->0, 'B'->1, ..., 'Z'->25
        new_position = 25 - position  # Reverse: 0->25, 1->24, ..., 25->0
        return chr(ord("A") + new_position)  # Convert back to a letter

    # --- Lowercase letters: a=97, b=98, ..., z=122 in ASCII ---
    if "a" <= char <= "z":
        position = ord(char) - ord("a")  # Convert 'a'->0, 'b'->1, ..., 'z'->25
        new_position = 25 - position  # Reverse: 0->25, 1->24, ..., 25->0
        return chr(ord("a") + new_position)  # Convert back to a letter

    # --- Non-alphabetic characters pass through unchanged ---
    return char


def encrypt(text: str) -> str:
    """Encrypt text using the Atbash cipher.

    Each letter is replaced by its reverse in the alphabet:
    A<->Z, B<->Y, C<->X, etc. Non-alphabetic characters are
    preserved. Case is maintained.

    Because the Atbash cipher is self-inverse, this function is
    identical to decrypt(). Both are provided for API clarity.

    Args:
        text: The plaintext string to encrypt.

    Returns:
        The encrypted string with each letter reversed in the alphabet.

    Examples:
        >>> encrypt("HELLO")
        'SVOOL'
        >>> encrypt("hello")
        'svool'
        >>> encrypt("Hello, World! 123")
        'Svool, Dliow! 123'
    """
    # Apply _atbash_char to every character in the string and join them back.
    # This is a simple character-by-character substitution cipher.
    return "".join(_atbash_char(c) for c in text)


def decrypt(text: str) -> str:
    """Decrypt text using the Atbash cipher.

    Because Atbash is self-inverse (applying it twice returns the original),
    decryption is identical to encryption. This function exists for API
    clarity and readability.

    Args:
        text: The ciphertext string to decrypt.

    Returns:
        The decrypted (original) string.

    Examples:
        >>> decrypt("SVOOL")
        'HELLO'
        >>> decrypt(encrypt("secret message"))
        'secret message'
    """
    # Decryption is the same operation as encryption for Atbash.
    # f(f(x)) = 25 - (25 - x) = x, so applying encrypt twice is identity.
    return encrypt(text)
