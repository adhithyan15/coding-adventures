"""
hash_functions — Pure non-cryptographic hash function implementations.

A hash function maps arbitrary-length input data to a fixed-size integer.
This library implements four real-world algorithms used in hash tables,
Bloom filters, HyperLogLog, and other data structures:

  - FNV-1a (32-bit and 64-bit): Fowler-Noll-Vo, one byte at a time
  - DJB2: Dan Bernstein's classic shift-and-add
  - Polynomial rolling: string-optimized rolling polynomial hash
  - MurmurHash3 (32-bit): Austin Appleby's high-quality block hash

All functions accept either bytes or str (UTF-8 encoded automatically)
and return unsigned integers.

Quick start:

    >>> from hash_functions import fnv1a_32, djb2, murmur3_32
    >>> fnv1a_32(b"hello")
    1335831723
    >>> djb2(b"abc")
    193485963
    >>> murmur3_32(b"abc")
    3016911924
"""

from hash_functions.algorithms import (
    djb2,
    fnv1a_32,
    fnv1a_64,
    murmur3_32,
    polynomial_rolling,
)
from hash_functions.analysis import avalanche_score, distribution_test

__all__ = [
    "fnv1a_32",
    "fnv1a_64",
    "djb2",
    "polynomial_rolling",
    "murmur3_32",
    "avalanche_score",
    "distribution_test",
]
