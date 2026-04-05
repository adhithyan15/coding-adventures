"""Cryptanalysis helpers -- tools for breaking classical ciphers.

This module provides two statistical measures used in cryptanalysis plus a
table of standard English letter frequencies.

Index of Coincidence (IC)
=========================
The IC measures the probability that two randomly chosen letters from a text
are the same. It was invented by William Friedman in 1922 and is one of the
most important tools in classical cryptanalysis.

  IC = Sum(n_i * (n_i - 1)) / (N * (N - 1))

where n_i is the count of the i-th letter and N is the total letter count.

Key values:
  - English text:  IC ~ 0.0667
  - Random text:   IC ~ 0.0385 (1/26)
  - German text:   IC ~ 0.0762
  - French text:   IC ~ 0.0778

The IC helps determine cipher type:
  - Monoalphabetic (Caesar, substitution): IC stays near the plaintext language
  - Polyalphabetic (Vigenere): IC drops toward random as key length increases

Shannon Entropy
===============
Entropy measures the information content (or "surprise") in a text.
Invented by Claude Shannon in 1948.

  H = -Sum(p_i * log2(p_i))

where p_i is the proportion of the i-th letter.

Key values:
  - Maximum entropy for 26 letters: log2(26) ~ 4.700 bits
  - English text: H ~ 4.0-4.5 bits (redundancy reduces entropy)
  - Perfectly uniform: H = log2(26) ~ 4.700 bits
"""

from __future__ import annotations

import math

from stats.frequency import frequency_count


# ── English Letter Frequencies ──────────────────────────────────────────
# Source: Standard English letter frequency table (Lewand, 2000).
# These are the expected proportions of each letter in a large sample of
# English text. Used as the "expected" distribution in chi-squared tests.
ENGLISH_FREQUENCIES: dict[str, float] = {
    "A": 0.08167,
    "B": 0.01492,
    "C": 0.02782,
    "D": 0.04253,
    "E": 0.12702,
    "F": 0.02228,
    "G": 0.02015,
    "H": 0.06094,
    "I": 0.06966,
    "J": 0.00153,
    "K": 0.00772,
    "L": 0.04025,
    "M": 0.02406,
    "N": 0.06749,
    "O": 0.07507,
    "P": 0.01929,
    "Q": 0.00095,
    "R": 0.05987,
    "S": 0.06327,
    "T": 0.09056,
    "U": 0.02758,
    "V": 0.00978,
    "W": 0.02360,
    "X": 0.00150,
    "Y": 0.01974,
    "Z": 0.00074,
}


def index_of_coincidence(text: str) -> float:
    """Index of Coincidence: probability that two random letters match.

    Formula: IC = Sum(n_i * (n_i - 1)) / (N * (N - 1))

    where n_i = count of letter i, N = total letter count.

    Worked example for "AABB":
      counts: A=2, B=2
      N = 4
      numerator = 2*1 + 2*1 = 4
      denominator = 4*3 = 12
      IC = 4/12 = 0.333...

    Returns 0.0 for texts with fewer than 2 letters (the formula requires
    N*(N-1) > 0).

    >>> index_of_coincidence("AABB")
    0.3333333333333333
    """
    counts = frequency_count(text)
    n = sum(counts.values())

    # ── Need at least 2 letters for the formula to work ──
    if n < 2:
        return 0.0

    # ── Numerator: Sum of n_i * (n_i - 1) ──
    # For each letter, this counts the number of ways to pick 2 of that
    # letter from the text. It is n_i choose 2, times 2.
    numerator = sum(count * (count - 1) for count in counts.values())

    # ── Denominator: N * (N - 1) ──
    # Total number of ways to pick any 2 letters from the text.
    denominator = n * (n - 1)

    return numerator / denominator


def entropy(text: str) -> float:
    """Shannon entropy of the letter distribution in bits.

    Formula: H = -Sum(p_i * log2(p_i))

    where p_i is the proportion of letter i in the text (A-Z only).

    Entropy measures how "surprising" or "random" the text appears.
    Higher entropy means more uniform distribution (harder to break).
    Lower entropy means some letters dominate (easier to break).

    Key reference values:
      - Uniform over 26 letters: log2(26) ~ 4.700 bits (maximum)
      - Typical English: ~4.0-4.5 bits
      - Single repeated letter: 0 bits (no surprise at all)

    Returns 0.0 for empty text.

    >>> entropy("AAAA")
    0.0
    """
    counts = frequency_count(text)
    total = sum(counts.values())

    if total == 0:
        return 0.0

    # ── Compute -Sum(p_i * log2(p_i)) ──
    # We skip letters with count 0 because 0 * log2(0) is defined as 0
    # in information theory (by the limit as p -> 0+).
    h = 0.0
    for count in counts.values():
        if count > 0:
            p = count / total
            h -= p * math.log2(p)
    return h
