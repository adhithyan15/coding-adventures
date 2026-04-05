"""cipher.py -- Core Vigenere cipher encryption and decryption.

The Vigenere Cipher: A Polyalphabetic Substitution
===================================================

The Caesar cipher uses a single shift for every letter. The Vigenere cipher
uses a KEYWORD that provides a different shift for each position.

Think of it like having 26 different Caesar ciphers and cycling through them
based on the keyword letters.

The Shift Mapping
-----------------

Each letter of the keyword maps to a numeric shift:

    A -> 0    B -> 1    C -> 2    ... Z -> 25

So the keyword "LEMON" gives shifts: [11, 4, 12, 14, 13].

Encryption Step-by-Step
-----------------------

For each character in the plaintext:
  - If it's a letter, shift it forward by the current keyword letter's value.
  - If it's not a letter (space, digit, punctuation), pass it through unchanged.
  - The keyword position ONLY advances when we process a letter.

Example: encrypt("Hello, World!", "key")

    Position in keyword starts at 0.

    H -> shift by K(=10) -> R    (keyword advances to position 1)
    e -> shift by E(=4)  -> i    (keyword advances to position 2)
    l -> shift by Y(=24) -> j    (keyword advances to position 0)
    l -> shift by K(=10) -> v    (keyword advances to position 1)
    o -> shift by E(=4)  -> s    (keyword advances to position 2)
    , -> pass through    -> ,    (keyword does NOT advance)
    ' '-> pass through   -> ' '  (keyword does NOT advance)
    W -> shift by Y(=24) -> U    (keyword advances to position 0)
    o -> shift by K(=10) -> y    (keyword advances to position 1)
    r -> shift by E(=4)  -> v    (keyword advances to position 2)
    l -> shift by Y(=24) -> j    (keyword advances to position 0)
    d -> shift by K(=10) -> n    (keyword advances to position 1)
    ! -> pass through    -> !    (keyword does NOT advance)

    Result: "Rijvs, Uyvjn!"

Notice: uppercase stays uppercase, lowercase stays lowercase.

Decryption
----------

Decryption is exactly the same process, but we shift BACKWARD instead of
forward. Mathematically:

    encrypt: ciphertext[i] = (plaintext[i] + key[j]) mod 26
    decrypt: plaintext[i]  = (ciphertext[i] - key[j]) mod 26

where j is the keyword position (only incremented on alphabetic characters).
"""


def _validate_key(key: str) -> None:
    """Validate that the key is non-empty and contains only letters.

    Args:
        key: The keyword string to validate.

    Raises:
        ValueError: If the key is empty or contains non-alphabetic characters.
    """
    if not key:
        msg = "Key must not be empty"
        raise ValueError(msg)
    if not key.isalpha():
        msg = f"Key must contain only letters, got {key!r}"
        raise ValueError(msg)


def encrypt(plaintext: str, key: str) -> str:
    """Encrypt plaintext using the Vigenere cipher.

    Each letter in the plaintext is shifted forward by the corresponding
    keyword letter (A=0, B=1, ..., Z=25). Non-alphabetic characters pass
    through unchanged and do not advance the keyword position.

    Args:
        plaintext: The text to encrypt. Can contain any characters.
        key: The keyword (must be non-empty, alphabetic only).

    Returns:
        The encrypted ciphertext with case preserved.

    Raises:
        ValueError: If key is empty or contains non-alphabetic characters.

    Examples:
        >>> encrypt("ATTACKATDAWN", "LEMON")
        'LXFOPVEFRNHR'
        >>> encrypt("Hello, World!", "key")
        'Rijvs, Uyvjn!'
    """
    _validate_key(key)

    # Convert the key to uppercase shift values (A=0, B=1, ..., Z=25).
    # We work in uppercase internally so the key is case-insensitive.
    key_shifts = [ord(c.upper()) - ord("A") for c in key]
    key_len = len(key_shifts)

    result: list[str] = []
    key_index = 0  # Tracks our position in the keyword

    for char in plaintext:
        if char.isalpha():
            # Determine the base: 'A' for uppercase, 'a' for lowercase
            base = ord("A") if char.isupper() else ord("a")

            # The shift formula: (letter_position + key_shift) mod 26
            # This wraps around the alphabet: Z + 1 = A
            shifted = (ord(char) - base + key_shifts[key_index % key_len]) % 26
            result.append(chr(base + shifted))

            # Only advance the keyword on alphabetic characters
            key_index += 1
        else:
            # Non-alpha: pass through unchanged, don't advance keyword
            result.append(char)

    return "".join(result)


def decrypt(ciphertext: str, key: str) -> str:
    """Decrypt ciphertext that was encrypted with the Vigenere cipher.

    Each letter in the ciphertext is shifted BACKWARD by the corresponding
    keyword letter. This is the exact reverse of encryption.

    Args:
        ciphertext: The encrypted text to decrypt.
        key: The keyword used during encryption.

    Returns:
        The decrypted plaintext with case preserved.

    Raises:
        ValueError: If key is empty or contains non-alphabetic characters.

    Examples:
        >>> decrypt("LXFOPVEFRNHR", "LEMON")
        'ATTACKATDAWN'
        >>> decrypt("Rijvs, Uyvjn!", "key")
        'Hello, World!'
    """
    _validate_key(key)

    # Same key preparation as encrypt
    key_shifts = [ord(c.upper()) - ord("A") for c in key]
    key_len = len(key_shifts)

    result: list[str] = []
    key_index = 0

    for char in ciphertext:
        if char.isalpha():
            base = ord("A") if char.isupper() else ord("a")

            # Decrypt: subtract instead of add. The +26 ensures we never
            # go negative before taking mod 26.
            #   encrypt: (p + k) mod 26 = c
            #   decrypt: (c - k + 26) mod 26 = p
            shifted = (ord(char) - base - key_shifts[key_index % key_len] + 26) % 26
            result.append(chr(base + shifted))

            key_index += 1
        else:
            result.append(char)

    return "".join(result)
