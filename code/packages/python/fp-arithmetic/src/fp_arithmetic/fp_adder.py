"""Floating-point addition and subtraction — built from logic gates.

=== How FP addition works at the hardware level ===

Adding two floating-point numbers is surprisingly complex compared to integer
addition. The core difficulty is that the two numbers might have very different
exponents, so their mantissas are "misaligned" and must be shifted before they
can be added.

Consider adding 1.5 + 0.125 in decimal scientific notation:
    1.5 x 10^0  +  1.25 x 10^-1

You can't just add 1.5 + 1.25 because they have different exponents. First,
you align them to the same exponent:
    1.5   x 10^0
    0.125 x 10^0   (shifted 1.25 right by 1 decimal place)
    ─────────────
    1.625 x 10^0

Binary FP addition follows the exact same principle, but with binary mantissas
and power-of-2 exponents.

=== The five steps of FP addition ===

    Step 1: Compare exponents
            Subtract exponents to find the difference.
            The number with the smaller exponent gets shifted.

    Step 2: Align mantissas
            Shift the smaller number's mantissa right by the exponent
            difference. This is like converting 0.125 to line up with 1.5.

    Step 3: Add or subtract mantissas
            If signs are the same: add mantissas
            If signs differ: subtract the smaller from the larger

    Step 4: Normalize
            The result might not be in 1.xxx form. Adjust:
            - If overflow (10.xxx): shift right, increment exponent
            - If underflow (0.0xxx): shift left, decrement exponent

    Step 5: Round
            The result might have more bits than the format allows.
            Round to fit, using "round to nearest even" (banker's rounding).

=== Why this is slow but clear ===

A real hardware FPU does all of this in 1-3 clock cycles using parallel
circuits (barrel shifters, leading-zero anticipators, etc.). Our implementation
is sequential and uses simple loops, which is much slower but much easier to
understand. Every step maps directly to the algorithm described above.
"""

from __future__ import annotations

from fp_arithmetic._gates import AND, NOT, OR, XOR, ripple_carry_adder

from fp_arithmetic.formats import FloatBits, FloatFormat
from fp_arithmetic.ieee754 import (
    _bits_msb_to_int,
    _int_to_bits_msb,
    is_inf,
    is_nan,
    is_zero,
)


# ---------------------------------------------------------------------------
# Helper: two's complement subtraction using ripple_carry_adder
# ---------------------------------------------------------------------------


def _subtract_unsigned(a_bits: list[int], b_bits: list[int]) -> tuple[list[int], int]:
    """Subtract two unsigned numbers: a - b using two's complement.

    To compute a - b, we use the identity:
        a - b = a + NOT(b) + 1

    This is how ALL subtraction works in binary hardware. There is no
    dedicated subtraction circuit — it's always addition with negation.

    Args:
        a_bits: First number as bits, MSB first.
        b_bits: Second number as bits, MSB first.

    Returns:
        (result_bits_msb, borrow) where borrow=1 if b > a.
        Result bits are MSB first.
    """
    width = len(a_bits)
    # Convert MSB-first to LSB-first for ripple_carry_adder
    a_lsb = list(reversed(a_bits))
    b_lsb = list(reversed(b_bits))

    # NOT(b) = one's complement
    b_inv_lsb = [NOT(bit) for bit in b_lsb]

    # a + NOT(b) + 1 = a - b in two's complement
    result_lsb, carry = ripple_carry_adder(a_lsb, b_inv_lsb, carry_in=1)

    # carry=1 means no borrow (result is non-negative)
    # carry=0 means borrow (result is negative, i.e., b > a)
    borrow = NOT(carry)

    return list(reversed(result_lsb)), borrow


# ---------------------------------------------------------------------------
# Helper: shift mantissa right by N positions
# ---------------------------------------------------------------------------


def _shift_right(bits: list[int], amount: int) -> list[int]:
    """Shift a bit list right by `amount` positions, filling with zeros.

    In hardware, this would be a barrel shifter built from layers of MUX gates.
    Each layer shifts by a power of 2 (1, 2, 4, 8, ...) controlled by one bit
    of the shift amount. Our implementation is simpler: just a loop.

    Example:
        >>> _shift_right([1, 0, 1, 1], 2)
        [0, 0, 1, 0]
        #  The bits shifted in from the left are 0s.
        #  The bits shifted out on the right are lost.

    The bits that fall off the right are lost (truncated). In a full hardware
    implementation, we'd keep track of the "sticky bit" (OR of all lost bits)
    for rounding. We handle rounding separately.

    Args:
        bits: Bit list, MSB first.
        amount: Number of positions to shift right.

    Returns:
        New bit list with zeros shifted in from the left.
    """
    if amount <= 0:
        return list(bits)
    if amount >= len(bits):
        return [0] * len(bits)
    return [0] * amount + bits[: len(bits) - amount]


def _shift_left(bits: list[int], amount: int) -> list[int]:
    """Shift a bit list left by `amount` positions, filling with zeros.

    Example:
        >>> _shift_left([1, 0, 1, 1], 2)
        [1, 1, 0, 0]

    Args:
        bits: Bit list, MSB first.
        amount: Number of positions to shift left.

    Returns:
        New bit list with zeros shifted in from the right.
    """
    if amount <= 0:
        return list(bits)
    if amount >= len(bits):
        return [0] * len(bits)
    return bits[amount:] + [0] * amount


# ---------------------------------------------------------------------------
# Helper: find the position of the leading 1 (most significant set bit)
# ---------------------------------------------------------------------------


def _find_leading_one(bits: list[int]) -> int:
    """Find the index of the first 1 bit in a list (MSB first).

    In hardware, this is called a "leading-one detector" or "priority encoder."
    It's built from a tree of OR gates. Our implementation is a simple scan.

    Returns -1 if all bits are 0.

    Example:
        >>> _find_leading_one([0, 0, 1, 0, 1])
        2
        >>> _find_leading_one([0, 0, 0, 0, 0])
        -1
    """
    for i, bit in enumerate(bits):
        if bit == 1:
            return i
    return -1


# ---------------------------------------------------------------------------
# Helper: add two bit lists (MSB first) using ripple_carry_adder
# ---------------------------------------------------------------------------


def _add_bits_msb(a: list[int], b: list[int], carry_in: int = 0) -> tuple[list[int], int]:
    """Add two bit lists (MSB first) using ripple_carry_adder.

    ripple_carry_adder expects LSB-first, so we reverse, add, and reverse back.

    Args:
        a: First operand, MSB first.
        b: Second operand, MSB first. Must be same length as a.
        carry_in: Initial carry bit (0 or 1).

    Returns:
        (result_msb, carry_out) where result is MSB first.
    """
    a_lsb = list(reversed(a))
    b_lsb = list(reversed(b))
    result_lsb, carry = ripple_carry_adder(a_lsb, b_lsb, carry_in=carry_in)
    return list(reversed(result_lsb)), carry


# ---------------------------------------------------------------------------
# Core: fp_add — floating-point addition from logic gates
# ---------------------------------------------------------------------------


def fp_add(a: FloatBits, b: FloatBits) -> FloatBits:
    """Add two floating-point numbers using logic gates.

    This implements the full IEEE 754 addition algorithm:
    1. Handle special cases (NaN, Inf, Zero)
    2. Compare exponents
    3. Align mantissas
    4. Add/subtract mantissas
    5. Normalize result
    6. Round to nearest even

    === Worked example: 1.5 + 0.25 in FP32 ===

        1.5 = 1.1 x 2^0    -> exp=127, mant=10000...0
        0.25 = 1.0 x 2^-2   -> exp=125, mant=00000...0

        Step 1: exp_diff = 127 - 125 = 2 (b has smaller exponent)
        Step 2: Shift b's mantissa right by 2:
                1.10000...0  (a, with implicit 1)
                0.01000...0  (b, shifted right by 2)
        Step 3: Add:  1.10000...0 + 0.01000...0 = 1.11000...0
        Step 4: Already normalized (starts with 1.)
        Step 5: No rounding needed (exact)
        Result: 1.11 x 2^0 = 1.75 (correct!)

    Args:
        a: First operand as FloatBits.
        b: Second operand as FloatBits. Must use the same FloatFormat as a.

    Returns:
        The sum as FloatBits in the same format.
    """
    fmt = a.fmt

    # ===================================================================
    # Step 0: Handle special cases
    # ===================================================================
    # IEEE 754 defines strict rules for special values:
    #   NaN + anything = NaN
    #   Inf + (-Inf) = NaN
    #   Inf + x = Inf (for finite x)
    #   0 + x = x

    # NaN propagation: any NaN input produces NaN output
    if is_nan(a) or is_nan(b):
        return FloatBits(
            sign=0,
            exponent=[1] * fmt.exponent_bits,
            mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
            fmt=fmt,
        )

    # Infinity handling
    a_inf = is_inf(a)
    b_inf = is_inf(b)
    if a_inf and b_inf:
        # Inf + Inf = Inf (same sign) or NaN (different signs)
        if a.sign == b.sign:
            return FloatBits(
                sign=a.sign,
                exponent=[1] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )
        else:
            # Inf + (-Inf) = NaN
            return FloatBits(
                sign=0,
                exponent=[1] * fmt.exponent_bits,
                mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
                fmt=fmt,
            )
    if a_inf:
        return a
    if b_inf:
        return b

    # Zero handling
    a_zero = is_zero(a)
    b_zero = is_zero(b)
    if a_zero and b_zero:
        # +0 + +0 = +0, -0 + -0 = -0, +0 + -0 = +0
        result_sign = AND(a.sign, b.sign)
        return FloatBits(
            sign=result_sign,
            exponent=[0] * fmt.exponent_bits,
            mantissa=[0] * fmt.mantissa_bits,
            fmt=fmt,
        )
    if a_zero:
        return b
    if b_zero:
        return a

    # ===================================================================
    # Step 1: Extract exponents and mantissas as integers
    # ===================================================================
    #
    # We work with extended mantissas that include the implicit leading bit.
    # For normal numbers, this is 1; for denormals, it's 0.
    #
    # We also add extra guard bits for rounding precision. The guard bits
    # are: Guard (G), Round (R), and Sticky (S) — 3 extra bits that capture
    # information about the bits that would otherwise be lost during shifting.
    #
    #   [implicit_1] [mantissa bits] [G] [R] [S]
    #    1 bit        N bits          1   1   1

    exp_a = _bits_msb_to_int(a.exponent)
    exp_b = _bits_msb_to_int(b.exponent)
    mant_a = _bits_msb_to_int(a.mantissa)
    mant_b = _bits_msb_to_int(b.mantissa)

    # Add implicit leading 1 for normal numbers (exponent != 0)
    # For denormals (exponent == 0), the implicit bit is 0
    if exp_a != 0:
        mant_a = (1 << fmt.mantissa_bits) | mant_a
    else:
        exp_a = 1  # Denormal true exponent = 1 - bias, stored as 1 for alignment
    if exp_b != 0:
        mant_b = (1 << fmt.mantissa_bits) | mant_b
    else:
        exp_b = 1

    # Add 3 guard bits (shift left by 3) for rounding precision
    guard_bits = 3
    mant_a <<= guard_bits
    mant_b <<= guard_bits

    # ===================================================================
    # Step 2: Align mantissas by shifting the smaller one right
    # ===================================================================
    #
    # If exp_a > exp_b, then b has a smaller magnitude per mantissa bit,
    # so we shift b's mantissa right by (exp_a - exp_b) positions.
    #
    # Example:
    #   1.5 = 1.1 x 2^0   (exp=127)
    #   0.25 = 1.0 x 2^-2  (exp=125)
    #   Shift 0.25's mantissa right by 2: 001.0 -> 0.01

    if exp_a >= exp_b:
        exp_diff = exp_a - exp_b
        # Before shifting, save the sticky bits (all bits that will be shifted out)
        if exp_diff > 0 and exp_diff < (fmt.mantissa_bits + 1 + guard_bits):
            # Sticky = OR of all bits that get shifted away
            shifted_out = mant_b & ((1 << exp_diff) - 1)
            sticky = 1 if shifted_out != 0 else 0
        else:
            sticky = 1 if mant_b != 0 and exp_diff > 0 else 0
        mant_b >>= exp_diff
        if sticky and exp_diff > 0:
            mant_b |= 1  # Set the sticky bit (LSB)
        result_exp = exp_a
    else:
        exp_diff = exp_b - exp_a
        if exp_diff > 0 and exp_diff < (fmt.mantissa_bits + 1 + guard_bits):
            shifted_out = mant_a & ((1 << exp_diff) - 1)
            sticky = 1 if shifted_out != 0 else 0
        else:
            sticky = 1 if mant_a != 0 and exp_diff > 0 else 0
        mant_a >>= exp_diff
        if sticky and exp_diff > 0:
            mant_a |= 1
        result_exp = exp_b

    # ===================================================================
    # Step 3: Add or subtract mantissas based on signs
    # ===================================================================
    #
    # If signs are the same: add mantissas, keep the sign
    # If signs differ: subtract the smaller from the larger
    #
    # In hardware, subtraction is done by adding the two's complement.

    if a.sign == b.sign:
        # Same sign: simple addition
        result_mant = mant_a + mant_b
        result_sign = a.sign
    else:
        # Different signs: subtract smaller from larger
        if mant_a >= mant_b:
            result_mant = mant_a - mant_b
            result_sign = a.sign
        else:
            result_mant = mant_b - mant_a
            result_sign = b.sign

    # ===================================================================
    # Step 4: Handle zero result
    # ===================================================================
    if result_mant == 0:
        return FloatBits(
            sign=0,  # +0 by convention
            exponent=[0] * fmt.exponent_bits,
            mantissa=[0] * fmt.mantissa_bits,
            fmt=fmt,
        )

    # ===================================================================
    # Step 5: Normalize the result
    # ===================================================================
    #
    # The result mantissa should be in the form 1.xxxx (the leading 1 in
    # position mantissa_bits + guard_bits).
    #
    # If the result is too large (e.g., 10.xxx from overflow), shift right
    # and increment the exponent.
    #
    # If the result is too small (e.g., 0.001xxx from cancellation), shift
    # left and decrement the exponent.

    # The "normal" position for the leading 1 is at bit (mantissa_bits + guard_bits)
    normal_pos = fmt.mantissa_bits + guard_bits

    # Find where the leading 1 actually is
    leading_pos = result_mant.bit_length() - 1

    if leading_pos > normal_pos:
        # Overflow: shift right to normalize
        shift_amount = leading_pos - normal_pos
        # Save bits being shifted out for rounding
        lost_bits = result_mant & ((1 << shift_amount) - 1)
        result_mant >>= shift_amount
        if lost_bits != 0:
            result_mant |= 1  # sticky
        result_exp += shift_amount
    elif leading_pos < normal_pos:
        # Underflow: shift left to normalize
        shift_amount = normal_pos - leading_pos
        if result_exp - shift_amount >= 1:
            result_mant <<= shift_amount
            result_exp -= shift_amount
        else:
            # Can't shift all the way — result becomes denormal
            actual_shift = result_exp - 1
            if actual_shift > 0:
                result_mant <<= actual_shift
            result_exp = 0

    # ===================================================================
    # Step 6: Round to nearest even
    # ===================================================================
    #
    # We have 3 extra guard bits beyond the mantissa. The rounding decision
    # depends on these bits:
    #
    #   [mantissa bits] [G] [R] [S]
    #                    ^   ^   ^
    #                    |   |   |
    #                    |   |   +-- sticky: OR of all bits below R
    #                    |   +------ round: the bit just below the last mantissa bit
    #                    +---------- guard: the first extra bit
    #
    # Round to nearest even rules:
    #   - If GRS = 0xx: round down (truncate)
    #   - If GRS = 100: round to even (round up if mantissa LSB is 1)
    #   - If GRS = 101, 110, 111: round up

    guard = (result_mant >> (guard_bits - 1)) & 1
    round_bit = (result_mant >> (guard_bits - 2)) & 1
    sticky_bit = result_mant & ((1 << (guard_bits - 2)) - 1)
    sticky_bit = 1 if sticky_bit != 0 else 0

    # Remove guard bits
    result_mant >>= guard_bits

    # Apply rounding
    if guard == 1:
        if round_bit == 1 or sticky_bit == 1:
            # Round up
            result_mant += 1
        elif (result_mant & 1) == 1:
            # Tie-breaking: round to even (round up if LSB is 1)
            result_mant += 1

    # Check if rounding caused overflow
    if result_mant >= (1 << (fmt.mantissa_bits + 1)):
        result_mant >>= 1
        result_exp += 1

    # ===================================================================
    # Step 7: Handle exponent overflow/underflow
    # ===================================================================
    max_exp = (1 << fmt.exponent_bits) - 1

    if result_exp >= max_exp:
        # Overflow to infinity
        return FloatBits(
            sign=result_sign,
            exponent=[1] * fmt.exponent_bits,
            mantissa=[0] * fmt.mantissa_bits,
            fmt=fmt,
        )

    if result_exp <= 0:
        # Denormal or zero
        if result_exp < -(fmt.mantissa_bits):
            # Too small, flush to zero
            return FloatBits(
                sign=result_sign,
                exponent=[0] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )
        # Denormal: shift mantissa right, exponent stays at 0
        shift = 1 - result_exp
        result_mant >>= shift
        result_exp = 0

    # ===================================================================
    # Step 8: Pack the result
    # ===================================================================
    # Remove the implicit leading 1 (if normal)
    if result_exp > 0:
        result_mant &= (1 << fmt.mantissa_bits) - 1  # Remove implicit 1

    return FloatBits(
        sign=result_sign,
        exponent=_int_to_bits_msb(result_exp, fmt.exponent_bits),
        mantissa=_int_to_bits_msb(result_mant, fmt.mantissa_bits),
        fmt=fmt,
    )


# ---------------------------------------------------------------------------
# fp_sub — subtraction is just addition with a flipped sign
# ---------------------------------------------------------------------------


def fp_sub(a: FloatBits, b: FloatBits) -> FloatBits:
    """Subtract two floating-point numbers: a - b.

    === Why subtraction is trivial once you have addition ===

    In IEEE 754, a - b = a + (-b). To negate b, we just flip its sign bit.
    This is a single NOT gate in hardware — the cheapest possible operation.

    Then we feed the result into fp_add, which handles all the complexity
    of alignment, normalization, and rounding.

    Args:
        a: The minuend (what we subtract from).
        b: The subtrahend (what we subtract).

    Returns:
        a - b as FloatBits.

    Example:
        >>> # 3.0 - 1.0 = 2.0
        >>> a = float_to_bits(3.0)
        >>> b = float_to_bits(1.0)
        >>> result = fp_sub(a, b)
        >>> bits_to_float(result)
        2.0
    """
    # Flip b's sign bit using XOR (XOR with 1 flips a bit)
    neg_b = FloatBits(
        sign=XOR(b.sign, 1),
        exponent=b.exponent,
        mantissa=b.mantissa,
        fmt=b.fmt,
    )
    return fp_add(a, neg_b)


# ---------------------------------------------------------------------------
# fp_neg — negate a floating-point number
# ---------------------------------------------------------------------------


def fp_neg(a: FloatBits) -> FloatBits:
    """Negate a floating-point number: return -a.

    This is the simplest floating-point operation: just flip the sign bit.
    In hardware, it's literally one NOT gate (or XOR with 1).

    Note: neg(+0) = -0 and neg(-0) = +0. Both are valid IEEE 754 zeros.

    Args:
        a: The number to negate.

    Returns:
        -a as FloatBits.
    """
    return FloatBits(
        sign=XOR(a.sign, 1),
        exponent=a.exponent,
        mantissa=a.mantissa,
        fmt=a.fmt,
    )


# ---------------------------------------------------------------------------
# fp_abs — absolute value
# ---------------------------------------------------------------------------


def fp_abs(a: FloatBits) -> FloatBits:
    """Return the absolute value of a floating-point number.

    Even simpler than negation: just force the sign bit to 0.
    In hardware, this is done by AND-ing the sign bit with 0 (or simply
    not connecting the sign wire).

    Note: abs(NaN) is still NaN (with sign=0). This is the IEEE 754 behavior.

    Args:
        a: The input number.

    Returns:
        |a| as FloatBits.
    """
    return FloatBits(
        sign=0,
        exponent=a.exponent,
        mantissa=a.mantissa,
        fmt=a.fmt,
    )


# ---------------------------------------------------------------------------
# fp_compare — compare two floating-point numbers
# ---------------------------------------------------------------------------


def fp_compare(a: FloatBits, b: FloatBits) -> int:
    """Compare two floating-point numbers.

    Returns:
        -1 if a < b
         0 if a == b
         1 if a > b

    NaN comparisons always return 0 (unordered). This is a simplification;
    real IEEE 754 has a separate "unordered" result, but for our purposes
    returning 0 is sufficient.

    === How FP comparison works in hardware ===

    Floating-point comparison is more complex than integer comparison because:
    1. The sign bit inverts the ordering (negative numbers are "backwards")
    2. The exponent is more significant than the mantissa
    3. Special values (NaN, Inf, zero) need special handling

    For two positive normal numbers:
    - Compare exponents first (larger exponent = larger number)
    - If exponents equal, compare mantissas

    For mixed signs: positive > negative (always).
    For two negative numbers: comparison is reversed.

    Args:
        a: First operand.
        b: Second operand.

    Returns:
        -1, 0, or 1.
    """
    # NaN is unordered — any comparison involving NaN returns 0
    if is_nan(a) or is_nan(b):
        return 0

    # Handle zeros: +0 == -0
    if is_zero(a) and is_zero(b):
        return 0

    # Different signs: positive > negative
    if a.sign != b.sign:
        if is_zero(a):
            return 1 if b.sign == 1 else -1
        if is_zero(b):
            return -1 if a.sign == 1 else 1
        return -1 if a.sign == 1 else 1

    # Same sign: compare exponent, then mantissa
    exp_a = _bits_msb_to_int(a.exponent)
    exp_b = _bits_msb_to_int(b.exponent)
    mant_a = _bits_msb_to_int(a.mantissa)
    mant_b = _bits_msb_to_int(b.mantissa)

    if exp_a != exp_b:
        if a.sign == 0:
            return 1 if exp_a > exp_b else -1
        else:
            return -1 if exp_a > exp_b else 1

    if mant_a != mant_b:
        if a.sign == 0:
            return 1 if mant_a > mant_b else -1
        else:
            return -1 if mant_a > mant_b else 1

    return 0
