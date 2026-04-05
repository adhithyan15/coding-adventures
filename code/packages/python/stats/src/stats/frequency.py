"""Frequency analysis -- tools for counting and comparing letter distributions.

These functions are essential for classical cryptanalysis. When you intercept an
encrypted message, the first thing you do is count how often each letter appears.
In English, 'E' appears about 12.7% of the time, 'T' about 9.1%, and so on.
By comparing the frequency distribution of a ciphertext against known English
frequencies, you can determine what kind of cipher was used and start to break it.

Key Concepts
============

**Frequency count:** Raw count of each letter (A-Z, case-insensitive).
Non-alphabetic characters are ignored.

**Frequency distribution:** Each count divided by the total number of letters.
This gives proportions that sum to 1.0 (approximately, due to floating point).

**Chi-squared statistic:** Measures how well an observed distribution matches
an expected one. A low chi-squared means a good match.

  chi_squared = Sum((observed_i - expected_i)^2 / expected_i)

The chi-squared test is the workhorse of frequency-based cipher breaking.
For a Caesar cipher, you shift the observed distribution and compute chi-squared
against English frequencies. The shift with the lowest chi-squared is likely
the key.
"""

from __future__ import annotations


def frequency_count(text: str) -> dict[str, int]:
    """Count occurrences of each letter (A-Z) in the text, case-insensitive.

    Non-alphabetic characters are silently ignored.

    The returned dictionary maps uppercase letters to their counts. Only letters
    that actually appear in the text are included.

    >>> frequency_count("Hello")
    {'H': 1, 'E': 1, 'L': 2, 'O': 1}
    """
    counts: dict[str, int] = {}
    for char in text.upper():
        if "A" <= char <= "Z":
            counts[char] = counts.get(char, 0) + 1
    return counts


def frequency_distribution(text: str) -> dict[str, float]:
    """Proportion of each letter (A-Z) in the text.

    Each proportion is count / total_letters. The proportions sum to
    approximately 1.0.

    This is what you compare against ENGLISH_FREQUENCIES to determine
    how "English-like" a piece of text is.

    >>> frequency_distribution("AABB")
    {'A': 0.5, 'B': 0.5}
    """
    counts = frequency_count(text)
    total = sum(counts.values())
    if total == 0:
        return {}
    return {letter: count / total for letter, count in counts.items()}


def chi_squared(observed: list[float], expected: list[float]) -> float:
    """Chi-squared statistic for two parallel arrays of values.

    Formula: chi_squared = Sum((O_i - E_i)^2 / E_i)

    This measures how far the observed distribution is from the expected one.
    A value of 0 means perfect match. Larger values mean worse match.

    Both arrays must have the same length. Expected values must be positive
    (division by zero is not meaningful).

    Worked example:
      observed = [10, 20, 30]
      expected = [20, 20, 20]

      chi_squared = (10-20)^2/20 + (20-20)^2/20 + (30-20)^2/20
                  = 100/20 + 0/20 + 100/20
                  = 5.0 + 0.0 + 5.0
                  = 10.0

    >>> chi_squared([10, 20, 30], [20, 20, 20])
    10.0
    """
    if len(observed) != len(expected):
        msg = "observed and expected must have the same length"
        raise ValueError(msg)
    return sum((o - e) ** 2 / e for o, e in zip(observed, expected))


def chi_squared_text(text: str, expected_freq: dict[str, float]) -> float:
    """Chi-squared statistic comparing text letter frequencies to expected.

    This is a convenience wrapper: it counts the letters in the text,
    computes expected counts from the expected frequency table, and
    runs the chi-squared formula.

    Steps:
    1. Count letters in text (A-Z, case-insensitive).
    2. For each letter in expected_freq, compute:
       expected_count = expected_freq[letter] * total_letters
    3. Compute chi_squared over all 26 letters.
       Letters not in the text get observed=0.

    This is the function you call when breaking a Caesar cipher:
    for each possible shift, decrypt, call chi_squared_text with
    ENGLISH_FREQUENCIES, and pick the shift with the lowest score.
    """
    counts = frequency_count(text)
    total = sum(counts.values())
    if total == 0:
        return 0.0

    result = 0.0
    for letter, freq in expected_freq.items():
        observed_count = float(counts.get(letter.upper(), 0))
        expected_count = freq * total
        if expected_count > 0:
            result += (observed_count - expected_count) ** 2 / expected_count
    return result
