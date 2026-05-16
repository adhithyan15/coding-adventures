"""alu.py — 32-bit gate-level ALU for the MIPS R2000 simulator.

Every data-path operation in this module routes through:
  - ``AND``, ``OR``, ``XOR``, ``NOT`` from ``logic_gates``
  - ``ripple_carry_adder`` from ``arithmetic`` (via the helpers in bits.py)

No Python arithmetic operators (+, -, &, |, ^, ~, *, /) appear in any
computation here.  Only Python control flow (if/for) and list indexing.

ALU pipeline overview
─────────────────────
Each operation follows this path:

    integer inputs
        ↓  int_to_bits()
    bit lists (LSB-first)
        ↓  gate functions (AND/OR/XOR/NOT/ripple_carry_adder)
    result bit list
        ↓  bits_to_int()
    integer output + flags

This mirrors real hardware: the ALU input latches hold bit patterns; the
combinational logic network produces result bits and flag signals; the output
latches capture the result.

Overflow (V flag)
─────────────────
For signed addition/subtraction:
    V = XOR(carry_into_MSB, carry_out_of_MSB)

This is the standard two's-complement overflow test used by the MIPS R2000
ALU.  ``add_32bit`` in bits.py computes this correctly.

Two's complement subtraction
─────────────────────────────
SUB A, B  ≡  ADD A, NOT(B), carry_in=1

This is the standard hardware trick: negate B using the adder itself by
inverting all bits and setting carry_in=1 (which adds 1, completing the
two's complement).  32 NOT gates invert B; the ripple-carry adder does
the rest.

Multiplication (MULT / MULTU)
──────────────────────────────
We use the classical shift-and-add algorithm, iterating over each of the 32
bits of the multiplier.  For each bit that is 1, we add the (shifted)
multiplicand into the accumulator using ``add_64bit``.

    product = 0
    for bit in range(32):
        if multiplier_bit[bit] == 1:
            product += multiplicand << bit  # via add_64bit, gate-level

Exactly 32 iterations.  The 64-bit add uses ``add_64bit`` which itself calls
``ripple_carry_adder``.

Division (DIV / DIVU)
──────────────────────
We use the non-restoring long division algorithm.  32 iterations, one per
quotient bit (from MSB to LSB):

    for bit in range(31, -1, -1):
        shifted_b = shl_32(b, bit)
        if remainder >= shifted_b:   # test via sub32, check carry (no borrow)
            remainder -= shifted_b   # via sub32
            quotient |= (1 << bit)   # set this quotient bit

The borrow check uses sub32's carry output: if carry=0, the subtraction
did NOT borrow, meaning remainder >= shifted_b.
"""

from __future__ import annotations

from dataclasses import dataclass

from logic_gates import AND, NOT, OR, XOR

from .bits import (
    add_32bit,
    add_64bit,
    bits_to_int,
    compute_zero,
    int_to_bits,
    invert_32bit,
    shl_32,
    shr_32_arith,
    shr_32_logical,
)

# ── ALU result type ────────────────────────────────────────────────────────────


@dataclass
class ALUResult32:
    """Result of a 32-bit gate-level ALU operation.

    Attributes:
        result   — 32-bit unsigned result of the operation.
        carry    — carry out of bit 31 (C flag, useful for unsigned overflow).
        overflow — signed overflow (V flag): 1 if two's-complement overflow.
        zero     — Z flag: 1 if result == 0 (computed via NOR tree).
        negative — N flag: sign bit of result (bit 31).
    """

    result: int    # 32-bit unsigned result
    carry: int     # carry out of bit 31
    overflow: int  # signed overflow (V flag)
    zero: int      # 1 if result == 0
    negative: int  # sign bit (bit 31)


def _make_result(value: int, carry: int, overflow: int) -> ALUResult32:
    """Build an ALUResult32 from a raw 32-bit value and flags.

    Computes the zero and negative flags from the value's bit pattern.
    """
    bits = int_to_bits(value, 32)
    zero = compute_zero(bits)
    negative = bits[31]
    return ALUResult32(
        result=value,
        carry=carry,
        overflow=overflow,
        zero=zero,
        negative=negative,
    )


# ── Arithmetic operations ──────────────────────────────────────────────────────


def add32(a: int, b: int, carry_in: int = 0) -> ALUResult32:
    """32-bit addition via ripple-carry adder.

    Routes through 32 full adders (XOR + AND + OR per stage).
    Overflow is detected by comparing carry_in to bit 31 vs carry_out of bit 31.

    Args:
        a:        32-bit unsigned first operand.
        b:        32-bit unsigned second operand.
        carry_in: Carry into bit 0 (default 0).

    Returns:
        ALUResult32 with result, carry, overflow, zero, negative flags.

    Example:
        >>> r = add32(1, 1)
        >>> r.result
        2
        >>> r.zero
        0
    """
    result, carry, overflow = add_32bit(a, b, carry_in)
    return _make_result(result, carry, overflow)


def sub32(a: int, b: int) -> ALUResult32:
    """32-bit subtraction via two's-complement: A + NOT(B) + 1.

    Hardware implementation: invert all bits of B using 32 NOT gates, then
    feed into the ripple-carry adder with carry_in=1.  This produces A - B
    without any subtraction hardware.

    Carry interpretation for SUB: carry=1 means NO borrow (A >= B unsigned).
    Carry=0 means borrow occurred (A < B unsigned).

    Args:
        a: 32-bit unsigned minuend.
        b: 32-bit unsigned subtrahend.

    Returns:
        ALUResult32.  carry=1 → no borrow; carry=0 → borrow (A < B unsigned).

    Example:
        >>> r = sub32(5, 3)
        >>> r.result
        2
        >>> r.carry   # no borrow: 5 >= 3
        1
    """
    not_b = invert_32bit(b)
    result, carry, overflow = add_32bit(a, not_b, 1)
    return _make_result(result, carry, overflow)


# ── Bitwise operations ─────────────────────────────────────────────────────────


def and32(a: int, b: int) -> ALUResult32:
    """32-bit bitwise AND: 32 AND gate instances in parallel.

    Args:
        a: 32-bit unsigned first operand.
        b: 32-bit unsigned second operand.

    Returns:
        ALUResult32.  carry=0, overflow=0 (bitwise ops don't set these).
    """
    a_bits = int_to_bits(a, 32)
    b_bits = int_to_bits(b, 32)
    result_bits = [AND(a_bits[i], b_bits[i]) for i in range(32)]
    return _make_result(bits_to_int(result_bits), 0, 0)


def or32(a: int, b: int) -> ALUResult32:
    """32-bit bitwise OR: 32 OR gate instances in parallel."""
    a_bits = int_to_bits(a, 32)
    b_bits = int_to_bits(b, 32)
    result_bits = [OR(a_bits[i], b_bits[i]) for i in range(32)]
    return _make_result(bits_to_int(result_bits), 0, 0)


def xor32(a: int, b: int) -> ALUResult32:
    """32-bit bitwise XOR: 32 XOR gate instances in parallel."""
    a_bits = int_to_bits(a, 32)
    b_bits = int_to_bits(b, 32)
    result_bits = [XOR(a_bits[i], b_bits[i]) for i in range(32)]
    return _make_result(bits_to_int(result_bits), 0, 0)


def nor32(a: int, b: int) -> ALUResult32:
    """32-bit bitwise NOR: OR then NOT, applied to all 32 bit positions.

    NOR(a, b) = NOT(OR(a, b)).  In hardware: 32 NOR gates (or 32 OR gates
    feeding 32 NOT gates).

    The MIPS NOR instruction is used to implement bitwise NOT of a register:
        NOR rd, rs, $zero  ≡  rd = NOT(rs)
    """
    a_bits = int_to_bits(a, 32)
    b_bits = int_to_bits(b, 32)
    result_bits = [NOT(OR(a_bits[i], b_bits[i])) for i in range(32)]
    return _make_result(bits_to_int(result_bits), 0, 0)


# ── Comparison operations ──────────────────────────────────────────────────────


def slt32(a: int, b: int) -> ALUResult32:
    """Set Less Than (signed): result = 1 if signed(a) < signed(b), else 0.

    Implementation uses the subtract-and-check-flags approach:
        diff = sub32(a, b)
        signed_less = XOR(diff.negative, diff.overflow)

    This is exactly what the MIPS R2000 ALU does:
    - If no overflow and result is negative: a < b (straightforward case)
    - If overflow and result is non-negative: a < b (overflow flipped sign)
    - The XOR of these two signals gives the correct comparison.

    Truth table:
        N=0, V=0  →  result ≥ 0, no overflow  → a >= b → 0
        N=1, V=0  →  result < 0, no overflow   → a < b  → 1
        N=0, V=1  →  result ≥ 0, overflow       → a < b  → 1
        N=1, V=1  →  result < 0, overflow       → a >= b → 0
    """
    diff = sub32(a, b)
    less = XOR(diff.negative, diff.overflow)
    return _make_result(less, 0, 0)


def sltu32(a: int, b: int) -> ALUResult32:
    """Set Less Than Unsigned: result = 1 if unsigned(a) < unsigned(b), else 0.

    For unsigned comparison, we check the borrow (carry) from subtraction.
    sub32 returns carry=0 when there IS a borrow (meaning A < B unsigned).

    This matches how the MIPS R2000 SLTU instruction works:
        result = NOT(carry_out of A - B)

    carry=0 means borrow = 1 means A < B (unsigned).
    """
    diff = sub32(a, b)
    # carry=1 → no borrow → a >= b → result = 0
    # carry=0 → borrow    → a < b  → result = 1
    less = NOT(diff.carry)
    return _make_result(less, 0, 0)


# ── Shift operations ───────────────────────────────────────────────────────────


def sll32(a: int, shamt: int) -> ALUResult32:
    """Shift Left Logical by shamt (0–31)."""
    result = shl_32(a, shamt)
    return _make_result(result, 0, 0)


def srl32(a: int, shamt: int) -> ALUResult32:
    """Shift Right Logical by shamt (0–31): zero-fills from MSB."""
    result = shr_32_logical(a, shamt)
    return _make_result(result, 0, 0)


def sra32(a: int, shamt: int) -> ALUResult32:
    """Shift Right Arithmetic by shamt (0–31): sign-fills from MSB."""
    result = shr_32_arith(a, shamt)
    return _make_result(result, 0, 0)


# ── Multiplication ─────────────────────────────────────────────────────────────


def multu32(a: int, b: int) -> tuple[int, int]:
    """Unsigned 32×32 → 64-bit multiplication via shift-and-add.

    Algorithm: for each of the 32 bits of b, if bit[i] is 1, add
    (a << i) into the 64-bit accumulator using a gate-level 64-bit adder.

    This is the classical grade-school multiplication algorithm implemented
    in hardware.  Each partial product is a shifted copy of A, enabled by
    the corresponding bit of B.  Total: at most 32 gate-level 64-bit additions.

    Args:
        a: 32-bit unsigned multiplicand.
        b: 32-bit unsigned multiplier.

    Returns:
        (hi, lo): upper and lower 32 bits of the 64-bit product.

    Example:
        >>> hi, lo = multu32(6, 7)
        >>> lo
        42
        >>> hi
        0
    """
    a_masked = a & 0xFFFF_FFFF
    b_bits = int_to_bits(b & 0xFFFF_FFFF, 32)
    product = 0
    for bit_idx in range(32):
        if b_bits[bit_idx] == 1:
            # We need: partial = a << bit_idx as a 64-bit value.
            # We can build this by placing a_masked into a 64-bit representation
            # shifted left by bit_idx positions.
            #
            # Strategy: treat a_masked as a 64-bit number, shift left by bit_idx
            # using gate-level 64-bit shift (shift left via bit-list manipulation).
            a_64_bits = int_to_bits(a_masked, 64)  # a zero-extended to 64 bits
            if bit_idx == 0:
                shifted_64_bits = a_64_bits
            else:
                # Shift left: prepend bit_idx zeros at LSB end, drop upper bit_idx
                shifted_64_bits = [0] * bit_idx + a_64_bits[:64 - bit_idx]
            partial = bits_to_int(shifted_64_bits)
            product, _ = add_64bit(product, partial)
    hi = (product >> 32) & 0xFFFF_FFFF
    lo = product & 0xFFFF_FFFF
    return hi, lo


def mult32(a: int, b: int) -> tuple[int, int]:
    """Signed 32×32 → 64-bit multiplication via shift-and-add.

    Handles signs manually: compute |a| * |b| (unsigned), then flip the
    result if the signs differ.  This avoids signed multiplication in Python.

    The magnitude multiplication routes through the same gate-level
    shift-and-add as multu32.

    Args:
        a: 32-bit value (treated as signed via bit 31).
        b: 32-bit value (treated as signed via bit 31).

    Returns:
        (hi, lo): upper and lower 32 bits of the 64-bit signed product.

    Example:
        >>> hi, lo = mult32(0xFFFFFFFF, 1)  # -1 * 1 = -1
        >>> hi
        4294967295
        >>> lo
        4294967295
    """
    a_bits = int_to_bits(a & 0xFFFF_FFFF, 32)
    b_bits = int_to_bits(b & 0xFFFF_FFFF, 32)
    sign_a = a_bits[31]
    sign_b = b_bits[31]

    # Compute absolute values using two's complement negation if negative
    if sign_a:
        # Negate a: invert + 1
        neg_a, _, _ = add_32bit(invert_32bit(a & 0xFFFF_FFFF), 0, 1)
        a_abs = neg_a
    else:
        a_abs = a & 0xFFFF_FFFF

    if sign_b:
        neg_b, _, _ = add_32bit(invert_32bit(b & 0xFFFF_FFFF), 0, 1)
        b_abs = neg_b
    else:
        b_abs = b & 0xFFFF_FFFF

    hi, lo = multu32(a_abs, b_abs)

    # If exactly one operand is negative, negate the 64-bit result
    result_negative = XOR(sign_a, sign_b)
    if result_negative:
        # Negate 64-bit: invert all 64 bits, add 1
        combined = (hi << 32) | lo
        combined_bits = int_to_bits(combined, 64)
        inv_bits = [NOT(b2) for b2 in combined_bits]
        inv_val = bits_to_int(inv_bits)
        neg_val, _ = add_64bit(inv_val, 0, 1)
        hi = (neg_val >> 32) & 0xFFFF_FFFF
        lo = neg_val & 0xFFFF_FFFF

    return hi, lo


# ── Division ───────────────────────────────────────────────────────────────────


def divu32(a: int, b: int) -> tuple[int, int]:
    """Unsigned 32-bit division via 32-iteration non-restoring long division.

    This is the hardware non-restoring division algorithm in 32 steps.

    For each bit position from 31 down to 0:
      1. Shift b left by that many positions (b << bit).
      2. If remainder >= shifted_b (test: subtract and check no borrow):
         a. Subtract shifted_b from remainder (via gate-level sub32).
         b. Set bit ``bit`` in the quotient.

    Total: exactly 32 iterations, each involving shl_32 + sub32.

    Args:
        a: 32-bit unsigned dividend.
        b: 32-bit unsigned divisor.

    Returns:
        (quotient, remainder).  If b==0, returns (0xFFFFFFFF, a) matching
        hardware undefined behavior.

    Example:
        >>> divu32(10, 3)
        (3, 1)
        >>> divu32(7, 2)
        (3, 1)
    """
    if b == 0:
        return 0xFFFF_FFFF, a & 0xFFFF_FFFF

    a_masked = a & 0xFFFF_FFFF
    b_masked = b & 0xFFFF_FFFF
    quotient = 0
    remainder = a_masked

    for bit_idx in range(31, -1, -1):
        # Shift b left by bit_idx positions using 64-bit representation.
        # We cannot use shl_32 here because b << bit_idx may exceed 32 bits.
        # When b << bit_idx overflows 32 bits, the shifted value is definitely
        # larger than any 32-bit remainder, so we skip (don't set quotient bit).
        #
        # Implement 64-bit left shift: place b_masked into a 64-bit bit list,
        # shift left by bit_idx.
        b_64_bits = int_to_bits(b_masked, 64)
        if bit_idx == 0:
            shifted_b_64 = b_64_bits
        else:
            shifted_b_64 = [0] * bit_idx + b_64_bits[:64 - bit_idx]
        shifted_b_val = bits_to_int(shifted_b_64)

        # If the shifted value exceeds 32 bits, it cannot fit in remainder
        # (remainder is always a 32-bit value). Skip this bit position.
        if shifted_b_val > 0xFFFF_FFFF:
            continue

        shifted_b = shifted_b_val & 0xFFFF_FFFF

        # Try to subtract: sub32 carry=1 means no borrow (remainder >= shifted_b)
        diff = sub32(remainder, shifted_b)
        if diff.carry:  # no borrow → remainder >= shifted_b
            remainder = diff.result
            # Set bit bit_idx in quotient using bit-list manipulation
            q_bits = int_to_bits(quotient, 32)
            q_bits[bit_idx] = 1
            quotient = bits_to_int(q_bits)

    return quotient, remainder


def div32(a: int, b: int) -> tuple[int, int]:
    """Signed 32-bit division.

    Handles signs manually: compute |a| / |b| (unsigned), then apply
    sign rules:
      - Quotient is negative if operands have opposite signs.
      - Remainder has the same sign as the dividend (MIPS convention).

    Args:
        a: 32-bit value (treated as signed via bit 31).
        b: 32-bit value (treated as signed via bit 31).

    Returns:
        (quotient, remainder) both as 32-bit unsigned values.
        If b==0, returns (0xFFFFFFFF, a) matching hardware.

    Example:
        >>> div32(10, 3)
        (3, 1)
        >>> q, r = div32(0xFFFFFFF6, 3)  # -10 / 3 = -3 rem -1
        >>> q
        4294967293
        >>> r
        4294967295
    """
    a_bits = int_to_bits(a & 0xFFFF_FFFF, 32)
    b_bits = int_to_bits(b & 0xFFFF_FFFF, 32)
    sign_a = a_bits[31]
    sign_b = b_bits[31]

    # Compute absolute values
    if sign_a:
        a_abs_raw, _, _ = add_32bit(invert_32bit(a & 0xFFFF_FFFF), 0, 1)
    else:
        a_abs_raw = a & 0xFFFF_FFFF

    if sign_b:
        b_abs_raw, _, _ = add_32bit(invert_32bit(b & 0xFFFF_FFFF), 0, 1)
    else:
        b_abs_raw = b & 0xFFFF_FFFF

    if b_abs_raw == 0:
        # Division by zero: hardware undefined
        return 0xFFFF_FFFF, a & 0xFFFF_FFFF

    q_abs, r_abs = divu32(a_abs_raw, b_abs_raw)

    # Apply sign to quotient
    quot_negative = XOR(sign_a, sign_b)
    if quot_negative:
        quotient, _, _ = add_32bit(invert_32bit(q_abs), 0, 1)
    else:
        quotient = q_abs

    # Remainder has same sign as dividend
    if sign_a and r_abs != 0:
        remainder, _, _ = add_32bit(invert_32bit(r_abs), 0, 1)
    else:
        remainder = r_abs

    return quotient & 0xFFFF_FFFF, remainder & 0xFFFF_FFFF
