"""
Hash function quality analysis utilities.

Two metrics are implemented:

  1. Avalanche score: measures how many output bits change when a single
     input bit is flipped.  A perfect hash function flips exactly 50% of
     output bits on any single-bit input change (Strict Avalanche Criterion).

  2. Distribution test: chi-squared test for uniformity across buckets.
     A chi-squared value close to (num_buckets - 1) indicates good uniformity.
     Much larger values indicate some buckets receive far more keys than others.
"""

from __future__ import annotations

import os
from collections.abc import Callable


def avalanche_score(
    hash_fn: Callable[[bytes], int],
    output_bits: int,
    sample_size: int = 1000,
) -> float:
    """
    Compute the avalanche score for a hash function.

    For each of `sample_size` random 8-byte inputs, we flip each of the
    64 input bits in turn and measure what fraction of the `output_bits`
    output bits change.

    The ideal score is 0.5 (50% of output bits change per input bit flip).
    Values between 0.40 and 0.60 indicate acceptable avalanche.
    Values outside this range suggest systematic bias.

    The avalanche effect is named after the observation that a small
    disturbance (one snowflake) should trigger a large change (an avalanche).
    In hashing, a single bit flip in the input should look completely
    different in the output — this prevents attackers from learning anything
    about a key by observing nearby hash values.

    Args:
        hash_fn:     Function from bytes → int.
        output_bits: Number of output bits the function produces (32 or 64).
        sample_size: Number of random 8-byte inputs to test.

    Returns:
        Average fraction of output bits that differ (ideal: 0.5).
    """
    total_bit_flips = 0
    total_trials = 0

    for _ in range(sample_size):
        # Generate a random 8-byte input.
        input_bytes = os.urandom(8)
        h1 = hash_fn(input_bytes)

        # Flip each of the 64 input bits and measure output difference.
        for bit_pos in range(len(input_bytes) * 8):
            byte_idx = bit_pos >> 3      # which byte contains this bit
            bit_mask = 1 << (bit_pos & 7)  # which bit within that byte

            # XOR the target byte with the mask to flip exactly one bit.
            flipped = bytearray(input_bytes)
            flipped[byte_idx] ^= bit_mask
            h2 = hash_fn(bytes(flipped))

            # Count how many output bits differ.
            diff = h1 ^ h2
            # popcount: count the set bits in `diff`
            total_bit_flips += bin(diff).count("1")
            total_trials += output_bits

    return total_bit_flips / total_trials


def distribution_test(
    hash_fn: Callable[[bytes], int],
    inputs: list[bytes],
    num_buckets: int,
) -> float:
    """
    Chi-squared uniformity test for a hash function.

    Hashes all `inputs` and distributes them into `num_buckets` buckets
    by computing hash(inp) % num_buckets.  Returns the chi-squared
    statistic measuring deviation from perfect uniformity.

    Chi-squared formula:
        χ² = Σ (observed_i - expected)² / expected
             for each bucket i

    Where expected = len(inputs) / num_buckets.

    Interpretation:
      χ² ≈ num_buckets - 1   → excellent uniformity (matches theoretical
                                chi-squared distribution with k-1 degrees
                                of freedom)
      χ² significantly > k   → poor uniformity (some buckets overcrowded)

    For 100 buckets and 10,000 inputs, χ² should be approximately 99.
    Values above 200 indicate problematic clustering.

    Args:
        hash_fn:     Function from bytes → int.
        inputs:      List of byte strings to hash.
        num_buckets: Number of buckets (hash table slots).

    Returns:
        Chi-squared statistic (lower is better; ideal ≈ num_buckets - 1).
    """
    counts: list[int] = [0] * num_buckets
    for inp in inputs:
        bucket = hash_fn(inp) % num_buckets
        counts[bucket] += 1

    expected = len(inputs) / num_buckets
    chi2 = sum((c - expected) ** 2 / expected for c in counts)
    return chi2
