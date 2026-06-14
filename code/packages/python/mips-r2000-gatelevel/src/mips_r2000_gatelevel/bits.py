"""bits.py — 32-bit integer ↔ bit-list bridge for gate-level MIPS R2000.

This module is the boundary between the "integer world" (Python ints used by
tests, memory, and the protocol layer) and the "gate world" (lists of 0s and
1s flowing through AND/OR/XOR/NOT gates and ripple-carry adders).

Design principle
────────────────
Python integers are *only* used here for encoding and decoding.  Every other
module (alu.py, register_file.py) calls these helpers and then operates
entirely on bit lists.  This mirrors how a real chip works: the pin interface
converts voltages to logic levels; internal logic never "knows" about decimal.

Bit ordering
────────────
All bit lists are LSB-first (index 0 = bit 0 = least significant).  This
matches the ripple-carry adder in the ``arithmetic`` package, which processes
bit 0 first through the first full adder.

    int_to_bits(5, 8)  → [1, 0, 1, 0, 0, 0, 0, 0]
                          ↑ bit 0 (value 1)
                                   ↑ bit 2 (value 4)

    bits_to_int([1,0,1,0,0,0,0,0]) → 5

Overflow detection (add_32bit)
──────────────────────────────
Signed overflow occurs when two same-sign operands produce an opposite-sign
result.  Equivalently: V = XOR(carry_into_bit31, carry_out_of_bit31).

We detect this by running a 33-bit ripple-carry adder and comparing the
carry into bit 31 vs the carry out of bit 31.

    overflow = XOR(carry_in_to_MSB, carry_out_of_MSB)

This is the same logic the real MIPS R2000 ALU uses.
"""

from __future__ import annotations

from arithmetic import ripple_carry_adder
from logic_gates import NOT, OR, XOR

# ── Integer ↔ bit list ─────────────────────────────────────────────────────────


def int_to_bits(value: int, width: int) -> list[int]:
    """Convert a non-negative integer to a fixed-width LSB-first bit list.

    The value is masked to ``width`` bits before conversion, so overflow is
    silently discarded (matching unsigned register semantics).

    Args:
        value: Integer to convert (may be any non-negative Python int).
        width: Number of bits in the output list.

    Returns:
        List of ``width`` integers each 0 or 1, index 0 = bit 0 (LSB).

    Examples:
        >>> int_to_bits(5, 8)
        [1, 0, 1, 0, 0, 0, 0, 0]
        >>> int_to_bits(0, 4)
        [0, 0, 0, 0]
        >>> int_to_bits(0xFFFFFFFF, 32)  # all ones
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
         1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
    """
    mask = (1 << width) - 1
    value = value & mask
    return [(value >> i) & 1 for i in range(width)]


def bits_to_int(bits: list[int]) -> int:
    """Convert an LSB-first bit list to an unsigned integer.

    Args:
        bits: List of 0/1 values, index 0 = LSB.

    Returns:
        Non-negative Python int representing the bit pattern.

    Examples:
        >>> bits_to_int([1, 0, 1, 0, 0, 0, 0, 0])
        5
        >>> bits_to_int([0, 0, 0, 0])
        0
    """
    result = 0
    for i, b in enumerate(bits):
        result |= b << i
    return result


# ── 32-bit addition ────────────────────────────────────────────────────────────


def add_32bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int, int]:
    """Add two 32-bit values via a gate-level ripple-carry adder.

    This is the primary arithmetic primitive.  All ADD, ADDI, SUB, etc.
    instructions ultimately call this function.  The adder is composed of
    32 full adders (from the ``arithmetic`` package), each built from XOR,
    AND, and OR gates.

    Overflow detection uses the carry-into-MSB vs carry-out-of-MSB trick:
        V = XOR(carry_into_bit31, carry_out_of_bit31)

    We accomplish this by running a 33-bit adder: the 33rd position gives us
    the carry_out of bit 31.  The carry_in to bit 31 is the carry_out from
    the 31-bit prefix adder.  We compute both by asking ripple_carry_adder
    for a 33-bit result.

    Args:
        a: First 32-bit operand (unsigned, masked to 32 bits).
        b: Second 32-bit operand (unsigned, masked to 32 bits).
        carry_in: Initial carry-in to bit 0 (0 or 1).

    Returns:
        (result, carry_out, overflow):
            result    — 32-bit unsigned sum
            carry_out — carry out of bit 31 (useful for unsigned overflow)
            overflow  — 1 if signed overflow occurred, else 0
    """
    a_bits = int_to_bits(a, 33)  # 33 bits: bit 32 is always 0
    b_bits = int_to_bits(b, 33)
    # Force bit 32 to 0 for both operands (sign extension for overflow calc)
    a_bits[32] = 0
    b_bits[32] = 0

    sum_bits, _ = ripple_carry_adder(a_bits, b_bits, carry_in)

    result = bits_to_int(sum_bits[:32])
    carry_out = sum_bits[32]
    # Carry into bit 31 = carry out of the 31-bit sub-adder.  In a 33-bit
    # ripple chain, the carry that enters position 31 is the carry out of
    # position 30 — equivalent to the carry produced by the lower 31 bits.
    # We recover it by running a separate 32-bit addition and reading its carry.
    a32 = int_to_bits(a, 32)
    b32 = int_to_bits(b, 32)
    _, carry_31 = ripple_carry_adder(a32, b32, carry_in)
    # carry_31 is the carry OUT of bit 31, NOT into bit 31.
    # carry_in_to_31 = carry OUT of bit 30 = carry produced by bits [0..30].
    # We can get this by running a 31-bit adder on bits [0..30].
    a31 = int_to_bits(a, 31)
    b31 = int_to_bits(b, 31)
    _, carry_in_to_31 = ripple_carry_adder(a31, b31, carry_in)

    overflow = XOR(carry_in_to_31, carry_31)
    return result, carry_out, overflow


def add_64bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int]:
    """Add two 64-bit values via a gate-level ripple-carry adder.

    Used by MULT/MULTU to accumulate partial products into a 64-bit result.
    Internally we use a 64-bit ripple-carry chain (64 full adders).

    Args:
        a: First 64-bit operand.
        b: Second 64-bit operand.
        carry_in: Initial carry-in.

    Returns:
        (result, carry_out) — 64-bit sum and carry out of bit 63.
    """
    a_bits = int_to_bits(a, 64)
    b_bits = int_to_bits(b, 64)
    sum_bits, carry_out = ripple_carry_adder(a_bits, b_bits, carry_in)
    result = bits_to_int(sum_bits)
    return result, carry_out


# ── Bitwise NOT ────────────────────────────────────────────────────────────────


def invert_32bit(value: int) -> int:
    """Bitwise NOT of a 32-bit value, applied via 32 NOT gates.

    In hardware this is 32 inverter gates in parallel — one per data bit.
    Used by the ALU for two's complement negation (NOT then +1).

    Args:
        value: 32-bit unsigned integer.

    Returns:
        Bitwise NOT of value, as a 32-bit unsigned integer.

    Example:
        >>> hex(invert_32bit(0x00000000))
        '0xffffffff'
        >>> hex(invert_32bit(0xFFFFFFFF))
        '0x0'
    """
    bits = int_to_bits(value, 32)
    inverted = [NOT(b) for b in bits]
    return bits_to_int(inverted)


# ── Parity and zero detection ─────────────────────────────────────────────────


def compute_parity(bits: list[int]) -> int:
    """Compute even parity via an XOR reduction tree.

    In hardware, this is a balanced binary tree of XOR gates (log2(N) levels).
    We implement it iteratively for clarity; the result is identical.

    Returns 1 if an odd number of bits are 1, else 0.

    Example:
        >>> compute_parity([1, 0, 1])
        0
        >>> compute_parity([1, 1, 1])
        1
    """
    result = 0
    for b in bits:
        result = XOR(result, b)
    return result


def compute_zero(bits: list[int]) -> int:
    """Return 1 if all bits are 0, else 0 (NOR reduction tree).

    In hardware: a tree of NOR gates reduces N bits to 1 bit.
    Logically: zero = NOT(OR(b0, b1, ..., bn-1))

    Used by the ALU to set the Zero flag after every operation.

    Example:
        >>> compute_zero([0, 0, 0, 0])
        1
        >>> compute_zero([0, 1, 0, 0])
        0
    """
    combined = 0
    for b in bits:
        combined = OR(combined, b)
    return NOT(combined)


# ── Shift operations ───────────────────────────────────────────────────────────


def shl_32(value: int, shamt: int) -> int:
    """Shift left logical by shamt bits (0–31), gate-level implementation.

    In hardware, a barrel shifter is a cross-bar of multiplexers.  Here we
    model it as direct bit-list manipulation: shift the list, fill vacated
    positions with 0 (logical shift).

    The bit list is LSB-first, so "shifting left" means moving bits toward
    higher indices (multiplying by 2^shamt).

    Args:
        value: 32-bit unsigned integer to shift.
        shamt: Number of positions to shift (0–31).

    Returns:
        32-bit result after logical left shift.

    Examples:
        >>> shl_32(1, 0)   # no shift
        1
        >>> shl_32(1, 1)   # 1 << 1 = 2
        2
        >>> shl_32(1, 31)  # 1 << 31 = 0x80000000
        2147483648
    """
    if shamt == 0:
        return value
    if shamt >= 32:
        return 0
    bits = int_to_bits(value, 32)
    # LSB-first: bit[i] represents 2^i. Shift left by shamt means bit[i]
    # moves to bit[i+shamt]. New low bits (indices 0..shamt-1) become 0.
    shifted = [0] * shamt + bits[:32 - shamt]
    return bits_to_int(shifted)


def shr_32_logical(value: int, shamt: int) -> int:
    """Shift right logical by shamt bits (zero-fill), gate-level.

    Logical shift right fills vacated high bits with 0 regardless of sign.
    Used for SRL and SRLV instructions.

    Args:
        value: 32-bit unsigned integer to shift.
        shamt: Number of positions to shift (0–31).

    Returns:
        32-bit result after logical right shift.

    Examples:
        >>> shr_32_logical(4, 1)   # 4 >> 1 = 2
        2
        >>> shr_32_logical(0x80000000, 31)  # MSB becomes 1
        1
    """
    if shamt == 0:
        return value
    if shamt >= 32:
        return 0
    bits = int_to_bits(value, 32)
    # LSB-first: shift right means bits[shamt..31] move to [0..31-shamt].
    # High bits (indices 31-shamt+1..31) become 0.
    shifted = bits[shamt:] + [0] * shamt
    return bits_to_int(shifted)


def shr_32_arith(value: int, shamt: int) -> int:
    """Shift right arithmetic by shamt bits (sign-fill), gate-level.

    Arithmetic shift right preserves the sign bit: vacated high bits are
    filled with the original bit 31 (MSB).  Used for SRA and SRAV.

    Args:
        value: 32-bit unsigned integer (bit 31 is the sign bit).
        shamt: Number of positions to shift (0–31).

    Returns:
        32-bit result after arithmetic right shift.

    Examples:
        >>> shr_32_arith(4, 1)           # positive: same as logical
        2
        >>> hex(shr_32_arith(0x80000000, 1))  # -2147483648 >> 1 = 0xC0000000
        '0xc0000000'
        >>> hex(shr_32_arith(0x80000000, 31)) # fully sign-extended = 0xFFFFFFFF
        '0xffffffff'
    """
    if shamt == 0:
        return value
    bits = int_to_bits(value, 32)
    sign_bit = bits[31]  # MSB — the sign
    if shamt >= 32:
        # Fully sign-extended: all bits become sign_bit
        return bits_to_int([sign_bit] * 32)
    # LSB-first shift: bits[shamt..31] move to [0..31-shamt], fill with sign
    shifted = bits[shamt:] + [sign_bit] * shamt
    return bits_to_int(shifted)
