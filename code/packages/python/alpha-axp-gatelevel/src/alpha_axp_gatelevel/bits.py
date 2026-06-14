"""bits.py — 64-bit bit-list conversion helpers for the Alpha AXP gate-level simulator.

This module is the bridge between the "integer world" (the Python API, test
programs, memory addresses) and the "gate world" (lists of 0/1 values flowing
through AND, OR, XOR, NOT primitives).

All actual arithmetic in this module uses Python integer operations because we
are doing bookkeeping (packing/unpacking bits), NOT simulating data-path
operations.  Data-path operations — ADD, SUB, AND, OR, XOR, NOT — live in
alu.py and must route through gate primitives.

LSB-first ordering
──────────────────
We use LSB-first bit lists throughout.  This matches the convention used by
the arithmetic package's ripple_carry_adder:

    int_to_bits(5, 8) → [1, 0, 1, 0, 0, 0, 0, 0]
                         ^bit0 (2^0=1, set)
                                ^bit2 (2^2=4, set)

This is the natural representation for a ripple-carry adder: bit[0] feeds
the first full adder (carry in = 0), bit[1] feeds the second, and so on.

Overflow detection
──────────────────
For a two's-complement addition of N-bit values:
  overflow = XOR(carry_into_bit_(N-1), carry_out_of_bit_(N-1))

For a 64-bit add:
  - carry_into_bit_63 = carry propagated from the ripple chain up to bit 62
  - carry_out         = carry out of bit 63 (returned by ripple_carry_adder)
  - overflow          = XOR(carry_into_63, carry_out)

We obtain carry_into_63 by running a 63-bit adder on bits[0:63], then using
that carry_out as carry_into_63 for the final (bit-63) full adder.

In practice, ripple_carry_adder already gives us the carry_out.  To get the
carry_into_bit_63 we split the 64-bit add into a 63-bit add (bits 0-62) plus
a 1-bit final stage, then XOR the two carries.
"""

from __future__ import annotations

from arithmetic import ripple_carry_adder
from logic_gates import NOT, XOR

# ── Integer ↔ bit-list bridge ──────────────────────────────────────────────────

def int_to_bits(value: int, width: int) -> list[int]:
    """Convert an integer to a LSB-first bit list of the given width.

    The value is first masked to `width` bits so that negative Python ints
    and values wider than `width` are handled correctly.

    Examples
    ────────
    >>> int_to_bits(5, 8)
    [1, 0, 1, 0, 0, 0, 0, 0]
    >>> int_to_bits(0, 4)
    [0, 0, 0, 0]
    >>> int_to_bits(255, 8)
    [1, 1, 1, 1, 1, 1, 1, 1]
    """
    mask = (1 << width) - 1
    v = value & mask
    return [(v >> i) & 1 for i in range(width)]


def bits_to_int(bits: list[int]) -> int:
    """Convert a LSB-first bit list to a non-negative integer.

    Examples
    ────────
    >>> bits_to_int([1, 0, 1, 0])
    5
    >>> bits_to_int([0, 0, 0, 0])
    0
    """
    result = 0
    for i, b in enumerate(bits):
        result |= b << i
    return result


# ── 64-bit gate-level arithmetic helpers ──────────────────────────────────────

def add_64bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int, int]:
    """Add two unsigned 64-bit values via ripple_carry_adder (64 full adders).

    Returns (result, carry_out, overflow).

    Overflow detection for signed two's-complement arithmetic:
      overflow = XOR(carry_into_bit_63, carry_out_of_bit_63)

    We split the add into a 63-bit ripple (bits 0–62) to obtain the carry
    into bit 63, then add the single bit 63 separately.

    Parameters
    ──────────
    a, b      : unsigned 64-bit integers
    carry_in  : initial carry (0 or 1)

    Returns
    ───────
    result    : unsigned 64-bit integer (bits 0–63 of a + b + carry_in)
    carry_out : carry out of bit 63
    overflow  : 1 if signed overflow occurred, 0 otherwise
    """
    a_bits = int_to_bits(a & 0xFFFF_FFFF_FFFF_FFFF, 64)
    b_bits = int_to_bits(b & 0xFFFF_FFFF_FFFF_FFFF, 64)

    # Full 64-bit ripple add
    sum_bits, carry_out = ripple_carry_adder(a_bits, b_bits, carry_in)

    # Carry into bit 63: run 63-bit adder on bits[0:63]
    low_sum, carry_into_63 = ripple_carry_adder(
        a_bits[:63], b_bits[:63], carry_in
    )
    overflow = XOR(carry_into_63, carry_out)

    return bits_to_int(sum_bits), carry_out, overflow


def add_128bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int]:
    """Add two 128-bit unsigned integers via ripple_carry_adder.

    Returns (result, carry_out).  Used for UMULH (upper 64 bits of a 128-bit
    product).

    Parameters
    ──────────
    a, b      : unsigned 128-bit integers (Python ints may be any size)
    carry_in  : initial carry (0 or 1)
    """
    mask128 = (1 << 128) - 1
    a_bits = int_to_bits(a & mask128, 128)
    b_bits = int_to_bits(b & mask128, 128)
    sum_bits, carry_out = ripple_carry_adder(a_bits, b_bits, carry_in)
    return bits_to_int(sum_bits), carry_out


def add_32bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int, int]:
    """Add two unsigned 32-bit values via ripple_carry_adder (32 full adders).

    Returns (result, carry_out, overflow).

    Overflow uses the same split-at-bit-31 technique as add_64bit.
    """
    a_bits = int_to_bits(a & 0xFFFF_FFFF, 32)
    b_bits = int_to_bits(b & 0xFFFF_FFFF, 32)

    sum_bits, carry_out = ripple_carry_adder(a_bits, b_bits, carry_in)

    low_sum, carry_into_31 = ripple_carry_adder(
        a_bits[:31], b_bits[:31], carry_in
    )
    overflow = XOR(carry_into_31, carry_out)

    return bits_to_int(sum_bits), carry_out, overflow


# ── Bitwise inversion via NOT gates ───────────────────────────────────────────

def invert_64bit(value: int) -> int:
    """Bitwise NOT of a 64-bit value: apply NOT to each of the 64 bits.

    This routes through 64 NOT gate calls, one per bit.

    Example
    ───────
    >>> hex(invert_64bit(0))
    '0xffffffffffffffff'
    >>> invert_64bit(0xFFFF_FFFF_FFFF_FFFF)
    0
    """
    bits = int_to_bits(value & 0xFFFF_FFFF_FFFF_FFFF, 64)
    inverted = [NOT(b) for b in bits]
    return bits_to_int(inverted)


def invert_32bit(value: int) -> int:
    """Bitwise NOT of a 32-bit value: apply NOT to each of the 32 bits.

    Example
    ───────
    >>> hex(invert_32bit(0))
    '0xffffffff'
    >>> invert_32bit(0xFFFF_FFFF)
    0
    """
    bits = int_to_bits(value & 0xFFFF_FFFF, 32)
    inverted = [NOT(b) for b in bits]
    return bits_to_int(inverted)


# ── Zero detection ─────────────────────────────────────────────────────────────

def compute_zero(bits: list[int]) -> int:
    """Return 1 if ALL bits in the list are 0, otherwise return 0.

    Gate-level implementation: OR all bits together, then NOT.
    This mirrors the hardware NOR-tree that feeds the zero flag.

    A single OR-reduction tree:
      combined = bits[0] | bits[1] | bits[2] | ...
      result   = NOT(combined)

    Example
    ───────
    >>> compute_zero([0, 0, 0, 0])
    1
    >>> compute_zero([0, 1, 0, 0])
    0
    """
    from logic_gates import OR

    combined = bits[0]
    for b in bits[1:]:
        combined = OR(combined, b)
    return NOT(combined)


# ── Shift operations via bit-list manipulation ─────────────────────────────────

def shl_64(value: int, shamt: int) -> int:
    """Shift left logical: shift the 64-bit value left by shamt bits.

    Implemented via bit-list manipulation: fill zeros at the low end.
    Clamps shamt to [0, 63]; shifting by >=64 returns 0.

    Example
    ───────
    >>> shl_64(1, 3)
    8
    >>> shl_64(0xFFFF_FFFF_FFFF_FFFF, 63)
    9223372036854775808
    """
    shamt = shamt & 63
    bits = int_to_bits(value & 0xFFFF_FFFF_FFFF_FFFF, 64)
    # Shift: the bit that was at position i moves to position i+shamt
    # Positions 0..shamt-1 become 0 (zero fill at low end)
    shifted = [0] * shamt + bits[: 64 - shamt]
    return bits_to_int(shifted)


def shr_64_logical(value: int, shamt: int) -> int:
    """Shift right logical: shift right by shamt, filling zeros at the top.

    Example
    ───────
    >>> shr_64_logical(8, 3)
    1
    >>> shr_64_logical(0xFFFF_FFFF_FFFF_FFFF, 1)
    9223372036854775807
    """
    shamt = shamt & 63
    bits = int_to_bits(value & 0xFFFF_FFFF_FFFF_FFFF, 64)
    # Shift right: bit at position i+shamt moves to position i
    # Positions 64-shamt..63 become 0 (zero fill at high end)
    shifted = bits[shamt:] + [0] * shamt
    return bits_to_int(shifted)


def shr_64_arith(value: int, shamt: int) -> int:
    """Shift right arithmetic: shift right by shamt, filling with sign bit.

    The sign bit (bit 63) is replicated into the vacated positions.
    This preserves the sign of two's-complement negative numbers.

    Example
    ───────
    >>> shr_64_arith(8, 3)
    1
    >>> import hex
    >>> # 0xFFFF_FFFF_FFFF_FFFF >> 1 should still be 0xFFFF_FFFF_FFFF_FFFF
    >>> hex(shr_64_arith(0xFFFF_FFFF_FFFF_FFFF, 1))
    '0xffffffffffffffff'
    """
    shamt = shamt & 63
    bits = int_to_bits(value & 0xFFFF_FFFF_FFFF_FFFF, 64)
    sign_bit = bits[63]  # MSB = sign bit
    # Fill with sign bit at the top
    shifted = bits[shamt:] + [sign_bit] * shamt
    return bits_to_int(shifted)


# ── Sign extension ─────────────────────────────────────────────────────────────

def sext32_to_64(value: int) -> int:
    """Sign-extend a 32-bit value to 64 bits.

    If bit 31 of value is 1 (negative in two's complement), bits 32–63 are
    filled with 1.  Otherwise they are 0.

    The result is always a non-negative Python int in [0, 2^64 - 1] that,
    when interpreted as a two's-complement 64-bit integer, has the same signed
    value as the original 32-bit input.

    Example
    ───────
    >>> hex(sext32_to_64(0x7FFFFFFF))    # positive: 2^31 - 1
    '0x7fffffff'
    >>> hex(sext32_to_64(0x80000000))    # negative: -2^31
    '0xffffffff80000000'
    >>> hex(sext32_to_64(0xFFFFFFFF))    # -1
    '0xffffffffffffffff'
    """
    bits32 = int_to_bits(value & 0xFFFF_FFFF, 32)
    sign_bit = bits32[31]
    # Extend to 64 bits by replicating the sign bit
    bits64 = bits32 + [sign_bit] * 32
    return bits_to_int(bits64)
