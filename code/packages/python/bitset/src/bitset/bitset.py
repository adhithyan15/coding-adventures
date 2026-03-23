"""
bitset.py -- Bitset: A Compact Boolean Array Packed into 64-bit Words
=====================================================================

A bitset stores a sequence of bits -- each one either 0 or 1 -- packed into
Python ``int`` values used as 64-bit words. Instead of using an entire ``bool``
object (~28 bytes on CPython) to represent a single true/false value, a bitset
packs 64 of them into a single word.

Why does this matter?

1. **Space**: 10,000 booleans as ``list[bool]`` ~ 80,000 bytes.
   As a bitset ~ 1,250 bytes. That's a 64x improvement.

2. **Speed**: AND-ing two boolean lists loops over 10,000 elements.
   AND-ing two bitsets loops over ~157 words. The CPU performs a single
   64-bit AND instruction on each word, operating on 64 bits at once.

3. **Ubiquity**: Bitsets appear in Bloom filters, register allocators,
   graph algorithms (visited sets), database bitmap indexes, filesystem
   free-block bitmaps, network subnet masks, and garbage collectors.

Bit Ordering: LSB-First
------------------------

We use Least Significant Bit first ordering. Bit 0 is the least significant
bit of word 0. Bit 63 is the most significant bit of word 0. Bit 64 is the
least significant bit of word 1. And so on.

::

    Word 0                              Word 1
    +-----------------------------+     +-----------------------------+
    | bit 63  ...  bit 2  bit 1  bit 0| | bit 127 ... bit 65  bit 64 |
    +-----------------------------+     +-----------------------------+
    MSB <--------------------- LSB      MSB <--------------------- LSB

The three fundamental formulas that drive every bitset operation::

    word_index = i // 64       (which word contains bit i?)
    bit_offset = i % 64        (which position within that word?)
    bitmask    = 1 << (i % 64) (a mask with only bit i set)

These are the heart of the entire implementation.

Python-Specific Notes
---------------------

Python's ``int`` type has arbitrary precision -- it never overflows. This is
both a blessing (no need for u64 casts) and a curse (we must manually mask
to 64 bits after operations like NOT that would otherwise produce negative
numbers). We use ``& WORD_MASK`` (where ``WORD_MASK = (1 << 64) - 1``) to
keep each word within the 64-bit unsigned range.
"""

from __future__ import annotations

from collections.abc import Iterator
from typing import Self


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
#
# BITS_PER_WORD is 64 because we use Python ints as 64-bit words. Every
# formula in this module uses this constant rather than a magic number.

BITS_PER_WORD: int = 64
"""Number of bits stored in each word of the bitset."""

WORD_MASK: int = (1 << BITS_PER_WORD) - 1
"""
Bitmask for a full 64-bit word: 0xFFFF_FFFF_FFFF_FFFF.

Python ints have arbitrary precision, so bitwise NOT (~x) on a positive int
produces a negative number (Python treats ints as having infinite leading 1s
in two's complement). We AND with WORD_MASK after NOT to get the unsigned
64-bit result we want.

Example::

    x = 5                       # 0b...0000_0101
    ~x                          # -6 in Python (infinite leading 1s)
    ~x & WORD_MASK              # 0xFFFFFFFFFFFFFFFA (correct 64-bit NOT)
"""


# ---------------------------------------------------------------------------
# Error type
# ---------------------------------------------------------------------------
#
# We have exactly one error: an invalid binary string was passed to
# from_binary_str. This keeps the error type minimal and focused.


class BitsetError(Exception):
    """Raised when a bitset operation encounters invalid input.

    Currently the only case is an invalid binary string passed to
    ``Bitset.from_binary_str`` -- a string containing characters other
    than ``'0'`` and ``'1'``.
    """


# ---------------------------------------------------------------------------
# Helper functions
# ---------------------------------------------------------------------------
#
# These small utility functions compute the word index, bit offset, and
# number of words needed for a given bit count. They're used throughout
# the implementation.


def _words_needed(bit_count: int) -> int:
    """How many 64-bit words do we need to store ``bit_count`` bits?

    This is ceiling division: ``(bit_count + 63) // 64``.

    Examples::

        _words_needed(0)   = 0   (no bits, no words)
        _words_needed(1)   = 1   (1 bit needs 1 word)
        _words_needed(64)  = 1   (64 bits fit exactly in 1 word)
        _words_needed(65)  = 2   (65 bits need 2 words)
        _words_needed(128) = 2   (128 bits fit exactly in 2 words)
        _words_needed(200) = 4   (200 bits need ceil(200/64) = 4 words)
    """
    return (bit_count + BITS_PER_WORD - 1) // BITS_PER_WORD


def _word_index(i: int) -> int:
    """Which word contains bit ``i``? Simply ``i // 64``.

    Examples::

        _word_index(0)   = 0   (bit 0 is in word 0)
        _word_index(63)  = 0   (bit 63 is the last bit of word 0)
        _word_index(64)  = 1   (bit 64 is the first bit of word 1)
        _word_index(137) = 2   (bit 137 is in word 2)
    """
    return i // BITS_PER_WORD


def _bit_offset(i: int) -> int:
    """Which bit position within its word does bit ``i`` occupy?

    Simply ``i % 64``.

    Examples::

        _bit_offset(0)   = 0
        _bit_offset(63)  = 63
        _bit_offset(64)  = 0   (first bit of the next word)
        _bit_offset(137) = 9   (137 - 2*64 = 9)
    """
    return i % BITS_PER_WORD


def _bitmask(i: int) -> int:
    """A bitmask with only bit ``i`` set within its word.

    This is ``1 << (i % 64)``. We use this mask to isolate, set, clear,
    or toggle a single bit within a word using bitwise operations::

        To set bit i:    word |= _bitmask(i)            (OR turns bit on)
        To clear bit i:  word &= ~_bitmask(i) & MASK    (AND with inverted mask)
        To test bit i:   (word & _bitmask(i)) != 0      (AND isolates the bit)
        To toggle bit i: word ^= _bitmask(i)            (XOR flips the bit)
    """
    return 1 << _bit_offset(i)


def _popcount_word(word: int) -> int:
    """Count the number of set bits in a single 64-bit word.

    Python 3.10+ provides ``int.bit_count()`` which maps to the hardware
    POPCNT instruction on modern CPUs. We use it directly.

    Examples::

        _popcount_word(0)    = 0
        _popcount_word(1)    = 1
        _popcount_word(0xFF) = 8   (8 bits set in 0b11111111)
        _popcount_word(WORD_MASK) = 64  (all 64 bits set)
    """
    return word.bit_count()


# ---------------------------------------------------------------------------
# The Bitset class
# ---------------------------------------------------------------------------
#
# Internal Representation
# ~~~~~~~~~~~~~~~~~~~~~~~
#
# We store bits in a list[int] called _words. Each int is treated as a
# 64-bit unsigned value (masked to WORD_MASK). We also track _len, the
# logical size -- the number of bits the user considers "addressable".
# The capacity is always len(_words) * 64.
#
#     +------------------------------------------------------------------+
#     |                          capacity (256 bits = 4 words)           |
#     |                                                                  |
#     |  +------------------------------------------+                    |
#     |  |              _len (200 bits)              | ... unused ....   |
#     |  |  (highest addressable bit index + 1)      | (always zero)    |
#     |  +------------------------------------------+                    |
#     +------------------------------------------------------------------+
#
# **Clean-trailing-bits invariant**: Bits beyond _len in the last word are
# always zero. This is critical for correctness of popcount, any, all, none,
# equality, and to_integer. Every operation that modifies the last word must
# clean trailing bits afterwards.


class Bitset:
    """A compact bitset that packs boolean values into 64-bit words.

    ``Bitset`` provides O(n/64) bulk bitwise operations (AND, OR, XOR, NOT),
    efficient iteration over set bits using trailing-zero-count, and
    ArrayList-style automatic growth when you set bits beyond the current size.

    Examples::

        from bitset import Bitset

        # Create a bitset and set some bits
        bs = Bitset(100)
        bs.set(0)
        bs.set(42)
        bs.set(99)
        assert bs.popcount() == 3

        # Iterate over set bits
        bits = list(bs.iter_set_bits())
        assert bits == [0, 42, 99]

        # Bulk operations return new bitsets
        other = Bitset(100)
        other.set(42)
        other.set(50)
        intersection = bs & other
        assert intersection.popcount() == 1  # only bit 42
    """

    __slots__ = ("_words", "_len")

    # ------------------------------------------------------------------
    # Constructors
    # ------------------------------------------------------------------

    def __init__(self, size: int = 0) -> None:
        """Create a new bitset with all bits initially zero.

        The ``size`` parameter sets the logical length (``_len``). The capacity
        is rounded up to the next multiple of 64.

        Examples::

            bs = Bitset(100)
            assert len(bs) == 100
            assert bs.capacity() == 128  # 2 words * 64 bits/word
            assert bs.popcount() == 0    # all bits start as zero

        ``Bitset(0)`` is valid and creates an empty bitset::

            bs = Bitset(0)
            assert len(bs) == 0
            assert bs.capacity() == 0
        """
        self._words: list[int] = [0] * _words_needed(size)
        self._len: int = size

    @classmethod
    def from_integer(cls, value: int) -> Self:
        """Create a bitset from a non-negative integer.

        Bit 0 of the bitset is the least significant bit of ``value``.
        The ``_len`` of the result is the position of the highest set bit + 1.
        If ``value == 0``, then ``_len = 0``.

        How it works: We split the integer into 64-bit chunks (words) by
        repeatedly masking and shifting::

            value = 0x0000_0000_0000_0005  (decimal 5, binary 101)
            word 0 = value & WORD_MASK = 5
            value >>= 64               → 0
            No more words needed.
            len = 3 (bit_length of 5)

        For values larger than 64 bits, we extract multiple words::

            value = (1 << 100)  (bit 100 set)
            word 0 = 0, word 1 = (1 << 36)
            len = 101

        Examples::

            bs = Bitset.from_integer(5)  # binary: 101
            assert len(bs) == 3          # highest bit is position 2
            assert bs.test(0)            # bit 0 = 1
            assert not bs.test(1)        # bit 1 = 0
            assert bs.test(2)            # bit 2 = 1

        Raises:
            BitsetError: If ``value`` is negative.
        """
        if value < 0:
            raise BitsetError(f"from_integer requires a non-negative value, got {value}")

        # Special case: zero produces an empty bitset.
        if value == 0:
            return cls(0)

        # The logical length is the position of the highest set bit + 1.
        # Python's int.bit_length() gives us exactly this.
        bit_len = value.bit_length()

        # Extract 64-bit words from the integer by masking and shifting.
        # This works for arbitrarily large Python ints.
        words: list[int] = []
        remaining = value
        while remaining > 0:
            words.append(remaining & WORD_MASK)
            remaining >>= BITS_PER_WORD

        bs = cls.__new__(cls)
        bs._words = words
        # Pad to the correct number of words for the bit length
        needed = _words_needed(bit_len)
        while len(bs._words) < needed:
            bs._words.append(0)
        bs._len = bit_len
        return bs

    @classmethod
    def from_binary_str(cls, s: str) -> Self:
        """Create a bitset from a string of ``'0'`` and ``'1'`` characters.

        The leftmost character is the highest-indexed bit (conventional binary
        notation, matching how humans write numbers). The rightmost character
        is bit 0.

        String-to-bits mapping::

            Input string: "1 0 1 0"
            Position:      3 2 1 0    (leftmost = highest bit index)

            Bit 0 = '0' (rightmost char)
            Bit 1 = '1'
            Bit 2 = '0'
            Bit 3 = '1' (leftmost char)

            This is the same as the integer 10 (binary 1010).

        Examples::

            bs = Bitset.from_binary_str("1010")
            assert len(bs) == 4
            assert bs.test(1)    # bit 1 = '1'
            assert bs.test(3)    # bit 3 = '1'
            assert not bs.test(0)  # bit 0 = '0'

        Raises:
            BitsetError: If the string contains characters other than
                ``'0'`` and ``'1'``.
        """
        # Validate: every character must be '0' or '1'.
        if not all(c in ("0", "1") for c in s):
            raise BitsetError(f"invalid binary string: {s!r}")

        # Empty string produces an empty bitset.
        if not s:
            return cls(0)

        # The string length is the logical len of the bitset.
        bit_len = len(s)
        bs = cls(bit_len)

        # Walk the string from right to left (LSB to MSB).
        # The rightmost character (index len(s)-1) is bit 0.
        # The leftmost character (index 0) is bit len(s)-1.
        for char_idx, ch in enumerate(reversed(s)):
            if ch == "1":
                # char_idx is the bit index (0 = rightmost = LSB).
                wi = _word_index(char_idx)
                bs._words[wi] |= _bitmask(char_idx)

        # Clean trailing bits defensively.
        bs._clean_trailing_bits()
        return bs

    # ------------------------------------------------------------------
    # Single-bit operations
    # ------------------------------------------------------------------
    #
    # These are the bread-and-butter operations: set a bit, clear a bit,
    # test whether a bit is set, toggle a bit. Each one translates to a
    # single bitwise operation on the containing word.
    #
    # Growth semantics:
    #   - set(i) and toggle(i) AUTO-GROW the bitset if i >= _len.
    #   - test(i) and clear(i) do NOT grow. They return False / do nothing
    #     for out-of-range indices. This is safe because unallocated bits
    #     are conceptually zero.

    def set(self, i: int) -> None:
        """Set bit ``i`` to 1. Auto-grows the bitset if ``i >= len``.

        How auto-growth works: If ``i`` is beyond the current capacity, we
        double the capacity repeatedly until it's large enough (with a minimum
        of 64 bits). This is the same amortized O(1) strategy used by Python's
        ``list``.

        ::

            Before: len=100, capacity=128 (2 words)
            set(200): 200 >= 128, so double: 128 -> 256. Now 200 < 256.
            After: len=201, capacity=256 (4 words)

        The core operation: OR the bitmask into the word::

            words[2] = 0b...0000_0000
            mask     = 0b...0010_0000   (bit 5 within the word)
            result   = 0b...0010_0000   (bit 5 is now set)

        OR is idempotent: setting an already-set bit is a no-op.

        Examples::

            bs = Bitset(10)
            bs.set(5)
            assert bs.test(5)

            # Auto-growth:
            bs.set(100)  # grows from len=10 to len=101
            assert len(bs) == 101
            assert bs.test(100)
        """
        self._ensure_capacity(i)
        self._words[_word_index(i)] |= _bitmask(i)

    def clear(self, i: int) -> None:
        """Set bit ``i`` to 0. No-op if ``i >= len`` (does not grow).

        Clearing a bit that's already 0 is a no-op. Clearing a bit beyond
        the bitset's length is also a no-op -- there's nothing to clear,
        because unallocated bits are conceptually zero.

        How it works: AND the word with the inverted bitmask. The inverted
        mask has all bits set EXCEPT the target bit, so every other bit is
        preserved::

            words[2] = 0b...0010_0100   (bits 2 and 5 set)
            mask     = 0b...0010_0000   (bit 5)
            ~mask    = 0b...1101_1111   (everything except bit 5)
            result   = 0b...0000_0100   (bit 5 cleared, bit 2 preserved)

        Note: In Python, ``~mask`` produces a negative number (because Python
        ints have infinite precision). We AND with ``WORD_MASK`` to get the
        correct 64-bit unsigned result.

        Examples::

            bs = Bitset(10)
            bs.set(5)
            assert bs.test(5)
            bs.clear(5)
            assert not bs.test(5)

            # Clearing beyond len is a no-op:
            bs.clear(999)  # no error, no growth
            assert len(bs) == 10
        """
        if i >= self._len:
            return  # out of range: nothing to clear
        self._words[_word_index(i)] &= (~_bitmask(i)) & WORD_MASK

    def test(self, i: int) -> bool:
        """Test whether bit ``i`` is set. Returns ``False`` if ``i >= len``.

        This is a pure read operation -- it never modifies the bitset.
        Testing a bit beyond the bitset's length returns ``False`` because
        unallocated bits are conceptually zero.

        How it works: AND the word with the bitmask. If the result is
        non-zero, the bit is set::

            words[2] = 0b...0010_0100   (bits 2 and 5 set)
            mask     = 0b...0010_0000   (bit 5)
            result   = 0b...0010_0000   (non-zero -> bit 5 is set)

            mask     = 0b...0000_1000   (bit 3)
            result   = 0b...0000_0000   (zero -> bit 3 is not set)

        Examples::

            bs = Bitset(10)
            bs.set(5)
            assert bs.test(5)
            assert not bs.test(3)
            assert not bs.test(999)  # beyond len -> False
        """
        if i >= self._len:
            return False  # out of range: conceptually zero
        return (self._words[_word_index(i)] & _bitmask(i)) != 0

    def toggle(self, i: int) -> None:
        """Toggle (flip) bit ``i``. Auto-grows if ``i >= len``.

        If the bit is 0, it becomes 1. If it's 1, it becomes 0.

        How it works: XOR with the bitmask flips exactly one bit::

            words[2] = 0b...0010_0100   (bits 2 and 5 set)
            mask     = 0b...0010_0000   (bit 5)
            result   = 0b...0000_0100   (bit 5 flipped to 0)

            words[2] = 0b...0000_0100   (only bit 2 set)
            mask     = 0b...0010_0000   (bit 5)
            result   = 0b...0010_0100   (bit 5 flipped to 1)

        Examples::

            bs = Bitset(10)
            bs.toggle(5)       # 0 -> 1
            assert bs.test(5)
            bs.toggle(5)       # 1 -> 0
            assert not bs.test(5)
        """
        self._ensure_capacity(i)
        self._words[_word_index(i)] ^= _bitmask(i)
        # Toggle might have set a bit in the last word's trailing region.
        # Clean trailing bits to maintain the invariant.
        self._clean_trailing_bits()

    # ------------------------------------------------------------------
    # Bulk bitwise operations
    # ------------------------------------------------------------------
    #
    # All bulk operations return a NEW bitset. They don't modify either
    # operand. The result has len = max(a._len, b._len).
    #
    # When two bitsets have different lengths, the shorter one is
    # "zero-extended" conceptually. In practice, we just stop reading
    # from the shorter one's words once they run out and treat missing
    # words as zero.
    #
    # Performance: each operation processes one 64-bit word per loop
    # iteration, so 64 bits are handled in a single CPU instruction.
    # This is the fundamental performance advantage of bitsets.

    def bitwise_and(self, other: Bitset) -> Bitset:
        """Bitwise AND: result bit is 1 only if BOTH input bits are 1.

        Truth table::

            A  B  A&B
            0  0   0
            0  1   0
            1  0   0
            1  1   1

        AND is used for **intersection**: elements that are in both sets.

        Returns a new bitset with ``len = max(self._len, other._len)``.

        Examples::

            a = Bitset.from_integer(0b1100)  # bits 2,3
            b = Bitset.from_integer(0b1010)  # bits 1,3
            c = a.bitwise_and(b)
            assert c.to_integer() == 0b1000  # only bit 3
        """
        return self._binary_op(other, lambda a, b: a & b)

    def bitwise_or(self, other: Bitset) -> Bitset:
        """Bitwise OR: result bit is 1 if EITHER (or both) input bits are 1.

        Truth table::

            A  B  A|B
            0  0   0
            0  1   1
            1  0   1
            1  1   1

        OR is used for **union**: elements that are in either set.

        Examples::

            a = Bitset.from_integer(0b1100)  # bits 2,3
            b = Bitset.from_integer(0b1010)  # bits 1,3
            c = a.bitwise_or(b)
            assert c.to_integer() == 0b1110  # bits 1,2,3
        """
        return self._binary_op(other, lambda a, b: a | b)

    def bitwise_xor(self, other: Bitset) -> Bitset:
        """Bitwise XOR: result bit is 1 if the input bits DIFFER.

        Truth table::

            A  B  A^B
            0  0   0
            0  1   1
            1  0   1
            1  1   0

        XOR is used for **symmetric difference**: elements in either set
        but not both.

        Examples::

            a = Bitset.from_integer(0b1100)  # bits 2,3
            b = Bitset.from_integer(0b1010)  # bits 1,3
            c = a.bitwise_xor(b)
            assert c.to_integer() == 0b0110  # bits 1,2
        """
        return self._binary_op(other, lambda a, b: a ^ b)

    def bitwise_not(self) -> Bitset:
        """Bitwise NOT: flip every bit within ``len``.

        Truth table::

            A  ~A
            0   1
            1   0

        NOT is used for **complement**: elements NOT in the set.

        **Important**: NOT flips bits within ``_len``, NOT within ``capacity``.
        Bits beyond ``_len`` remain zero (clean-trailing-bits invariant).
        The result has the same ``_len`` as the input.

        Note: In Python, ``~word`` on a positive int gives a negative result
        (because Python ints have infinite precision in two's complement).
        We AND with ``WORD_MASK`` to get the correct 64-bit unsigned NOT.

        Examples::

            a = Bitset.from_integer(0b1010)  # len=4, bits 1,3 set
            b = a.bitwise_not()
            assert b.to_integer() == 0b0101  # len=4, bits 0,2 set
        """
        result_words = [(~w) & WORD_MASK for w in self._words]

        # Critical: clean trailing bits! The NOT operation flipped ALL bits
        # in every word, including the trailing bits beyond _len that were
        # zero. We must zero them out again to maintain the invariant.
        result = Bitset.__new__(Bitset)
        result._words = result_words
        result._len = self._len
        result._clean_trailing_bits()
        return result

    def and_not(self, other: Bitset) -> Bitset:
        """AND-NOT (set difference): bits in ``self`` that are NOT in ``other``.

        This is equivalent to ``self & (~other)``, but more efficient because
        we don't need to create an intermediate NOT result.

        Truth table::

            A  B  A & ~B
            0  0    0
            0  1    0
            1  0    1
            1  1    0

        AND-NOT is used for **set difference**: elements in A but not in B.

        Examples::

            a = Bitset.from_integer(0b1110)  # bits 1,2,3
            b = Bitset.from_integer(0b1010)  # bits 1,3
            c = a.and_not(b)
            assert c.to_integer() == 0b0100  # only bit 2
        """
        return self._binary_op(other, lambda a, b: a & ((~b) & WORD_MASK))

    # ------------------------------------------------------------------
    # Counting and query operations
    # ------------------------------------------------------------------

    def popcount(self) -> int:
        """Count the number of set (1) bits.

        Named after the CPU instruction ``POPCNT`` (population count) that
        counts set bits in a single word. The implementation applies Python's
        ``int.bit_count()`` (which maps to hardware POPCNT on modern CPUs)
        to each word and sums.

        For a bitset with N bits, this runs in O(N/64) time -- we process
        64 bits per loop iteration.

        Examples::

            bs = Bitset.from_integer(0b10110)  # bits 1,2,4 set
            assert bs.popcount() == 3
        """
        return sum(_popcount_word(w) for w in self._words)

    def capacity(self) -> int:
        """Return the capacity: total allocated bits (always a multiple of 64).

        Capacity >= len. The difference (capacity - len) is "slack space" --
        bits that exist in memory but are always zero.

        Examples::

            bs = Bitset(100)
            assert bs.capacity() == 128  # 2 words * 64 bits
        """
        return len(self._words) * BITS_PER_WORD

    def any(self) -> bool:
        """Return ``True`` if at least one bit is set.

        Short-circuits: returns as soon as it finds a non-zero word,
        without scanning the rest. This is O(1) in the best case
        (first word is non-zero) and O(N/64) in the worst case.

        Examples::

            bs = Bitset(100)
            assert not bs.any()
            bs.set(50)
            assert bs.any()
        """
        return any(w != 0 for w in self._words)

    def all(self) -> bool:
        """Return ``True`` if ALL bits in ``0.._len`` are set.

        For an empty bitset (``_len = 0``), returns ``True`` -- this is
        **vacuous truth**, the same convention used by Python's ``all([])``,
        Rust's ``Iterator::all``, and mathematical logic ("for all x in {},
        P(x)" is true).

        How it works: For each full word (all except possibly the last), we
        check if every bit is set (``word == WORD_MASK``). For the last word,
        we only check the bits within ``_len``.

        Examples::

            bs = Bitset(0)
            assert bs.all()  # vacuous truth

            bs = Bitset.from_binary_str("1111")
            assert bs.all()

            bs = Bitset.from_binary_str("1110")
            assert not bs.all()
        """
        # Vacuous truth: all bits of nothing are set.
        if self._len == 0:
            return True

        num_words = len(self._words)

        # Check all full words (all bits must be 1 = WORD_MASK).
        for i in range(max(0, num_words - 1)):
            if self._words[i] != WORD_MASK:
                return False

        # Check the last word: only the bits within _len matter.
        remaining = _bit_offset(self._len)
        if remaining == 0:
            # _len is a multiple of 64, so the last word is a full word.
            return self._words[num_words - 1] == WORD_MASK
        else:
            # Create a mask for the valid bits: (1 << remaining) - 1
            # Example: remaining = 8 -> mask = 0xFF (bits 0-7)
            mask = (1 << remaining) - 1
            return self._words[num_words - 1] == mask

    def none(self) -> bool:
        """Return ``True`` if no bits are set. Equivalent to ``not self.any()``.

        Examples::

            bs = Bitset(100)
            assert bs.none()
        """
        return not self.any()

    # ------------------------------------------------------------------
    # Iteration
    # ------------------------------------------------------------------

    def iter_set_bits(self) -> Iterator[int]:
        """Yield the indices of all set bits in ascending order.

        How it works: trailing-zero-count trick
        ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

        For each non-zero word, we use ``(word & -word).bit_length() - 1``
        to find the lowest set bit (equivalent to hardware ``CTZ``), yield
        its index, then clear it with ``word &= word - 1``::

            word = 0b10100100   (bits 2, 5, 7 are set)

            Step 1: trailing_zeros = 2  -> yield base + 2
                    word &= word - 1   -> 0b10100000  (clear bit 2)

            Step 2: trailing_zeros = 5  -> yield base + 5
                    word &= word - 1   -> 0b10000000  (clear bit 5)

            Step 3: trailing_zeros = 7  -> yield base + 7
                    word &= word - 1   -> 0b00000000  (clear bit 7)

            word == 0, move to next word.

        The trick ``word &= word - 1`` clears the lowest set bit::

            word     = 0b10100100
            word - 1 = 0b10100011  (borrow propagates through trailing zeros)
            AND      = 0b10100000  (lowest set bit is cleared)

        This is O(k) where k is the number of set bits, and it skips zero
        words entirely, making it very efficient for sparse bitsets.

        Examples::

            bs = Bitset.from_integer(0b10100101)
            bits = list(bs.iter_set_bits())
            assert bits == [0, 2, 5, 7]
        """
        for word_idx, word in enumerate(self._words):
            base = word_idx * BITS_PER_WORD
            # Process each set bit in this word using the trailing-zeros trick.
            while word != 0:
                # Find the position of the lowest set bit.
                # In Python, (word & -word) isolates the lowest set bit,
                # and .bit_length() - 1 gives its position.
                bit_pos = (word & -word).bit_length() - 1
                index = base + bit_pos

                # Only yield bits within _len (don't yield trailing garbage).
                if index >= self._len:
                    return

                yield index

                # Clear the lowest set bit: word &= word - 1
                word &= word - 1

    # ------------------------------------------------------------------
    # Conversion operations
    # ------------------------------------------------------------------

    def to_integer(self) -> int:
        """Convert the bitset to a non-negative integer.

        Since Python has arbitrary precision integers, this always succeeds
        (unlike Go or Rust which are limited to 64/128 bits).

        Returns 0 for an empty bitset.

        How it works: We reconstruct the integer by shifting each word into
        its correct position and OR-ing them together::

            integer = words[0] | (words[1] << 64) | (words[2] << 128) | ...

        Examples::

            bs = Bitset.from_integer(42)
            assert bs.to_integer() == 42

            bs = Bitset(0)
            assert bs.to_integer() == 0
        """
        if not self._words:
            return 0

        result = 0
        for i, word in enumerate(self._words):
            result |= word << (i * BITS_PER_WORD)
        return result

    def to_binary_str(self) -> str:
        """Convert to a string of ``'0'`` and ``'1'`` characters.

        The highest bit is on the left (conventional binary notation).
        This is the inverse of ``from_binary_str``. An empty bitset produces
        an empty string ``""``.

        Examples::

            bs = Bitset.from_integer(5)  # binary 101
            assert bs.to_binary_str() == "101"

            bs = Bitset(0)
            assert bs.to_binary_str() == ""
        """
        if self._len == 0:
            return ""

        # Build the string from the highest bit (_len-1) down to bit 0.
        # This produces conventional binary notation: MSB on the left.
        chars: list[str] = []
        for i in range(self._len - 1, -1, -1):
            chars.append("1" if self.test(i) else "0")
        return "".join(chars)

    # ------------------------------------------------------------------
    # Operator overloads
    # ------------------------------------------------------------------
    #
    # Python lets us define __and__, __or__, __xor__, __invert__ so users
    # can write `a & b`, `a | b`, `a ^ b`, `~a` instead of calling the
    # named methods directly.

    def __and__(self, other: Bitset) -> Bitset:
        """``a & b`` -- bitwise AND (intersection)."""
        return self.bitwise_and(other)

    def __or__(self, other: Bitset) -> Bitset:
        """``a | b`` -- bitwise OR (union)."""
        return self.bitwise_or(other)

    def __xor__(self, other: Bitset) -> Bitset:
        """``a ^ b`` -- bitwise XOR (symmetric difference)."""
        return self.bitwise_xor(other)

    def __invert__(self) -> Bitset:
        """``~a`` -- bitwise NOT (complement)."""
        return self.bitwise_not()

    def __len__(self) -> int:
        """``len(bs)`` -- returns the logical size (number of addressable bits).

        This is the value passed to ``Bitset(size)``, or the highest bit
        index + 1 after any auto-growth operations.
        """
        return self._len

    def __contains__(self, i: object) -> bool:
        """``i in bs`` -- test whether bit ``i`` is set.

        Returns ``False`` for out-of-range indices. Non-integer values
        always return ``False``.

        Examples::

            bs = Bitset(10)
            bs.set(5)
            assert 5 in bs
            assert 3 not in bs
        """
        if not isinstance(i, int):
            return False
        return self.test(i)

    def __iter__(self) -> Iterator[int]:
        """``for bit in bs`` -- iterate over set bit indices.

        This is an alias for ``iter_set_bits()``, allowing bitsets to be
        used directly in for loops::

            for bit_index in my_bitset:
                print(f"bit {bit_index} is set")
        """
        return self.iter_set_bits()

    def __eq__(self, other: object) -> bool:
        """Two bitsets are equal if they have the same ``_len`` and same bits set.

        Capacity is irrelevant to equality -- a bitset with ``capacity = 128``
        can equal one with ``capacity = 256`` if their ``_len`` and set bits
        match.

        Thanks to the clean-trailing-bits invariant, we can compare words
        directly -- trailing bits are always zero.
        """
        if not isinstance(other, Bitset):
            return NotImplemented

        if self._len != other._len:
            return False

        # Compare word-by-word. If one has more words allocated, the
        # extra words must all be zero (due to clean-trailing-bits).
        max_words = max(len(self._words), len(other._words))
        for i in range(max_words):
            a = self._words[i] if i < len(self._words) else 0
            b = other._words[i] if i < len(other._words) else 0
            if a != b:
                return False
        return True

    def __repr__(self) -> str:
        """Human-readable representation like ``Bitset('101')``.

        For empty bitsets, returns ``Bitset('')``.

        Examples::

            bs = Bitset.from_integer(5)
            assert repr(bs) == "Bitset('101')"

            bs = Bitset(0)
            assert repr(bs) == "Bitset('')"
        """
        return f"Bitset('{self.to_binary_str()}')"

    def __hash__(self) -> int:
        """Bitsets are hashable so they can be used in sets and as dict keys.

        The hash is based on ``_len`` and the tuple of words (excluding
        trailing zero words for consistency with equality).
        """
        # Trim trailing zero words for consistent hashing
        words = self._words[:]
        while words and words[-1] == 0:
            words.pop()
        return hash((self._len, tuple(words)))

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _ensure_capacity(self, i: int) -> None:
        """Ensure the bitset has capacity for bit ``i``. If not, grow by doubling.

        After this call, ``i < capacity`` and ``_len >= i + 1``.

        Growth strategy: We double the capacity repeatedly until it exceeds
        ``i``. The minimum capacity after growth is 64 (one word). This
        doubling strategy gives amortized O(1) growth, just like Python's
        ``list.append``.

        ::

            Example: capacity=128, set(500)
              128 -> 256 -> 512 -> 1024  (stop: 500 < 1024)
        """
        if i < self.capacity():
            # Already have room. But we might need to update _len.
            if i >= self._len:
                self._len = i + 1
            return

        # Need to grow. Start with current capacity (or 64 as minimum).
        new_cap = max(self.capacity(), BITS_PER_WORD)
        while new_cap <= i:
            new_cap *= 2

        # Extend the word list with zeros.
        new_word_count = new_cap // BITS_PER_WORD
        self._words.extend([0] * (new_word_count - len(self._words)))

        # Update _len to include the new bit.
        self._len = i + 1

    def _clean_trailing_bits(self) -> None:
        """Zero out any bits beyond ``_len`` in the last word.

        This maintains the clean-trailing-bits invariant. It must be called
        after any operation that might set bits beyond ``_len``:
          - bitwise_not() flips all bits, including trailing ones
          - from_binary_str might have rounding issues
          - toggle() on the last word
          - bulk operations (AND, OR, XOR) when operands have different sizes

        How it works::

            _len = 200, capacity = 256
            The last word holds bits 192-255, but only 192-199 are "real".
            remaining = 200 % 64 = 8
            mask = (1 << 8) - 1 = 0xFF  (bits 0-7)
            _words[3] &= 0xFF  -> zeroes out bits 8-63 of word 3

        If ``_len`` is a multiple of 64, there are no trailing bits to clean.
        """
        if self._len == 0 or not self._words:
            return

        remaining = _bit_offset(self._len)
        if remaining != 0:
            last_idx = len(self._words) - 1
            mask = (1 << remaining) - 1
            self._words[last_idx] &= mask

    def _binary_op(
        self, other: Bitset, op: object  # Callable[[int, int], int]
    ) -> Bitset:
        """Apply a binary operation word-by-word to produce a new bitset.

        This is the shared implementation for AND, OR, XOR, and AND-NOT.
        The ``op`` parameter is a function that takes two word values and
        returns the result word.

        The result has ``_len = max(self._len, other._len)``.
        Missing words from the shorter bitset are treated as zero.
        """
        result_len = max(self._len, other._len)
        max_words = max(len(self._words), len(other._words))
        result_words: list[int] = []

        for i in range(max_words):
            a = self._words[i] if i < len(self._words) else 0
            b = other._words[i] if i < len(other._words) else 0
            result_words.append(op(a, b) & WORD_MASK)  # type: ignore[operator]

        result = Bitset.__new__(Bitset)
        result._words = result_words
        result._len = result_len
        result._clean_trailing_bits()
        return result
