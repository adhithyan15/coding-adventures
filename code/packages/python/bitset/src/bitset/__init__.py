"""
Bitset: A Compact Boolean Array Packed into 64-bit Words
========================================================

A bitset stores a sequence of bits -- each one either 0 or 1 -- packed into
machine-word-sized integers. Instead of using an entire Python ``bool`` object
(~28 bytes each) to represent a single true/false value, a bitset packs 64 of
them into a single ``int``.

**Space**: A ``list`` of 10,000 booleans consumes roughly 80,000 bytes. A bitset
storing the same 10,000 bits uses ~1,250 bytes -- a 64x improvement.

**Speed**: AND-ing two boolean lists loops over 10,000 elements. AND-ing two
bitsets loops over ~157 words, performing a single 64-bit AND per word.

Quick start::

    from bitset import Bitset

    bs = Bitset(100)
    bs.set(0)
    bs.set(42)
    bs.set(99)
    assert bs.popcount() == 3

    # Iterate over set bits
    print(list(bs.iter_set_bits()))  # [0, 42, 99]

    # Bulk operations return new bitsets
    other = Bitset(100)
    other.set(42)
    other.set(50)
    intersection = bs.bitwise_and(other)   # or: bs & other
    assert intersection.popcount() == 1    # only bit 42

Error handling::

    from bitset import Bitset, BitsetError

    try:
        Bitset.from_binary_str("102")  # invalid character
    except BitsetError:
        print("caught it!")
"""

from bitset.bitset import Bitset, BitsetError

__all__ = [
    "Bitset",
    "BitsetError",
]
