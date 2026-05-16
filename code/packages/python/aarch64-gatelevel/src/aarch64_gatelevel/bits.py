"""bits.py — 64-bit bit-list conversion helpers for the AArch64 gate-level simulator.

This module is the bridge between the "integer world" (the Python API, test
programs, memory addresses) and the "gate world" (lists of 0/1 values flowing
through AND, OR, XOR, NOT primitives).

All actual arithmetic in this module uses Python integer operations ONLY for
bookkeeping (packing/unpacking bits from/to integers), NOT for simulating
data-path operations.  Data-path operations — ADD, SUB, AND, OR, XOR, NOT —
live in alu.py and must route through gate primitives.

LSB-first ordering
──────────────────
We use LSB-first bit lists throughout.  This matches the convention used by
the arithmetic package's ripple_carry_adder:

    int_to_bits(5, 8) → [1, 0, 1, 0, 0, 0, 0, 0]
                         ^bit0 (2^0=1, set)
                                ^bit2 (2^2=4, set)

This is the natural representation for a ripple-carry adder: bit[0] feeds
the first full adder (carry in = 0), bit[1] feeds the second, and so on.

Overflow detection for 64-bit operations
────────────────────────────────────────
For a two's-complement addition of N-bit values:
  overflow = XOR(carry_into_bit_(N-1), carry_out_of_bit_(N-1))

For a 64-bit add:
  - carry_into_bit_63 = carry propagated from the ripple chain up to bit 62
  - carry_out         = carry out of bit 63 (returned by ripple_carry_adder)
  - overflow          = XOR(carry_into_63, carry_out)

We obtain carry_into_63 by running a 63-bit adder on bits[0:63], then using
that carry_out as carry_into_63 for the final (bit-63) full adder.

AArch64 two's-complement subtract
──────────────────────────────────
AArch64 (like all ARM generations) defines subtraction as:
  A - B = A + NOT(B) + 1

The carry output of this operation signals "no borrow" (C=1 means A >= B
unsigned), which is the ARM borrow-complement convention.

Shift and rotate (64-bit and 32-bit variants)
─────────────────────────────────────────────
AArch64 W-register (32-bit) operations use 32-bit shifts.  We provide both
64-bit (shl_64, shr_64_logical, shr_64_arith, ror_64) and 32-bit
(shl_32, shr_32_logical, shr_32_arith, ror_32) variants.

All shifts are implemented via bit-list slicing: no Python shift operators
on register values.  The slicing itself is a bookkeeping operation — we are
rearranging wires in the circuit, not doing arithmetic.
"""

from __future__ import annotations

from arithmetic import ripple_carry_adder
from logic_gates import NOT, OR, XOR

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
    >>> int_to_bits(0xFFFFFFFFFFFFFFFF, 64)  # all ones
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
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


def add_64bit(
    a_bits: list[int], b_bits: list[int], carry_in: int = 0
) -> tuple[list[int], int, int]:
    """Add two 64-bit LSB-first bit lists via ripple_carry_adder (64 full adders).

    Returns (result_bits, carry_out, overflow).

    Overflow detection for signed two's-complement arithmetic:
      overflow = XOR(carry_into_bit_63, carry_out_of_bit_63)

    We split the add into a 63-bit ripple (bits 0–62) to obtain the carry
    into bit 63, then run the full 64-bit adder to get the real carry out.

    Parameters
    ──────────
    a_bits    : LSB-first 64-element bit list
    b_bits    : LSB-first 64-element bit list
    carry_in  : initial carry (0 or 1)

    Returns
    ───────
    result_bits : LSB-first 64-element bit list (result of a + b + carry_in)
    carry_out   : carry out of bit 63 (1 = unsigned overflow)
    overflow    : 1 if signed overflow occurred (XOR of carry_into_63 and carry_out)

    Example
    ───────
    >>> a = int_to_bits(1, 64); b = int_to_bits(1, 64)
    >>> r, c, v = add_64bit(a, b)
    >>> bits_to_int(r), c, v
    (2, 0, 0)
    >>> # Max unsigned + 1 wraps, carry set, no signed overflow
    >>> a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64); b = int_to_bits(1, 64)
    >>> r, c, v = add_64bit(a, b)
    >>> bits_to_int(r), c, v
    (0, 1, 0)
    """
    # Full 64-bit ripple add
    sum_bits, carry_out = ripple_carry_adder(a_bits, b_bits, carry_in)

    # Carry into bit 63: run 63-bit adder on bits[0:63]
    _low_sum, carry_into_63 = ripple_carry_adder(a_bits[:63], b_bits[:63], carry_in)
    overflow = XOR(carry_into_63, carry_out)

    return sum_bits, carry_out, overflow


def sub_64bit(a_bits: list[int], b_bits: list[int]) -> tuple[list[int], int, int]:
    """Subtract two 64-bit values via two's complement: A + NOT(B) + 1.

    AArch64 (ARM convention):
      carry_out=1 → no borrow (A >= B unsigned)
      carry_out=0 → borrow (A < B unsigned)

    Gate implementation:
      1. Invert all 64 bits of b (64 NOT gates)
      2. Add a + NOT(b) with carry_in=1 (ripple_carry_adder)

    Returns
    ───────
    result_bits : LSB-first 64-element bit list
    carry_out   : 1 = no borrow (ARM convention: C flag for SUBS)
    overflow    : signed overflow flag

    Example
    ───────
    >>> a = int_to_bits(10, 64); b = int_to_bits(3, 64)
    >>> r, c, v = sub_64bit(a, b)
    >>> bits_to_int(r), c
    (7, 1)
    >>> # Underflow: 0 - 1
    >>> a = int_to_bits(0, 64); b = int_to_bits(1, 64)
    >>> r, c, v = sub_64bit(a, b)
    >>> c   # borrow occurred → carry = 0 in ARM convention
    0
    """
    not_b = [NOT(b) for b in b_bits]
    return add_64bit(a_bits, not_b, carry_in=1)


def add_32bit(
    a_bits: list[int], b_bits: list[int], carry_in: int = 0
) -> tuple[list[int], int, int]:
    """Add two 32-bit LSB-first bit lists via ripple_carry_adder (32 full adders).

    Returns (result_bits, carry_out, overflow).

    Mirrors add_64bit but for 32-bit (W-register) operations.
    overflow = XOR(carry_into_bit_31, carry_out_of_bit_31)

    Example
    ───────
    >>> a = int_to_bits(5, 32); b = int_to_bits(3, 32)
    >>> r, c, v = add_32bit(a, b)
    >>> bits_to_int(r), c, v
    (8, 0, 0)
    """
    sum_bits, carry_out = ripple_carry_adder(a_bits, b_bits, carry_in)
    _low_sum, carry_into_31 = ripple_carry_adder(a_bits[:31], b_bits[:31], carry_in)
    overflow = XOR(carry_into_31, carry_out)
    return sum_bits, carry_out, overflow


def sub_32bit(a_bits: list[int], b_bits: list[int]) -> tuple[list[int], int, int]:
    """Subtract two 32-bit values via two's complement: A + NOT(B) + 1.

    Returns (result_bits, carry_out, overflow) following ARM carry convention.

    Example
    ───────
    >>> a = int_to_bits(10, 32); b = int_to_bits(3, 32)
    >>> r, c, v = sub_32bit(a, b)
    >>> bits_to_int(r), c
    (7, 1)
    """
    not_b = [NOT(b) for b in b_bits]
    return add_32bit(a_bits, not_b, carry_in=1)


# ── 64-bit bitwise operations via gate primitives ──────────────────────────────


def and_64bit(a: list[int], b: list[int]) -> list[int]:
    """64-bit AND: apply AND gate to each bit pair.

    Gate count: 64 AND gates.

    Example
    ───────
    >>> from logic_gates import AND
    >>> a = int_to_bits(0b1010, 64); b = int_to_bits(0b1100, 64)
    >>> bits_to_int(and_64bit(a, b))
    8
    """
    from logic_gates import AND
    return [AND(a[i], b[i]) for i in range(64)]


def or_64bit(a: list[int], b: list[int]) -> list[int]:
    """64-bit OR: apply OR gate to each bit pair.

    Gate count: 64 OR gates.
    """
    return [OR(a[i], b[i]) for i in range(64)]


def xor_64bit(a: list[int], b: list[int]) -> list[int]:
    """64-bit XOR: apply XOR gate to each bit pair.

    Gate count: 64 XOR gates.
    """
    return [XOR(a[i], b[i]) for i in range(64)]


def not_64bit(a: list[int]) -> list[int]:
    """64-bit NOT: apply NOT gate to each bit.

    Gate count: 64 NOT gates.
    """
    return [NOT(a[i]) for i in range(64)]


def and_32bit(a: list[int], b: list[int]) -> list[int]:
    """32-bit AND: apply AND gate to each bit pair."""
    from logic_gates import AND
    return [AND(a[i], b[i]) for i in range(32)]


def or_32bit(a: list[int], b: list[int]) -> list[int]:
    """32-bit OR: apply OR gate to each bit pair."""
    return [OR(a[i], b[i]) for i in range(32)]


def xor_32bit(a: list[int], b: list[int]) -> list[int]:
    """32-bit XOR: apply XOR gate to each bit pair."""
    return [XOR(a[i], b[i]) for i in range(32)]


def not_32bit(a: list[int]) -> list[int]:
    """32-bit NOT: apply NOT gate to each bit."""
    return [NOT(a[i]) for i in range(32)]


# ── Zero detection via NOR tree ────────────────────────────────────────────────


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
    combined = bits[0]
    for b in bits[1:]:
        combined = OR(combined, b)
    return NOT(combined)


# ── Shift and rotate operations via bit-list slicing ──────────────────────────
#
# Shifts move bits to different positions.  In a real circuit, a barrel shifter
# implements this with multiplexers: each output wire is connected to the
# correct input wire based on the shift amount.
#
# In our model, we implement shifts as Python list slicing, which is equivalent
# bookkeeping.  This is NOT a data-path operation — we are just rearranging
# which bit list element corresponds to which bit position.


def shl_64(bits_in: list[int], n: int) -> list[int]:
    """Shift left logical 64-bit by n positions (fill LSBs with 0).

    Bit at position i moves to position i+n.
    Positions 0..n-1 are filled with 0.
    If n >= 64, all bits are zero.

    Example
    ───────
    >>> bits_to_int(shl_64(int_to_bits(1, 64), 4))
    16
    >>> bits_to_int(shl_64(int_to_bits(1, 64), 64))
    0
    """
    if n >= 64:
        return [0] * 64
    # bits[0..63-n] move to positions [n..63]; positions [0..n-1] are 0
    return [0] * n + bits_in[: 64 - n]


def shr_64_logical(bits_in: list[int], n: int) -> list[int]:
    """Shift right logical 64-bit by n positions (fill MSBs with 0).

    Bit at position i+n moves to position i.
    Positions 63-n+1..63 are filled with 0.
    If n >= 64, all bits are zero.

    Example
    ───────
    >>> bits_to_int(shr_64_logical(int_to_bits(16, 64), 4))
    1
    """
    if n >= 64:
        return [0] * 64
    return bits_in[n:] + [0] * n


def shr_64_arith(bits_in: list[int], n: int) -> list[int]:
    """Shift right arithmetic 64-bit by n: fill MSBs with sign bit.

    The sign bit (bit 63) is replicated into the vacated high positions.
    This preserves the sign of two's-complement negative numbers.

    Example
    ───────
    >>> # -1 (all ones) >> 1 = -1 (still all ones)
    >>> bits_to_int(shr_64_arith(int_to_bits(0xFFFFFFFFFFFFFFFF, 64), 1))
    18446744073709551615
    """
    if n >= 64:
        n = 63  # saturate: arithmetic shift >= 64 replicates sign
    sign_bit = bits_in[63]  # MSB in LSB-first list
    return bits_in[n:] + [sign_bit] * n


def ror_64(bits_in: list[int], n: int) -> list[int]:
    """Rotate right 64-bit by n positions.

    Bit at position i moves to position (i - n) % 64.
    In LSB-first list: the n low bits wrap around to become the n high bits.

    Example
    ───────
    >>> bits_to_int(ror_64(int_to_bits(1, 64), 1))
    9223372036854775808
    """
    n = n % 64
    if n == 0:
        return bits_in[:]
    return bits_in[n:] + bits_in[:n]


def shl_32(bits_in: list[int], n: int) -> list[int]:
    """Shift left logical 32-bit by n positions.

    Example
    ───────
    >>> bits_to_int(shl_32(int_to_bits(1, 32), 4))
    16
    """
    if n >= 32:
        return [0] * 32
    return [0] * n + bits_in[: 32 - n]


def shr_32_logical(bits_in: list[int], n: int) -> list[int]:
    """Shift right logical 32-bit by n positions.

    Example
    ───────
    >>> bits_to_int(shr_32_logical(int_to_bits(16, 32), 4))
    1
    """
    if n >= 32:
        return [0] * 32
    return bits_in[n:] + [0] * n


def shr_32_arith(bits_in: list[int], n: int) -> list[int]:
    """Shift right arithmetic 32-bit: fill with sign bit.

    Example
    ───────
    >>> # 0x80000000 (-2^31) >> 1 = 0xC0000000
    >>> hex(bits_to_int(shr_32_arith(int_to_bits(0x80000000, 32), 1)))
    '0xc0000000'
    """
    if n >= 32:
        n = 31
    sign_bit = bits_in[31]
    return bits_in[n:] + [sign_bit] * n


def ror_32(bits_in: list[int], n: int) -> list[int]:
    """Rotate right 32-bit by n positions.

    Example
    ───────
    >>> bits_to_int(ror_32(int_to_bits(1, 32), 1))
    2147483648
    """
    n = n % 32
    if n == 0:
        return bits_in[:]
    return bits_in[n:] + bits_in[:n]


# ── Count Leading Zeros ────────────────────────────────────────────────────────


def clz_64(bits_in: list[int]) -> int:
    """Count leading zeros in a 64-bit LSB-first bit list.

    Scans from the MSB (bit 63) downward, counting zeros until a 1 is found.
    Returns 64 if all bits are 0.

    Gate-level: sequentially checks each bit from MSB to LSB using AND.
    Once a 1 is found, the scan stops.  In real hardware this is a priority
    encoder circuit.

    The return value is a Python int (the count), used for bookkeeping to
    then call int_to_bits(count, 64) in the ALU for the data-path result.

    Example
    ───────
    >>> clz_64(int_to_bits(0, 64))
    64
    >>> clz_64(int_to_bits(1, 64))
    63
    >>> clz_64(int_to_bits(0x8000000000000000, 64))
    0
    """
    from logic_gates import AND
    count = 0
    for bit_pos in range(63, -1, -1):
        if AND(bits_in[bit_pos], 1) == 1:
            break
        count += 1
    return count


def clz_32(bits_in: list[int]) -> int:
    """Count leading zeros in a 32-bit LSB-first bit list.

    Example
    ───────
    >>> clz_32(int_to_bits(0, 32))
    32
    >>> clz_32(int_to_bits(1, 32))
    31
    """
    from logic_gates import AND
    count = 0
    for bit_pos in range(31, -1, -1):
        if AND(bits_in[bit_pos], 1) == 1:
            break
        count += 1
    return count


# ── 64-bit multiply via shift-and-add ────────────────────────────────────────
#
# Binary multiplication (schoolbook algorithm):
#   product = 0
#   for each bit i of b (0..63):
#     if b[i] == 1:
#       product += a << i
#
# We accumulate a 128-bit product using two 64-bit halves.
# For mul_64, we only return the low 64 bits.
# For umulh_64/smulh_64 we return the high 64 bits.


def mul_64(a_bits: list[int], b_bits: list[int]) -> list[int]:
    """64-bit × 64-bit → low 64 bits via 64-iteration shift-and-add.

    This is the gate-level schoolbook multiplication algorithm.
    At each step, if bit[i] of b is 1, we add (a << i) to the accumulator.
    We accumulate as two 64-bit halves (lo, hi) and return just lo.

    Gate-level operations used: AND (to check each bit), add_64bit.
    Shift is bookkeeping (bit-list slicing).

    Example
    ───────
    >>> a = int_to_bits(6, 64); b = int_to_bits(7, 64)
    >>> bits_to_int(mul_64(a, b))
    42
    """
    from logic_gates import AND

    # Running product as two 64-bit halves (LSB-first)
    prod_lo = [0] * 64
    prod_hi = [0] * 64

    for i in range(64):
        # Check if bit i of b is 1 (gate-level)
        if AND(b_bits[i], 1) == 0:
            continue
        # Compute a << i: low 64 bits and high 64 bits
        # a << i: bits 0..63-i of a move to positions i..63 in lo word
        #         bits 64-i..63 of a move to positions 0..i-1 in hi word
        if i == 0:
            shifted_lo = a_bits[:]
            shifted_hi = [0] * 64
        elif i < 64:
            shifted_lo = [0] * i + a_bits[: 64 - i]
            shifted_hi = a_bits[64 - i :] + [0] * (64 - i)
        else:
            shifted_lo = [0] * 64
            shifted_hi = [0] * i + a_bits[: 64 - i] if i < 128 else [0] * 64

        # Add shifted_lo to prod_lo, propagate carry to prod_hi
        new_lo, carry, _ = add_64bit(prod_lo, shifted_lo, 0)
        # Add shifted_hi to prod_hi; carry from lo addition is handled separately below
        new_hi, _, _ = add_64bit(prod_hi, shifted_hi, 0)
        # Add the carry from lo into hi
        if carry:
            carry_lo = int_to_bits(1, 64)
            new_hi, _, _ = add_64bit(new_hi, carry_lo, 0)

        prod_lo = new_lo
        prod_hi = new_hi

    return prod_lo


def umulh_64(a_bits: list[int], b_bits: list[int]) -> list[int]:
    """Upper 64 bits of 128-bit unsigned product via 64-iteration shift-and-add.

    Same algorithm as mul_64, but returns the high 64 bits.
    Used for UMULH instruction.

    Example
    ───────
    >>> # (2^64 - 1)^2 = 2^128 - 2^65 + 1
    >>> # High 64 bits = 2^64 - 2 = 0xFFFFFFFFFFFFFFFE
    >>> a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    >>> bits_to_int(umulh_64(a, a))
    18446744073709551614
    """
    from logic_gates import AND

    prod_lo = [0] * 64
    prod_hi = [0] * 64

    for i in range(64):
        if AND(b_bits[i], 1) == 0:
            continue
        if i == 0:
            shifted_lo = a_bits[:]
            shifted_hi = [0] * 64
        elif i < 64:
            shifted_lo = [0] * i + a_bits[: 64 - i]
            shifted_hi = a_bits[64 - i :] + [0] * (64 - i)
        else:
            shifted_lo = [0] * 64
            shifted_hi = [0] * 64

        new_lo, carry, _ = add_64bit(prod_lo, shifted_lo, 0)
        new_hi, _, _ = add_64bit(prod_hi, shifted_hi, 0)
        if carry:
            carry_lo = int_to_bits(1, 64)
            new_hi, _, _ = add_64bit(new_hi, carry_lo, 0)

        prod_lo = new_lo
        prod_hi = new_hi

    return prod_hi


def smulh_64(a_bits: list[int], b_bits: list[int]) -> list[int]:
    """Upper 64 bits of signed 128-bit product via sign-correction method.

    Algorithm (Baugh-Wooley / standard sign correction):
      1. Compute unsigned high product: umulh_64(a, b)
      2. If a is negative (MSB=1): subtract b from high word
      3. If b is negative (MSB=1): subtract a from high word

    This is the standard conversion from unsigned to signed high multiply.

    Example
    ───────
    >>> # -1 * -1 = +1, high 64 bits = 0
    >>> a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)  # -1
    >>> bits_to_int(smulh_64(a, a))
    0
    """
    # Unsigned high product
    hi = umulh_64(a_bits, b_bits)

    # Sign correction
    a_sign = a_bits[63]  # MSB
    b_sign = b_bits[63]

    from logic_gates import AND
    if AND(a_sign, 1):
        # Subtract b from hi: hi = hi + NOT(b) + 1 = hi - b
        hi, _, _ = add_64bit(hi, [NOT(bb) for bb in b_bits], 1)

    if AND(b_sign, 1):
        # Subtract a from hi: hi = hi - a
        hi, _, _ = add_64bit(hi, [NOT(ab) for ab in a_bits], 1)

    return hi


# ── 64-bit unsigned division via restoring algorithm ─────────────────────────
#
# Binary division (restoring long division):
#   quotient = 0, remainder = a
#   for bit = 63 downto 0:
#     if (b << bit) fits in 64 bits AND remainder >= (b << bit):
#       remainder -= b << bit
#       set quotient bit
#
# This is a non-restoring approach adapted to 64-bit: we check whether
# b << bit would overflow 64 bits before attempting the comparison.
# When no overflow, we use sub_64bit to test.


def udiv_64(
    a_bits: list[int], b_bits: list[int]
) -> tuple[list[int], list[int]]:
    """64-bit unsigned division via 64-iteration restoring long division.

    Returns (quotient_bits, remainder_bits).
    If divisor is zero, returns ([0]*64, [0]*64) per AArch64 spec.

    Algorithm
    ─────────
    For each bit position from 63 downto 0:
      1. Check if b << bit would overflow 64 bits (if so, skip — too large)
      2. Compute shifted_b = b << bit
      3. Compute remainder - shifted_b using gate-level sub_64bit
      4. If no borrow (carry_out=1): update remainder, set quotient bit

    Gate-level operations: sub_64bit (carry check), shl_64 (shift), AND.

    Example
    ───────
    >>> a = int_to_bits(100, 64); b = int_to_bits(7, 64)
    >>> q, r = udiv_64(a, b)
    >>> bits_to_int(q), bits_to_int(r)
    (14, 2)
    """
    from logic_gates import AND

    # Division by zero
    if AND(compute_zero(b_bits), 1):
        return [0] * 64, [0] * 64

    quotient = [0] * 64
    remainder = a_bits[:]

    for bit in range(63, -1, -1):
        # Check if b << bit overflows 64 bits
        # Overflow happens when any of the top `bit` bits of b are set
        if bit > 0:
            top_bits = b_bits[64 - bit :]  # the bits that would shift above 63
            overflows = AND(NOT(compute_zero(top_bits)), 1)
        else:
            overflows = 0

        if overflows:
            continue  # b << bit > 2^64-1 >= remainder, cannot subtract

        # shifted_b = b << bit
        shifted_b = shl_64(b_bits, bit)

        # Try: remainder - shifted_b
        diff_bits, carry_out, _ = sub_64bit(remainder, shifted_b)

        # carry_out=1 means no borrow (remainder >= shifted_b)
        if AND(carry_out, 1):
            remainder = diff_bits
            quotient[bit] = 1  # set this quotient bit

    return quotient, remainder


def sdiv_64(
    a_bits: list[int], b_bits: list[int]
) -> tuple[list[int], list[int]]:
    """Signed 64-bit division: sign-normalize, call udiv_64, restore sign.

    AArch64 SDIV truncates toward zero.
    Division by zero returns 0 per AArch64 spec.

    Algorithm
    ─────────
    1. Record signs of a and b
    2. Negate any negatives (NOT + 1 via gate-level add_64bit with carry=1)
    3. Perform unsigned division
    4. If result should be negative, negate the quotient

    Example
    ───────
    >>> a = int_to_bits(-100 & 0xFFFFFFFFFFFFFFFF, 64)
    >>> b = int_to_bits(7, 64)
    >>> q, r = sdiv_64(a, b)
    >>> import ctypes; ctypes.c_int64(bits_to_int(q)).value
    -14
    """
    from logic_gates import AND

    if AND(compute_zero(b_bits), 1):
        return [0] * 64, [0] * 64

    a_sign = a_bits[63]
    b_sign = b_bits[63]

    # Compute |a|
    if AND(a_sign, 1):
        a_abs, _, _ = add_64bit([NOT(x) for x in a_bits], int_to_bits(1, 64), 0)
    else:
        a_abs = a_bits[:]

    # Compute |b|
    if AND(b_sign, 1):
        b_abs, _, _ = add_64bit([NOT(x) for x in b_bits], int_to_bits(1, 64), 0)
    else:
        b_abs = b_bits[:]

    q, r = udiv_64(a_abs, b_abs)

    # If exactly one operand was negative, negate the quotient
    from logic_gates import XOR as _XOR
    result_neg = _XOR(a_sign, b_sign)
    if AND(result_neg, 1):
        q, _, _ = add_64bit([NOT(x) for x in q], int_to_bits(1, 64), 0)

    return q, r
