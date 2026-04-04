"""cipher.py -- Core Scytale cipher implementation.

The Scytale Cipher
==================

The Scytale cipher is a *transposition* cipher. Unlike substitution ciphers
(Caesar, Atbash) which replace each character with a different character,
the Scytale rearranges the order of characters without changing them.

Think of it like shuffling a deck of cards — the same cards are there,
just in a different order.

How Encryption Works
--------------------

Given a plaintext and a key (number of columns):

1. Write the text into a grid with `key` columns, filling row by row.
2. If the last row is incomplete, pad it with spaces.
3. Read the grid column by column to produce the ciphertext.

Example: encrypt("HELLO WORLD", 3)

    Step 1 — Calculate grid dimensions:
        text length = 11, key = 3 columns
        rows = ceil(11 / 3) = 4
        padded length = 4 * 3 = 12

    Step 2 — Pad and fill the grid row by row:

        "HELLO WORLD" + " " (1 space of padding)

             col0  col1  col2
        row0:  H     E     L
        row1:  L     O     ' '
        row2:  W     O     R
        row3:  L     D     ' '

    Step 3 — Read column by column:

        col0: H, L, W, L  →  "HLWL"
        col1: E, O, O, D  →  "EOOD"
        col2: L, ' ', R, ' '  →  "L R "

    Result: "HLWLEOODL R "

How Decryption Works
--------------------

Decryption reverses the process:

1. Calculate the number of rows: ceil(len(ciphertext) / key).
2. Write the ciphertext into the grid column by column.
3. Read the grid row by row.
4. Strip any trailing padding spaces that were added during encryption.

Example: decrypt("HLWLEOODL R ", 3)

    Step 1 — Calculate grid:
        length = 12, key = 3 columns
        rows = ceil(12 / 3) = 4

    Step 2 — Fill column by column (each column has 4 chars):

        col0 (chars 0-3):  H, L, W, L
        col1 (chars 4-7):  E, O, O, D
        col2 (chars 8-11): L, ' ', R, ' '

    Step 3 — Read row by row:

        row0: H, E, L       →  "HEL"
        row1: L, O, ' '     →  "LO "
        row2: W, O, R       →  "WOR"
        row3: L, D, ' '     →  "LD "

    Combined: "HELLO WORLD "
    After stripping trailing pad: "HELLO WORLD"

Why It's Insecure
-----------------

The key space is tiny: for a message of length n, there are only
about n/2 possible keys (from 2 to n/2). An attacker can try every
key and look for readable text. This is what brute_force() does.
"""

import math


def encrypt(text: str, key: int) -> str:
    """Encrypt text using the Scytale transposition cipher.

    The text is written row-by-row into a grid with `key` columns,
    then read column-by-column to produce the ciphertext.

    Args:
        text: The plaintext string to encrypt. All characters are preserved.
        key: The number of columns (rod circumference). Must be >= 2.

    Returns:
        The transposed ciphertext string.

    Raises:
        ValueError: If key < 2 or key > len(text).

    Examples:
        >>> encrypt("HELLO WORLD", 3)
        'HLWLEOODL R '
        >>> encrypt("ABCDEF", 2)
        'ACEBDF'
        >>> encrypt("ABCDEF", 3)
        'ADBECF'
    """
    # --- Validate inputs ---
    if not text:
        return ""
    if key < 2:
        msg = f"Key must be >= 2, got {key}"
        raise ValueError(msg)
    if key > len(text):
        msg = f"Key must be <= text length ({len(text)}), got {key}"
        raise ValueError(msg)

    # --- Step 1: Calculate grid dimensions ---
    # The number of rows is ceil(text_length / key).
    # We pad the text with spaces so it fills a complete grid.
    num_rows = math.ceil(len(text) / key)
    padded_length = num_rows * key
    padded_text = text.ljust(padded_length)  # Pad with spaces on the right

    # --- Step 2: Read column by column ---
    # Column c contains characters at positions: c, c+key, c+2*key, ...
    # This is the core of the transposition: we're reading the grid
    # in a different order than it was written.
    result: list[str] = []
    for col in range(key):
        for row in range(num_rows):
            result.append(padded_text[row * key + col])

    return "".join(result)


def decrypt(text: str, key: int) -> str:
    """Decrypt ciphertext that was encrypted with the Scytale cipher.

    The ciphertext is written column-by-column into a grid, then read
    row-by-row to recover the original plaintext. Trailing padding
    spaces are stripped.

    Args:
        text: The ciphertext string to decrypt.
        key: The number of columns used during encryption. Must be >= 2.

    Returns:
        The decrypted plaintext string (trailing pad spaces stripped).

    Raises:
        ValueError: If key < 2 or key > len(text).

    Examples:
        >>> decrypt("HLWLEOODL R ", 3)
        'HELLO WORLD'
        >>> decrypt(encrypt("secret message", 5), 5)
        'secret message'
    """
    # --- Validate inputs ---
    if not text:
        return ""
    if key < 2:
        msg = f"Key must be >= 2, got {key}"
        raise ValueError(msg)
    if key > len(text):
        msg = f"Key must be <= text length ({len(text)}), got {key}"
        raise ValueError(msg)

    # --- Step 1: Calculate grid dimensions ---
    n = len(text)
    num_rows = math.ceil(n / key)

    # --- Step 2: Write ciphertext into columns ---
    # During encryption, each column got `num_rows` characters — but only
    # if the text length is a perfect multiple of the key. When it's not
    # (which happens during brute-force with a "wrong" key), some columns
    # are shorter.
    #
    # Specifically, if n = num_rows * key - remainder, then:
    #   - The first (key - remainder) columns have num_rows characters
    #   - The last (remainder) columns have (num_rows - 1) characters
    #
    # But wait: properly encrypted text is always padded to num_rows * key,
    # so len(text) % key == 0. For brute-force with arbitrary keys,
    # we handle the general case where columns may differ in length.
    #
    # Number of "full" columns (with num_rows chars):
    full_cols = n % key if n % key != 0 else key
    # Remaining columns have (num_rows - 1) chars — but only if there's
    # actually a remainder. If n % key == 0, all columns have num_rows.

    # Build a list of column start indices and lengths
    # col_start[c] = starting index of column c in the ciphertext
    # col_len[c] = number of characters in column c
    if n % key == 0:
        # All columns have exactly num_rows characters
        col_lengths = [num_rows] * key
    else:
        # First `full_cols` columns have num_rows chars,
        # remaining columns have (num_rows - 1) chars
        col_lengths = [num_rows] * full_cols + [num_rows - 1] * (key - full_cols)

    # Compute starting index for each column
    col_starts: list[int] = []
    offset = 0
    for length in col_lengths:
        col_starts.append(offset)
        offset += length

    # Read row-by-row: character at grid position (row, col) is at
    # ciphertext index col_starts[col] + row
    result: list[str] = []
    for row in range(num_rows):
        for col in range(key):
            if row < col_lengths[col]:
                result.append(text[col_starts[col] + row])

    # --- Step 3: Strip trailing padding ---
    # During encryption, we may have added spaces to fill the last row.
    # We strip those trailing spaces now.
    return "".join(result).rstrip(" ")


def brute_force(text: str) -> list[dict[str, int | str]]:
    """Try all possible Scytale keys and return the decrypted results.

    The Scytale cipher has a very small key space: for a message of
    length n, there are only floor(n/2) - 1 possible keys (from 2 to
    floor(n/2)). This function demonstrates why a small key space makes
    a cipher trivially breakable.

    Args:
        text: The ciphertext to brute-force.

    Returns:
        A list of dictionaries, each with:
        - "key": The key that was tried (int).
        - "text": The decrypted text for that key (str).

    Examples:
        >>> results = brute_force("ACEBDF")
        >>> results[0]
        {'key': 2, 'text': 'ABCDEF'}
    """
    if len(text) < 4:
        return []

    results: list[dict[str, int | str]] = []
    max_key = len(text) // 2
    for candidate_key in range(2, max_key + 1):
        decrypted = decrypt(text, candidate_key)
        results.append({"key": candidate_key, "text": decrypted})
    return results
