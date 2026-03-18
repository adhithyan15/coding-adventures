"""IEEE 754 encoding and decoding — converting between Python floats and bits.

=== How does a computer store 3.14? ===

When you write `x = 3.14` in Python, the computer stores it as 32 (or 64) bits
following the IEEE 754 standard. This module converts between Python's native
float representation and our explicit bit-level representation (FloatBits).

=== Encoding: float -> bits ===

For FP32, we use Python's `struct` module to get the exact same bit pattern
that the hardware uses. For FP16 and BF16, we manually extract the bits
because Python doesn't natively support these formats.

=== Special values in IEEE 754 ===

IEEE 754 reserves certain bit patterns for special values:

    Exponent      Mantissa    Meaning
    ----------    --------    -------
    All 1s        All 0s      +/- Infinity
    All 1s        Non-zero    NaN (Not a Number)
    All 0s        All 0s      +/- Zero
    All 0s        Non-zero    Denormalized number (very small, near zero)
    Other         Any         Normal number

These special values allow floating-point to handle edge cases gracefully:
- 1.0 / 0.0 = Inf (not a crash!)
- 0.0 / 0.0 = NaN (undefined, but doesn't crash)
- Denormals allow "gradual underflow" near zero
"""

from __future__ import annotations

import math
import struct

from fp_arithmetic._gates import AND, OR

from fp_arithmetic.formats import BF16, FP16, FP32, FloatBits, FloatFormat


# ---------------------------------------------------------------------------
# Helper: integer <-> bit list conversions
# ---------------------------------------------------------------------------


def _int_to_bits_msb(value: int, width: int) -> list[int]:
    """Convert a non-negative integer to a list of bits, MSB first.

    This is the fundamental conversion between Python's arbitrary-precision
    integers and our bit-level representation.

    Example:
        >>> _int_to_bits_msb(5, 8)
        [0, 0, 0, 0, 0, 1, 0, 1]
        #  128 64 32 16  8  4  2  1
        #                   4     1  = 5

    How it works:
        We check each bit position from the most significant (leftmost) to
        the least significant (rightmost). For each position i (counting from
        width-1 down to 0), we check if that bit is set using a right-shift
        and AND with 1.

    Args:
        value: The integer to convert (must be >= 0).
        width: The number of bits in the output list.

    Returns:
        A list of 0s and 1s, MSB first, with exactly `width` elements.
    """
    return [(value >> (width - 1 - i)) & 1 for i in range(width)]


def _bits_msb_to_int(bits: list[int]) -> int:
    """Convert a list of bits (MSB first) back to a non-negative integer.

    This is the inverse of _int_to_bits_msb.

    Example:
        >>> _bits_msb_to_int([0, 0, 0, 0, 0, 1, 0, 1])
        5
        #  Each bit contributes: bit_value * 2^position
        #  0*128 + 0*64 + 0*32 + 0*16 + 0*8 + 1*4 + 0*2 + 1*1 = 5

    How it works:
        We iterate through the bits from MSB to LSB. For each bit, we shift
        the accumulator left by 1 (multiply by 2) and OR in the new bit.
        This is equivalent to: sum(bit * 2^(width-1-i) for i, bit in enumerate(bits))

    Args:
        bits: List of 0s and 1s, MSB first.

    Returns:
        The integer value represented by the bits.
    """
    result = 0
    for bit in bits:
        result = (result << 1) | bit
    return result


# ---------------------------------------------------------------------------
# Encoding: Python float -> FloatBits
# ---------------------------------------------------------------------------


def float_to_bits(value: float, fmt: FloatFormat = FP32) -> FloatBits:
    """Convert a Python float to its IEEE 754 bit representation.

    === How FP32 encoding works (using struct) ===

    For FP32, we leverage Python's `struct` module which gives us access to the
    exact bit pattern that the hardware uses:

        struct.pack('!f', 3.14) -> b'\\x40\\x48\\xf5\\xc3'

    The '!f' format means: big-endian ('!') single-precision float ('f').
    We then unpack those 4 bytes as a 32-bit unsigned integer to get the raw bits.

    === How FP16/BF16 encoding works (manual) ===

    For FP16 and BF16, Python doesn't have native support, so we:
    1. First encode as FP32 (which we know is exact for the hardware)
    2. Extract the sign, exponent, and mantissa from the FP32 encoding
    3. Re-encode into the target format, adjusting exponent bias and
       truncating the mantissa

    === Worked example: encoding 3.14 as FP32 ===

        3.14 in binary: 11.00100011110101110000101...
        Normalized:     1.100100011110101110000101... x 2^1

        Sign:     0 (positive)
        Exponent: 1 + 127 (bias) = 128 = 10000000 in binary
        Mantissa: 10010001111010111000010 (23 bits after the implicit 1)
                  ^-- note: the leading 1 is NOT stored

        Packed: 0 10000000 10010001111010111000011
                s exponent         mantissa

    Args:
        value: The Python float to encode.
        fmt: The target format (FP32, FP16, or BF16). Default is FP32.

    Returns:
        FloatBits with the sign, exponent, and mantissa bit lists.

    Example:
        >>> bits = float_to_bits(1.0, FP32)
        >>> bits.sign
        0
        >>> bits.exponent  # 127 in binary = 01111111
        [0, 1, 1, 1, 1, 1, 1, 1]
        >>> bits.mantissa  # 1.000... so mantissa is all zeros
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    """
    # --- Handle NaN specially ---
    # Python has float('nan'), and IEEE 754 defines NaN as exponent=all-1s,
    # mantissa=non-zero. We use a "quiet NaN" with the MSB of mantissa set to 1.
    if math.isnan(value):
        return FloatBits(
            sign=0,
            exponent=[1] * fmt.exponent_bits,
            mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
            fmt=fmt,
        )

    # --- Handle Infinity ---
    # +Inf and -Inf: exponent=all-1s, mantissa=all-0s.
    if math.isinf(value):
        sign = 0 if value > 0 else 1
        return FloatBits(
            sign=sign,
            exponent=[1] * fmt.exponent_bits,
            mantissa=[0] * fmt.mantissa_bits,
            fmt=fmt,
        )

    # --- FP32: use struct for hardware-exact encoding ---
    if fmt == FP32:
        # struct.pack('!f', value) gives 4 bytes in big-endian IEEE 754.
        # struct.unpack('!I', ...) reads those bytes as an unsigned 32-bit int.
        packed = struct.pack("!f", value)
        int_bits = struct.unpack("!I", packed)[0]

        # Extract the three fields using bit shifts and masks:
        #   Bit 31:     sign
        #   Bits 30-23: exponent (8 bits)
        #   Bits 22-0:  mantissa (23 bits)
        sign = (int_bits >> 31) & 1
        exp_int = (int_bits >> 23) & 0xFF  # 8 bits
        mant_int = int_bits & 0x7FFFFF  # 23 bits

        return FloatBits(
            sign=sign,
            exponent=_int_to_bits_msb(exp_int, 8),
            mantissa=_int_to_bits_msb(mant_int, 23),
            fmt=FP32,
        )

    # --- FP16 and BF16: manual conversion from FP32 ---
    #
    # Strategy: encode as FP32 first, then convert.
    # This handles all the tricky cases (denormals, rounding) correctly
    # because the FP32 encoding uses hardware-exact struct.pack.

    fp32_bits = float_to_bits(value, FP32)
    fp32_exp = _bits_msb_to_int(fp32_bits.exponent)
    fp32_mant = _bits_msb_to_int(fp32_bits.mantissa)
    sign = fp32_bits.sign

    # --- Handle zero ---
    if fp32_exp == 0 and fp32_mant == 0:
        return FloatBits(
            sign=sign,
            exponent=[0] * fmt.exponent_bits,
            mantissa=[0] * fmt.mantissa_bits,
            fmt=fmt,
        )

    # --- Compute the true (unbiased) exponent ---
    # For normal FP32 numbers: true_exp = stored_exp - 127
    # For denormal FP32 numbers: true_exp = 1 - 127 = -126
    if fp32_exp == 0:
        # Denormal in FP32: true exponent is -126, implicit bit is 0
        true_exp = 1 - FP32.bias  # = -126
        # Denormal mantissa: no implicit 1, so full mantissa is 0.mantissa
        full_mantissa = fp32_mant
        implicit_bit = 0
    else:
        true_exp = fp32_exp - FP32.bias
        # Normal: full mantissa includes the implicit leading 1
        full_mantissa = (1 << FP32.mantissa_bits) | fp32_mant
        implicit_bit = 1

    # --- Map to target format ---
    target_exp = true_exp + fmt.bias
    max_exp = (1 << fmt.exponent_bits) - 1  # All 1s = special

    # --- Overflow: exponent too large for target format -> Infinity ---
    if target_exp >= max_exp:
        return FloatBits(
            sign=sign,
            exponent=[1] * fmt.exponent_bits,
            mantissa=[0] * fmt.mantissa_bits,
            fmt=fmt,
        )

    # --- Normal case: exponent fits in target format ---
    if target_exp > 0:
        # Truncate mantissa from 23 bits to fmt.mantissa_bits
        # We take the top bits and apply round-to-nearest-even
        if fmt.mantissa_bits < FP32.mantissa_bits:
            shift = FP32.mantissa_bits - fmt.mantissa_bits
            truncated = fp32_mant >> shift
            # Round-to-nearest-even: check the bit just below the truncation point
            round_bit = (fp32_mant >> (shift - 1)) & 1
            sticky = fp32_mant & ((1 << (shift - 1)) - 1)
            if round_bit and (sticky or (truncated & 1)):
                truncated += 1
                # Rounding overflow: mantissa exceeded max, carry into exponent
                if truncated >= (1 << fmt.mantissa_bits):
                    truncated = 0
                    target_exp += 1
                    if target_exp >= max_exp:
                        return FloatBits(
                            sign=sign,
                            exponent=[1] * fmt.exponent_bits,
                            mantissa=[0] * fmt.mantissa_bits,
                            fmt=fmt,
                        )
        else:
            truncated = fp32_mant << (fmt.mantissa_bits - FP32.mantissa_bits)

        return FloatBits(
            sign=sign,
            exponent=_int_to_bits_msb(target_exp, fmt.exponent_bits),
            mantissa=_int_to_bits_msb(truncated, fmt.mantissa_bits),
            fmt=fmt,
        )

    # --- Underflow: number is too small for normal representation ---
    # It might still be representable as a denormal in the target format.
    # Denormal: exponent=0, mantissa encodes the value directly.
    #
    # The shift amount tells us how many bits we lose going denormal.
    denorm_shift = 1 - target_exp  # how far below the minimum normal exponent

    if denorm_shift > fmt.mantissa_bits:
        # Too small even for denormal -> flush to zero
        return FloatBits(
            sign=sign,
            exponent=[0] * fmt.exponent_bits,
            mantissa=[0] * fmt.mantissa_bits,
            fmt=fmt,
        )

    # Shift the full mantissa right to create a denormal
    # full_mantissa has (mantissa_bits + 1) bits (including implicit 1)
    # We need to fit it into fmt.mantissa_bits after shifting
    denorm_mant = full_mantissa >> (denorm_shift + FP32.mantissa_bits - fmt.mantissa_bits)

    return FloatBits(
        sign=sign,
        exponent=[0] * fmt.exponent_bits,
        mantissa=_int_to_bits_msb(denorm_mant & ((1 << fmt.mantissa_bits) - 1), fmt.mantissa_bits),
        fmt=fmt,
    )


# ---------------------------------------------------------------------------
# Decoding: FloatBits -> Python float
# ---------------------------------------------------------------------------


def bits_to_float(bits: FloatBits) -> float:
    """Convert an IEEE 754 bit representation back to a Python float.

    === How decoding works ===

    For FP32, we reconstruct the 32-bit integer and use struct.unpack to get
    the exact Python float. For FP16/BF16, we manually compute the value
    using the formula:

        value = (-1)^sign x 2^(exponent - bias) x 1.mantissa

    === Worked example: decoding FP32 bits for 3.14 ===

        Sign: 0 -> positive
        Exponent: 10000000 -> 128 -> true exponent = 128 - 127 = 1
        Mantissa: 10010001111010111000011

        Value = +1.0 x 2^1 x (1 + 0.5 + 0.0625 + ...)
              = 2 x 1.5700000524520874
              = 3.140000104904175

    Args:
        bits: The FloatBits to decode.

    Returns:
        The Python float value.

    Example:
        >>> bits = float_to_bits(3.14, FP32)
        >>> bits_to_float(bits)
        3.140000104904175
    """
    exp_int = _bits_msb_to_int(bits.exponent)
    mant_int = _bits_msb_to_int(bits.mantissa)
    max_exp = (1 << bits.fmt.exponent_bits) - 1

    # --- Special values ---

    # NaN: exponent all 1s, mantissa non-zero
    if exp_int == max_exp and mant_int != 0:
        return float("nan")

    # Infinity: exponent all 1s, mantissa all zeros
    if exp_int == max_exp and mant_int == 0:
        return float("-inf") if bits.sign == 1 else float("inf")

    # Zero: exponent all 0s, mantissa all zeros
    if exp_int == 0 and mant_int == 0:
        # IEEE 754 has both +0 and -0
        return -0.0 if bits.sign == 1 else 0.0

    # --- For FP32, use struct for exact conversion ---
    if bits.fmt == FP32:
        # Reconstruct the 32-bit integer
        int_bits = (bits.sign << 31) | (exp_int << 23) | mant_int
        packed = struct.pack("!I", int_bits)
        return struct.unpack("!f", packed)[0]

    # --- For FP16/BF16, compute the float value manually ---

    # Denormalized: exponent=0, implicit bit is 0
    if exp_int == 0:
        # value = (-1)^sign x 2^(1-bias) x 0.mantissa
        # The mantissa represents a fraction: mant_int / 2^mantissa_bits
        true_exp = 1 - bits.fmt.bias
        mantissa_value = mant_int / (1 << bits.fmt.mantissa_bits)
    else:
        # Normal: implicit leading 1
        # value = (-1)^sign x 2^(exponent-bias) x 1.mantissa
        true_exp = exp_int - bits.fmt.bias
        mantissa_value = 1.0 + mant_int / (1 << bits.fmt.mantissa_bits)

    value = mantissa_value * (2.0 ** true_exp)
    if bits.sign == 1:
        value = -value

    return value


# ---------------------------------------------------------------------------
# Special value detection — using logic gates
# ---------------------------------------------------------------------------
# These functions detect special IEEE 754 values by examining the bit pattern.
# We use AND and OR from logic_gates to check bit fields, staying true to
# the "built from gates" philosophy.


def _all_ones(bits: list[int]) -> bool:
    """Check if all bits in a list are 1, using AND gates.

    In hardware, this would be a wide AND gate:
        all_ones = AND(bit[0], AND(bit[1], AND(bit[2], ...)))

    If ALL bits are 1, the final AND output is 1.
    If ANY bit is 0, the chain collapses to 0.

    Example:
        >>> _all_ones([1, 1, 1, 1])
        True
        >>> _all_ones([1, 0, 1, 1])
        False
    """
    result = bits[0]
    for i in range(1, len(bits)):
        result = AND(result, bits[i])
    return result == 1


def _all_zeros(bits: list[int]) -> bool:
    """Check if all bits in a list are 0, using OR gates then NOT.

    In hardware: NOR across all bits.
        any_one = OR(bit[0], OR(bit[1], OR(bit[2], ...)))
        all_zeros = NOT(any_one)

    If ANY bit is 1, the OR chain produces 1, and we return False.
    If ALL bits are 0, the OR chain produces 0, and we return True.

    Example:
        >>> _all_zeros([0, 0, 0, 0])
        True
        >>> _all_zeros([0, 0, 1, 0])
        False
    """
    result = bits[0]
    for i in range(1, len(bits)):
        result = OR(result, bits[i])
    return result == 0


def is_nan(bits: FloatBits) -> bool:
    """Check if a FloatBits represents NaN (Not a Number).

    NaN is defined as: exponent = all 1s AND mantissa != all 0s.

    In IEEE 754, NaN is the result of undefined operations like:
        0 / 0, Inf - Inf, sqrt(-1)

    There are two types of NaN:
    - Quiet NaN (qNaN): mantissa MSB = 1, propagates silently
    - Signaling NaN (sNaN): mantissa MSB = 0, raises exception

    We don't distinguish between them here.

    Args:
        bits: The FloatBits to check.

    Returns:
        True if the value is NaN.

    Example:
        >>> is_nan(float_to_bits(float('nan')))
        True
        >>> is_nan(float_to_bits(1.0))
        False
    """
    return _all_ones(bits.exponent) and not _all_zeros(bits.mantissa)


def is_inf(bits: FloatBits) -> bool:
    """Check if a FloatBits represents Infinity (+Inf or -Inf).

    Infinity is defined as: exponent = all 1s AND mantissa = all 0s.

    IEEE 754 uses Infinity to represent overflow results:
        1e38 * 10 = +Inf (in FP32)
        -1.0 / 0.0 = -Inf

    Args:
        bits: The FloatBits to check.

    Returns:
        True if the value is +Inf or -Inf.

    Example:
        >>> is_inf(float_to_bits(float('inf')))
        True
        >>> is_inf(float_to_bits(float('-inf')))
        True
        >>> is_inf(float_to_bits(1.0))
        False
    """
    return _all_ones(bits.exponent) and _all_zeros(bits.mantissa)


def is_zero(bits: FloatBits) -> bool:
    """Check if a FloatBits represents zero (+0 or -0).

    Zero is defined as: exponent = all 0s AND mantissa = all 0s.

    IEEE 754 has both +0 and -0. They compare equal (0.0 == -0.0 is True
    in Python), but they are different bit patterns. Having -0 is important
    for preserving the sign through operations like 1.0 / -Inf = -0.

    Args:
        bits: The FloatBits to check.

    Returns:
        True if the value is +0 or -0.

    Example:
        >>> is_zero(float_to_bits(0.0))
        True
        >>> is_zero(float_to_bits(-0.0))
        True
        >>> is_zero(float_to_bits(1.0))
        False
    """
    return _all_zeros(bits.exponent) and _all_zeros(bits.mantissa)


def is_denormalized(bits: FloatBits) -> bool:
    """Check if a FloatBits represents a denormalized (subnormal) number.

    Denormalized is defined as: exponent = all 0s AND mantissa != all 0s.

    === What are denormalized numbers? ===

    Normal IEEE 754 numbers have an implicit leading 1: the value is 1.mantissa.
    But what about very small numbers close to zero? The smallest normal FP32
    number is about 1.18e-38. Without denormals, the next smaller value would
    be 0 — a sudden jump called "the underflow gap."

    Denormalized numbers fill this gap. When the exponent is all zeros, the
    implicit bit becomes 0 instead of 1, and the true exponent is fixed at
    (1 - bias). This allows gradual underflow: numbers smoothly approach zero
    rather than jumping to it.

        Normal:     1.mantissa x 2^(exp-bias)     (implicit 1)
        Denormal:   0.mantissa x 2^(1-bias)       (implicit 0)

    The smallest positive denormal in FP32 is:
        0.00000000000000000000001 x 2^(-126) = 2^(-149) ~ 1.4e-45

    Args:
        bits: The FloatBits to check.

    Returns:
        True if the value is denormalized.

    Example:
        >>> # The smallest positive FP32 denormal
        >>> tiny = FloatBits(sign=0, exponent=[0]*8,
        ...     mantissa=[0]*22 + [1], fmt=FP32)
        >>> is_denormalized(tiny)
        True
    """
    return _all_zeros(bits.exponent) and not _all_zeros(bits.mantissa)
