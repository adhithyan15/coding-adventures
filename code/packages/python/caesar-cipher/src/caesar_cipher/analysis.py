"""analysis.py -- Breaking the Caesar Cipher
============================================

The Caesar cipher is trivially breakable.  There are only 25 possible
non-identity shifts (1 through 25), so an attacker can simply try all of
them.  Even better, the attacker can use *frequency analysis* -- comparing
letter frequencies in the ciphertext to known English letter frequencies --
to guess the correct shift without reading all 25 candidates.

This module implements two attack strategies:

1. **Brute force** -- try every possible shift and return all results.
2. **Frequency analysis** -- statistically determine the most likely shift.

Why the Caesar Cipher Is Insecure
---------------------------------

The Caesar cipher has a *key space* of only 26 (including the identity
shift of 0).  A human can try all 25 non-trivial shifts by hand in
minutes.  A computer can do it in microseconds.

Worse, because the cipher only shifts letters (it does not *rearrange*
them), the *frequency distribution* of letters in the ciphertext is
identical to the distribution in the plaintext -- just shifted.  English
has very distinctive letter frequencies:

    Most common: E (12.7%), T (9.1%), A (8.2%), O (7.5%), I (7.0%)
    Least common: Z (0.07%), Q (0.10%), X (0.15%), J (0.15%)

By counting letter frequencies in the ciphertext and comparing them to
the expected English frequencies, we can determine the shift with high
accuracy -- even for moderately short texts.

Frequency Analysis: The Chi-Squared Method
------------------------------------------

The chi-squared (chi^2) statistic measures how well an *observed*
frequency distribution matches an *expected* distribution:

    chi^2 = sum( (observed_i - expected_i)^2 / expected_i )

For each candidate shift (0-25), we:
    1. "Decrypt" the ciphertext using that shift.
    2. Count the letter frequencies in the decrypted text.
    3. Compare those frequencies to the expected English frequencies
       using the chi-squared formula.

The shift that produces the *lowest* chi-squared value is the one that
makes the letter frequencies most closely match English -- and is therefore
the most likely correct decryption.

Worked Example
--------------

Suppose we have ciphertext "KHOOR" and want to find the shift.

For shift=3 (the correct one):
    Decrypted: "HELLO"
    Letter counts: H=1, E=1, L=2, O=1
    These frequencies match common English letters well -> low chi^2.

For shift=7 (wrong):
    Decrypted: "DAHHK"
    Letter counts: D=1, A=1, H=2, K=1
    These frequencies don't match English as well -> higher chi^2.

The algorithm picks the shift with the lowest chi^2.
"""

from __future__ import annotations

from caesar_cipher.cipher import encrypt

# ---------------------------------------------------------------------------
# English Letter Frequencies
# ---------------------------------------------------------------------------
#
# These values represent the relative frequency of each letter in a large
# corpus of English text, expressed as proportions (0.0 to 1.0).
#
# Source: standard English letter frequency tables (e.g., Lewand, 2000).
#
# These frequencies are the "fingerprint" of English.  Every language has
# its own characteristic frequency distribution.  French, for example,
# has a very high frequency for 'E' (similar to English) but different
# patterns for other letters.

ENGLISH_FREQUENCIES: dict[str, float] = {
    "a": 0.08167,
    "b": 0.01492,
    "c": 0.02782,
    "d": 0.04253,
    "e": 0.12702,
    "f": 0.02228,
    "g": 0.02015,
    "h": 0.06094,
    "i": 0.06966,
    "j": 0.00153,
    "k": 0.00772,
    "l": 0.04025,
    "m": 0.02406,
    "n": 0.06749,
    "o": 0.07507,
    "p": 0.01929,
    "q": 0.00095,
    "r": 0.05987,
    "s": 0.06327,
    "t": 0.09056,
    "u": 0.02758,
    "v": 0.00978,
    "w": 0.02360,
    "x": 0.00150,
    "y": 0.01974,
    "z": 0.00074,
}

# Sanity check: frequencies should sum to approximately 1.0.
# (They sum to ~0.99999 due to rounding, which is fine.)
_FREQ_SUM = sum(ENGLISH_FREQUENCIES.values())
assert 0.99 < _FREQ_SUM < 1.01, f"Frequencies sum to {_FREQ_SUM}, expected ~1.0"

# ---------------------------------------------------------------------------
# Brute Force Attack
# ---------------------------------------------------------------------------


def brute_force(ciphertext: str) -> list[tuple[int, str]]:
    """Try all 25 non-trivial shifts and return every possible decryption.

    Since the Caesar cipher only has 26 possible keys (shifts 0-25), and
    shift=0 produces the original ciphertext, we try shifts 1 through 25.
    A human can then visually inspect the results to find the one that
    produces readable English.

    Parameters
    ----------
    ciphertext : str
        The encrypted text to break.

    Returns
    -------
    list[tuple[int, str]]
        A list of 25 ``(shift, plaintext)`` tuples, one for each shift
        from 1 to 25.

    Examples
    --------
    >>> results = brute_force("KHOOR")
    >>> len(results)
    25
    >>> # The correct decryption (shift=3) should be in there:
    >>> any(text == "HELLO" for shift, text in results)
    True
    """
    # Try every shift from 1 to 25.  Shift 0 is the identity (no change),
    # so we skip it -- the attacker already has the ciphertext.
    return [(shift, encrypt(ciphertext, -shift)) for shift in range(1, 26)]


# ---------------------------------------------------------------------------
# Frequency Analysis Attack
# ---------------------------------------------------------------------------


def _count_letters(text: str) -> dict[str, int]:
    """Count occurrences of each letter (case-insensitive) in *text*.

    Non-alphabetic characters are ignored.

    Parameters
    ----------
    text : str
        The text to analyze.

    Returns
    -------
    dict[str, int]
        A dictionary mapping each lowercase letter to its count.

    Examples
    --------
    >>> counts = _count_letters("Hello, World!")
    >>> counts['l']
    3
    >>> counts['z']
    0
    """
    counts: dict[str, int] = {chr(c): 0 for c in range(ord("a"), ord("z") + 1)}
    for ch in text.lower():
        if ch.isalpha():
            counts[ch] += 1
    return counts


def _chi_squared(observed_counts: dict[str, int], total: int) -> float:
    """Compute the chi-squared statistic comparing observed letter counts
    to the expected English letter frequency distribution.

    The chi-squared formula for each letter is:

        chi^2_i = (observed_i - expected_i)^2 / expected_i

    where:
        - observed_i = the count of letter i in the text
        - expected_i = total_letters * english_frequency_of_letter_i

    The total chi-squared is the sum over all 26 letters.

    A *lower* chi-squared value means the observed distribution is *closer*
    to English.  A *higher* value means it's further away.

    Parameters
    ----------
    observed_counts : dict[str, int]
        Letter counts from ``_count_letters``.
    total : int
        Total number of letters in the text.

    Returns
    -------
    float
        The chi-squared statistic.  Lower is better (closer to English).
    """
    if total == 0:
        return 0.0

    chi2 = 0.0
    for letter, expected_freq in ENGLISH_FREQUENCIES.items():
        observed = observed_counts.get(letter, 0)
        expected = total * expected_freq
        # Guard against division by zero for extremely rare letters
        if expected > 0:
            chi2 += (observed - expected) ** 2 / expected
    return chi2


def frequency_analysis(ciphertext: str) -> tuple[int, str]:
    """Determine the most likely Caesar cipher shift using frequency analysis.

    This function tries all 26 shifts (0-25), decrypts the ciphertext with
    each one, computes the chi-squared statistic comparing the resulting
    letter frequencies to English, and returns the shift that produces the
    best match.

    How it works step by step:
        1. For shift = 0, 1, 2, ..., 25:
           a. Decrypt the ciphertext using this shift.
           b. Count the letter frequencies in the decrypted text.
           c. Compute chi-squared against English frequencies.
        2. Return the shift with the *lowest* chi-squared value.

    This works because the Caesar cipher preserves letter frequency patterns.
    The only shift that aligns the ciphertext's frequencies with English is
    the correct one.

    Parameters
    ----------
    ciphertext : str
        The encrypted text to analyze.

    Returns
    -------
    tuple[int, str]
        A ``(shift, plaintext)`` tuple where *shift* is the estimated
        encryption shift and *plaintext* is the decrypted text.

    Notes
    -----
    - For very short texts (< 20 characters), frequency analysis may be
      unreliable because the sample size is too small.
    - For non-English text, the results will be meaningless because the
      expected frequency table is for English.
    - An empty ciphertext returns ``(0, "")``.

    Examples
    --------
    >>> plaintext = "the quick brown fox jumps over the lazy dog"
    >>> from caesar_cipher.cipher import encrypt
    >>> ciphertext = encrypt(plaintext, 7)
    >>> shift, decrypted = frequency_analysis(ciphertext)
    >>> shift
    7
    >>> decrypted == plaintext
    True
    """
    if not ciphertext:
        return (0, "")

    best_shift = 0
    best_chi2 = float("inf")
    best_plaintext = ciphertext

    for shift in range(26):
        # Decrypt with this candidate shift
        candidate = encrypt(ciphertext, -shift)

        # Count letters in the candidate plaintext
        counts = _count_letters(candidate)
        total = sum(counts.values())

        # Compute chi-squared against English frequencies
        chi2 = _chi_squared(counts, total)

        # Keep the shift with the lowest chi-squared (best match to English)
        if chi2 < best_chi2:
            best_chi2 = chi2
            best_shift = shift
            best_plaintext = candidate

    return (best_shift, best_plaintext)
