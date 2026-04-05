"""analysis.py -- Cryptanalysis tools for breaking the Vigenere cipher.

Breaking the "Unbreakable" Cipher
==================================

For 300 years, the Vigenere cipher was considered unbreakable. The key
insight that defeated it: if you know the KEY LENGTH, the cipher reduces
to multiple independent Caesar ciphers -- and Caesar is trivially broken
by frequency analysis.

The attack has two phases:

Phase 1: Find the Key Length (Index of Coincidence)
----------------------------------------------------

The Index of Coincidence (IC) measures how likely it is that two randomly
chosen letters from a text are the same letter. For English text:

    IC_english ~ 0.0667  (letters are unevenly distributed: lots of E, T, A)
    IC_random  ~ 0.0385  (all 26 letters equally likely: 1/26)

The formula for IC of a sequence of N letters with frequency counts f_i:

    IC = sum(f_i * (f_i - 1)) / (N * (N - 1))

Why does this help? When we guess the CORRECT key length k and group
every k-th letter together, each group was encrypted with a SINGLE Caesar
shift. The letter frequencies within each group look like shifted English
-- so the IC of each group will be close to 0.0667.

When we guess WRONG, the groups mix letters from different Caesar shifts,
which scrambles the frequencies toward uniform distribution (IC ~ 0.0385).

Algorithm:
  For each candidate key length k = 2, 3, ..., max_length:
    1. Split the ciphertext into k groups (group i = letters at positions
       i, i+k, i+2k, ...)
    2. Compute IC of each group
    3. Average the ICs
  Return the k with the highest average IC.

Phase 2: Find Each Key Letter (Chi-Squared Test)
-------------------------------------------------

Once we know the key length k, we split the ciphertext into k groups.
Each group is a Caesar cipher with an unknown shift. For each group:

  1. Try all 26 possible shifts (0-25)
  2. For each shift, compute the chi-squared statistic against expected
     English letter frequencies
  3. The shift with the LOWEST chi-squared is most likely correct

The chi-squared formula:

    chi2 = sum((observed_i - expected_i)^2 / expected_i)

where observed_i is the count of letter i after applying the trial shift,
and expected_i = N * english_freq_i (N = total letters in group).

A lower chi-squared means the observed distribution is closer to English.

English Letter Frequencies
--------------------------

These are the standard English letter frequencies used for analysis.
They represent the probability of each letter in a large corpus of
English text. Notice the huge variation: E appears ~12.7% of the time,
while Z appears only ~0.07% -- this non-uniformity is what makes
frequency analysis possible.
"""

# --- English letter frequency table ---
# Source: standard English letter frequency corpus.
# Each value is the probability of that letter appearing in English text.
# The sum of all 26 values is approximately 1.0.
ENGLISH_FREQUENCIES: list[float] = [
    0.08167,  # A -- common in articles ("a", "an", "and")
    0.01492,  # B
    0.02782,  # C
    0.04253,  # D
    0.12702,  # E -- the most common letter in English by far
    0.02228,  # F
    0.02015,  # G
    0.06094,  # H -- common in "the", "that", "this"
    0.06966,  # I
    0.00153,  # J -- rare
    0.00772,  # K
    0.04025,  # L
    0.02406,  # M
    0.06749,  # N
    0.07507,  # O
    0.01929,  # P
    0.00095,  # Q -- rarest (almost always followed by U)
    0.05987,  # R
    0.06327,  # S
    0.09056,  # T -- second most common, "the" is the most common word
    0.02758,  # U
    0.00978,  # V
    0.02360,  # W
    0.00150,  # X -- rare
    0.01974,  # Y
    0.00074,  # Z -- rarest letter
]


def _index_of_coincidence(text: str) -> float:
    """Calculate the Index of Coincidence of a string of letters.

    The IC measures how "English-like" a distribution of letters is.
    Higher IC (~0.0667) suggests a monoalphabetic substitution or plain
    English. Lower IC (~0.0385) suggests a random or polyalphabetic mix.

    Formula: IC = sum(f_i * (f_i - 1)) / (N * (N - 1))

    where f_i is the count of the i-th letter and N is the total count.

    Args:
        text: A string of uppercase letters (no spaces or punctuation).

    Returns:
        The IC value as a float. Returns 0.0 if text has fewer than 2 letters.
    """
    n = len(text)
    if n < 2:
        return 0.0

    # Count frequency of each letter A-Z
    counts = [0] * 26
    for ch in text:
        counts[ord(ch) - ord("A")] += 1

    # IC = sum(f * (f-1)) / (N * (N-1))
    numerator = sum(f * (f - 1) for f in counts)
    return numerator / (n * (n - 1))


def _chi_squared(observed_counts: list[int], expected_freqs: list[float]) -> float:
    """Calculate chi-squared statistic between observed counts and expected frequencies.

    Chi-squared measures how well observed data fits an expected distribution.
    Lower values = better fit. We use this to find which Caesar shift makes
    a group of letters look most like English.

    Formula: chi2 = sum((O_i - E_i)^2 / E_i)

    where O_i = observed count, E_i = expected count = N * freq_i.

    Args:
        observed_counts: List of 26 counts (one per letter A-Z).
        expected_freqs: List of 26 expected frequencies (should sum to ~1.0).

    Returns:
        The chi-squared statistic (lower = better fit to expected).
    """
    total = sum(observed_counts)
    if total == 0:
        return float("inf")

    chi2 = 0.0
    for i in range(26):
        expected = total * expected_freqs[i]
        if expected > 0:
            chi2 += (observed_counts[i] - expected) ** 2 / expected

    return chi2


def _extract_alpha_upper(text: str) -> str:
    """Extract only alphabetic characters from text and convert to uppercase.

    This preprocessing step is needed for cryptanalysis -- we only care
    about the letter content, not spaces, digits, or punctuation.

    Args:
        text: Any string.

    Returns:
        A string containing only uppercase letters.
    """
    return "".join(ch.upper() for ch in text if ch.isalpha())


def find_key_length(ciphertext: str, max_length: int = 20) -> int:
    """Estimate the key length of a Vigenere cipher using Index of Coincidence.

    For each candidate key length k, we split the ciphertext into k groups
    (every k-th letter goes into the same group). If k is correct, each
    group is a simple Caesar cipher and its IC will be close to English
    (~0.0667). If k is wrong, the IC will be closer to random (~0.0385).

    Args:
        ciphertext: The encrypted text to analyze.
        max_length: Maximum key length to try (default 20).

    Returns:
        The estimated key length (the k with highest average IC).

    Examples:
        >>> # Text encrypted with key "SECRET" (length 6)
        >>> find_key_length(long_ciphertext)
        6
    """
    letters = _extract_alpha_upper(ciphertext)

    # Compute average IC for each candidate key length
    ic_scores: list[tuple[int, float]] = []

    for k in range(2, max_length + 1):
        # Split into k groups: group i contains letters at positions
        # i, i+k, i+2k, i+3k, ...
        #
        # Example with k=3 and text "ABCDEFGHIJ":
        #   group 0: A, D, G, J  (positions 0, 3, 6, 9)
        #   group 1: B, E, H     (positions 1, 4, 7)
        #   group 2: C, F, I     (positions 2, 5, 8)
        groups = [""] * k
        for i, ch in enumerate(letters):
            groups[i % k] += ch

        # Average IC across all groups
        total_ic = sum(_index_of_coincidence(g) for g in groups)
        avg_ic = total_ic / k
        ic_scores.append((k, avg_ic))

    # Find the best IC value
    best_ic = max(ic for _, ic in ic_scores)

    # Among all key lengths whose IC is within 5% of the best,
    # choose the SHORTEST. This avoids selecting multiples of the
    # true key length (e.g., 12 or 18 instead of 6), since multiples
    # also produce high IC because each sub-group remains monoalphabetic.
    threshold = best_ic * 0.95
    candidates = [(k, ic) for k, ic in ic_scores if ic >= threshold]
    candidates.sort(key=lambda x: x[0])  # Sort by length ascending

    return candidates[0][0]


def find_key(ciphertext: str, key_length: int) -> str:
    """Determine each letter of the key using chi-squared frequency analysis.

    Once we know the key length, we split the ciphertext into `key_length`
    groups. Each group was encrypted with the same Caesar shift. For each
    group, we try all 26 possible shifts and pick the one that produces
    letter frequencies closest to English (lowest chi-squared).

    Args:
        ciphertext: The encrypted text.
        key_length: The known or estimated key length.

    Returns:
        The recovered key as an uppercase string.

    Examples:
        >>> find_key(long_ciphertext, 6)
        'SECRET'
    """
    letters = _extract_alpha_upper(ciphertext)

    key_chars: list[str] = []

    for pos in range(key_length):
        # Extract every key_length-th letter starting at position pos
        group = letters[pos::key_length]

        # Try all 26 shifts and find the one with the lowest chi-squared
        # against English frequencies.
        #
        # Shift of 0 means key letter is 'A' (no shift).
        # Shift of 1 means key letter is 'B' (shift back by 1 to "undo" encryption).
        # ...
        # Shift of 25 means key letter is 'Z'.
        best_shift = 0
        best_chi2 = float("inf")

        for shift in range(26):
            # Shift each letter in the group backward by `shift` positions
            # to simulate decryption with this trial key letter.
            counts = [0] * 26
            for ch in group:
                decrypted_pos = (ord(ch) - ord("A") - shift + 26) % 26
                counts[decrypted_pos] += 1

            chi2 = _chi_squared(counts, ENGLISH_FREQUENCIES)

            if chi2 < best_chi2:
                best_chi2 = chi2
                best_shift = shift

        # The best shift IS the key letter (A=0, B=1, ..., Z=25)
        key_chars.append(chr(ord("A") + best_shift))

    return "".join(key_chars)


def break_cipher(ciphertext: str) -> tuple[str, str]:
    """Automatically break a Vigenere cipher: find the key and decrypt.

    This combines all three steps of the Kasiski/IC attack:
    1. find_key_length: estimate the key length using IC analysis.
    2. find_key: determine each key letter using chi-squared analysis.
    3. decrypt: use the recovered key to decrypt the message.

    This works well on ciphertexts of ~200+ characters. Shorter texts may
    not have enough statistical signal for reliable key recovery.

    Args:
        ciphertext: The Vigenere-encrypted text to break.

    Returns:
        A tuple of (recovered_key, decrypted_plaintext).

    Examples:
        >>> key, plaintext = break_cipher(long_ciphertext)
        >>> print(f"Key: {key}")
        Key: SECRET
    """
    # Import decrypt here to avoid circular import
    from vigenere_cipher.cipher import decrypt

    key_length = find_key_length(ciphertext)
    key = find_key(ciphertext, key_length)
    plaintext = decrypt(ciphertext, key)

    return key, plaintext
