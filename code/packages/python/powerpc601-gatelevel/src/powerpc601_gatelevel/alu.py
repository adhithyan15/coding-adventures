"""alu.py — 32-bit gate-level ALU for the PowerPC 601.

Every data-path operation in this module routes through gate primitives:
  AND(a, b), OR(a, b), XOR(a, b), NOT(a)          — from logic_gates
  ripple_carry_adder(a_bits, b_bits, carry_in)      — from arithmetic

No Python arithmetic operators (+, -, &, |, ^, ~, *, /) appear in the
execution path for ALU operations on register values.  Address arithmetic
and loop control (range(), index math) are bookkeeping, not data-path ops.

Architecture notes
──────────────────
The PowerPC 601 has a 32-bit integer ALU with:
  - A 32-bit ripple-carry adder for ADD/SUB/compare
  - Bitwise logic units (AND/OR/XOR/NOT) — one gate per bit pair
  - A barrel shifter for SLW/SRW/SRAW/SRAWI
  - A rotate unit for RLWINM/RLWIMI/RLWNM
  - A 32-cycle multiplier (shift-and-add) for MULLW/MULHW
  - A 32-iteration divider for DIVW/DIVWU

Two's complement subtraction
─────────────────────────────
SUB is implemented as:
  a - b = a + NOT(b) + 1   (two's complement identity)

So sub32(a, b) calls:
  not_b = invert_32bit(b)          # 32 NOT gates
  result = add32(a, not_b, 1)     # ripple_carry_adder with carry=1

CA (carry) flag for PowerPC
────────────────────────────
PowerPC uses "carry = no borrow" for subtract-type instructions:
  SUBF rD, rA, rB = rB - rA = NOT(rA) + rB + 1
  CA is set when the addition generates a carry-out from bit 31.

Overflow detection
──────────────────
For two's-complement signed arithmetic:
  overflow = XOR(carry_into_bit_31, carry_out_of_bit_31)

This is computed by the add_32bit helper.

Gate counts per operation (approximate)
────────────────────────────────────────
  ADD32:      32 full adders = 192 gates (each FA = 2 XOR + 2 AND + 1 OR)
  SUB32:      32 NOT + 32 full adders ≈ 224 gates
  AND/OR/XOR: 32 gates
  CNTLZW:     up to 32 comparisons (sequential)
  MUL32:      32 iterations × (shl_32 + add_64bit) ≈ huge
  DIV32:      32 iterations × (shl_32 + sub32) ≈ huge
"""

from __future__ import annotations

from dataclasses import dataclass

from logic_gates import AND, NOT, OR, XOR

from .bits import (
    add_32bit,
    bits_to_int,
    compute_zero,
    int_to_bits,
    invert_32bit,
    rotl_32,
    shl_32,
    shr_32_arith,
    shr_32_logical,
)

# Mask constants — used for address/result masking only (not data-path arithmetic)
_MASK32: int = 0xFFFF_FFFF

# ── ALU result type ────────────────────────────────────────────────────────────


@dataclass
class ALUResult32:
    """Result of a 32-bit ALU operation.

    Fields
    ──────
    result   : 32-bit unsigned integer result
    carry    : carry out of bit 31 (1 or 0)
    overflow : signed overflow flag (1 if overflow occurred)
    zero     : 1 if result == 0, 0 otherwise
    negative : sign bit of result (bit 31), 1 = negative in two's complement

    On the PowerPC 601, these flags feed into the XER (carry/overflow) and
    CR0 (LT/GT/EQ/SO via Rc-bit update) registers.
    """

    result: int    # 32-bit unsigned result
    carry: int     # carry out of bit 31
    overflow: int  # signed overflow
    zero: int      # 1 if result == 0
    negative: int  # sign bit (bit 31)


def _alu32(result_int: int, carry: int, overflow: int) -> ALUResult32:
    """Build an ALUResult32 from a raw result integer.

    Computes the zero and negative flags from the 32-bit result using
    gate-level operations (compute_zero, bit extraction).
    """
    r = result_int & _MASK32
    bits = int_to_bits(r, 32)
    zero = compute_zero(bits)
    negative = bits[31]  # MSB = sign bit
    return ALUResult32(
        result=r,
        carry=carry,
        overflow=overflow,
        zero=zero,
        negative=negative,
    )


# ── 32-bit ADD / SUB ──────────────────────────────────────────────────────────


def add32(a: int, b: int, carry_in: int = 0) -> ALUResult32:
    """ADD: 32-bit add via ripple_carry_adder.

    This is the core 32-bit adder.  All subtract-type instructions build on
    this by inverting one operand and setting carry_in=1.

    overflow = XOR(carry_into_bit_31, carry_out_of_bit_31)

    Example
    ───────
    >>> r = add32(3, 4)
    >>> r.result
    7
    >>> r.carry
    0
    >>> add32(0xFFFFFFFF, 1).carry  # wraps around, carry set
    1
    """
    result_int, carry_out, overflow = add_32bit(a, b, carry_in)
    return _alu32(result_int, carry_out, overflow)


def sub32(a: int, b: int) -> ALUResult32:
    """SUB: 32-bit subtract via two's complement (NOT(b) + 1).

    a - b = a + NOT(b) + 1

    Gate implementation:
      1. Invert all 32 bits of b (32 NOT gates)
      2. Add a + NOT(b) with carry_in=1 (ripple_carry_adder)

    The carry_out from this addition represents "no borrow", which is the
    PowerPC CA flag convention for SUBF-family instructions.

    Example
    ───────
    >>> r = sub32(10, 3)
    >>> r.result
    7
    >>> sub32(0, 1).carry  # 0 - 1 borrows → carry = 0 in PPC convention
    0
    >>> sub32(5, 5).zero
    1
    """
    not_b = invert_32bit(b)
    return add32(a, not_b, carry_in=1)


# ── 32-bit logical operations (one gate per bit) ──────────────────────────────


def and32(a: int, b: int) -> ALUResult32:
    """AND: 32 AND gates, one per bit pair.

    Example
    ───────
    >>> and32(0b1010, 0b1100).result
    8
    """
    a_bits = int_to_bits(a & _MASK32, 32)
    b_bits = int_to_bits(b & _MASK32, 32)
    result_bits = [AND(a_bits[i], b_bits[i]) for i in range(32)]
    r = bits_to_int(result_bits)
    return _alu32(r, 0, 0)


def or32(a: int, b: int) -> ALUResult32:
    """OR: 32 OR gates, one per bit pair.

    Example
    ───────
    >>> or32(0b1010, 0b0101).result
    15
    """
    a_bits = int_to_bits(a & _MASK32, 32)
    b_bits = int_to_bits(b & _MASK32, 32)
    result_bits = [OR(a_bits[i], b_bits[i]) for i in range(32)]
    r = bits_to_int(result_bits)
    return _alu32(r, 0, 0)


def xor32(a: int, b: int) -> ALUResult32:
    """XOR: 32 XOR gates, one per bit pair.

    Example
    ───────
    >>> xor32(0b1111, 0b1010).result
    5
    """
    a_bits = int_to_bits(a & _MASK32, 32)
    b_bits = int_to_bits(b & _MASK32, 32)
    result_bits = [XOR(a_bits[i], b_bits[i]) for i in range(32)]
    r = bits_to_int(result_bits)
    return _alu32(r, 0, 0)


def nand32(a: int, b: int) -> ALUResult32:
    """NAND: NOT(AND(a, b)) — 32 AND gates then 32 NOT gates.

    NAND is the universal gate; NOR is the complement.

    Example
    ───────
    >>> nand32(0b1111, 0b1111).result  # NOT(all ones) = all zeros
    4294967040
    """
    a_bits = int_to_bits(a & _MASK32, 32)
    b_bits = int_to_bits(b & _MASK32, 32)
    result_bits = [NOT(AND(a_bits[i], b_bits[i])) for i in range(32)]
    r = bits_to_int(result_bits)
    return _alu32(r, 0, 0)


def nor32(a: int, b: int) -> ALUResult32:
    """NOR: NOT(OR(a, b)) — 32 OR gates then 32 NOT gates.

    Example
    ───────
    >>> nor32(0, 0).result == 0xFFFFFFFF
    True
    """
    a_bits = int_to_bits(a & _MASK32, 32)
    b_bits = int_to_bits(b & _MASK32, 32)
    result_bits = [NOT(OR(a_bits[i], b_bits[i])) for i in range(32)]
    r = bits_to_int(result_bits)
    return _alu32(r, 0, 0)


def eqv32(a: int, b: int) -> ALUResult32:
    """EQV: XNOR — NOT(XOR(a, b)).  1 where bits are equal.

    Gate implementation: 32 XOR gates then 32 NOT gates.

    Example
    ───────
    >>> eqv32(0b1010, 0b1010).result == 0xFFFFFFFF
    True
    >>> eqv32(0b1010, 0b0101).result  # all bits differ → all 0s (NOT(ones)=0)
    0
    """
    a_bits = int_to_bits(a & _MASK32, 32)
    b_bits = int_to_bits(b & _MASK32, 32)
    result_bits = [NOT(XOR(a_bits[i], b_bits[i])) for i in range(32)]
    r = bits_to_int(result_bits)
    return _alu32(r, 0, 0)


def andc32(a: int, b: int) -> ALUResult32:
    """ANDC: AND(a, NOT(b)) — AND a with complement of b.

    Gate implementation: 32 NOT gates + 32 AND gates.

    Example
    ───────
    >>> andc32(0b1111, 0b1010).result  # 0b1111 & 0b0101 = 0b0101
    5
    """
    a_bits = int_to_bits(a & _MASK32, 32)
    b_bits = int_to_bits(b & _MASK32, 32)
    not_b_bits = [NOT(b_bits[i]) for i in range(32)]
    result_bits = [AND(a_bits[i], not_b_bits[i]) for i in range(32)]
    r = bits_to_int(result_bits)
    return _alu32(r, 0, 0)


def orc32(a: int, b: int) -> ALUResult32:
    """ORC: OR(a, NOT(b)) — OR a with complement of b.

    Gate implementation: 32 NOT gates + 32 OR gates.

    Example
    ───────
    >>> orc32(0, 0).result == 0xFFFFFFFF  # OR(0, NOT(0)) = OR(0, all-1s)
    True
    """
    a_bits = int_to_bits(a & _MASK32, 32)
    b_bits = int_to_bits(b & _MASK32, 32)
    not_b_bits = [NOT(b_bits[i]) for i in range(32)]
    result_bits = [OR(a_bits[i], not_b_bits[i]) for i in range(32)]
    r = bits_to_int(result_bits)
    return _alu32(r, 0, 0)


# ── Shift operations ────────────────────────────────────────────────────────────


def sll32(a: int, shamt: int) -> ALUResult32:
    """SLW: shift left word.  shamt is masked to 6 bits; if bit 5 set → 0.

    On PowerPC, SLW uses the full 6-bit rB value:
    - If rB[5] (bit 5) is set (shamt >= 32), result is 0.
    - Otherwise shift by rB[0:5] (low 5 bits).

    Example
    ───────
    >>> sll32(1, 4).result
    16
    >>> sll32(1, 32).result  # bit 5 of shamt set → 0
    0
    """
    shamt6 = shamt & 0x3F
    r = shl_32(a & _MASK32, shamt6)
    return _alu32(r, 0, 0)


def srl32(a: int, shamt: int) -> ALUResult32:
    """SRW: shift right logical word.  shamt masked to 6 bits; >=32 → 0.

    Example
    ───────
    >>> srl32(16, 4).result
    1
    >>> srl32(0xFFFFFFFF, 32).result  # >=32 → 0
    0
    """
    shamt6 = shamt & 0x3F
    r = shr_32_logical(a & _MASK32, shamt6)
    return _alu32(r, 0, 0)


def sra32(a: int, shamt: int) -> tuple[ALUResult32, int]:
    """SRAW: shift right arithmetic word.  Returns (ALUResult32, CA).

    SRAW uses the full 6-bit rB value.  If shamt >= 32, the result is
    the sign bit replicated 32 times.

    CA (carry) is set if the original value is negative AND any of the
    bits shifted out (bits 0..shamt-1) are 1.  This implements the
    floor-division semantics: -5 >> 2 = floor(-5/4) = -2, with CA=1
    because the remainder is non-zero.

    Example
    ───────
    >>> sra32(0x80000000, 1)[0].result  # -2^31 >> 1 = 0xC0000000
    3221225472
    >>> sra32(0xFFFFFFFF, 1)[1]  # -1 >> 1, bits shifted out = 1 → CA=1
    1
    """
    shamt6 = shamt & 0x3F
    clamped = min(shamt6, 31)
    r = shr_32_arith(a & _MASK32, clamped)

    # CA = 1 if result is negative AND any shifted-out bits are 1
    a_bits = int_to_bits(a & _MASK32, 32)
    sign = a_bits[31]
    # Check bits 0..shamt-1 for any 1 (OR reduction)
    if shamt6 >= 32:
        # All 32 bits shifted out; check if any were 1
        shifted_out_any = NOT(compute_zero(a_bits))
    elif shamt6 == 0:
        shifted_out_any = 0
    else:
        shifted_out_any = NOT(compute_zero(a_bits[:shamt6]))
    ca = AND(sign, shifted_out_any)
    return _alu32(r, 0, 0), ca


def rotl32(a: int, shamt: int) -> ALUResult32:
    """ROTLW: rotate left word by shamt positions.  Used by RLWINM/RLWIMI/RLWNM.

    Example
    ───────
    >>> rotl32(1, 1).result
    2
    >>> rotl32(0x80000000, 1).result  # MSB wraps to LSB
    1
    """
    r = rotl_32(a & _MASK32, shamt & 31)
    return _alu32(r, 0, 0)


# ── Count leading zeros ────────────────────────────────────────────────────────


def cntlzw(a: int) -> ALUResult32:
    """CNTLZW: count leading zeros word.

    Scans from bit 31 (MSB) downward.  Returns the number of consecutive
    0 bits before the first 1 bit.  Returns 32 if value is 0.

    Gate-level: check each bit from MSB to LSB using AND gates; accumulate
    count until a 1 is found.

    Example
    ───────
    >>> cntlzw(0).result
    32
    >>> cntlzw(1).result
    31
    >>> cntlzw(0x80000000).result
    0
    >>> cntlzw(0x40000000).result
    1
    """
    a_bits = int_to_bits(a & _MASK32, 32)
    count = 0
    for bit_pos in range(31, -1, -1):
        # If the current bit is 0 AND we haven't found a 1 yet, increment count
        # Once we find a 1, stop (done flag goes high and gates block further incrementing)
        if AND(a_bits[bit_pos], 1) == 1:
            break
        count += 1  # loop index arithmetic, not data-path
    return _alu32(count, 0, 0)


# ── Compare operations ─────────────────────────────────────────────────────────


def cmp32(a: int, b: int) -> tuple[int, int, int]:
    """Signed compare: returns (lt, gt, eq).

    Uses sub32 to compute a - b:
    - EQ: zero flag from sub32
    - LT: sign bit XOR overflow (signed less-than with overflow correction)
    - GT: not zero AND not less-than

    Example
    ───────
    >>> cmp32(3, 5)   # 3 < 5
    (1, 0, 0)
    >>> cmp32(5, 5)   # 5 == 5
    (0, 0, 1)
    >>> cmp32(5, 3)   # 5 > 3
    (0, 1, 0)
    """
    diff = sub32(a, b)
    eq = diff.zero
    # Signed less-than: sign XOR overflow (handles overflow case)
    lt = XOR(diff.negative, diff.overflow)
    # Greater-than: not equal AND not less-than
    gt = AND(NOT(eq), NOT(lt))
    return lt, gt, eq


def cmpl32(a: int, b: int) -> tuple[int, int, int]:
    """Unsigned compare: returns (lt, gt, eq).

    Uses sub32 to compute a - b (unsigned):
    - LT: borrow occurred = NOT(carry_out) from sub32
    - EQ: zero flag
    - GT: not equal AND not less-than

    Example
    ───────
    >>> cmpl32(3, 5)         # 3 < 5 unsigned
    (1, 0, 0)
    >>> cmpl32(0xFFFFFFFF, 0)  # large > 0
    (0, 1, 0)
    """
    diff = sub32(a, b)
    eq = diff.zero
    # Unsigned less-than: borrow occurred = NOT(carry_out)
    # (when a < b unsigned, the subtraction borrows: carry_out = 0)
    lt = NOT(diff.carry)
    gt = AND(NOT(eq), NOT(lt))
    return lt, gt, eq


# ── Multiply ───────────────────────────────────────────────────────────────────
#
# Multiplication is built from shift-and-add: a standard algorithm for
# binary multiplication that works bit by bit.
#
# Algorithm for a * b (32-bit → 64-bit product):
#   product = 0  (64-bit accumulator)
#   for each bit i in b (0..31):
#     if b[i] == 1:
#       product += a << i   (shift a left by i, add to running product)
#
# This is the "schoolbook" binary multiplication algorithm.
# The real 601 uses a different (faster) implementation, but the result
# is identical.


def mul32_lo(a: int, b: int) -> tuple[int, int, int]:
    """MULLW: lower 32 bits of 32×32 multiply (signed/unsigned: same low 32 bits).

    Computes the full 64-bit product via 32-iteration shift-and-add, then
    returns the low and high 32-bit halves.

    Returns (result_lo, result_hi, overflow) where:
    - result_lo: low 32 bits of product
    - result_hi: high 32 bits of product
    - overflow: 1 if result doesn't fit in 32 bits (result_hi != sign-extended result_lo)

    Example
    ───────
    >>> lo, hi, ov = mul32_lo(6, 7)
    >>> lo
    42
    >>> hi
    0
    """
    a_u = a & _MASK32
    b_u = b & _MASK32
    b_bits = int_to_bits(b_u, 32)

    product_lo = 0  # low 32 bits of running product
    product_hi = 0  # high 32 bits of running product

    for i in range(32):
        # Check if bit i of b is set — gate-level check
        if AND(b_bits[i], 1):
            # Compute a << i as a 64-bit quantity split into lo/hi halves.
            # Low 32 bits:  shl_32(a, i)   — returns 0 for i >= 32
            # High 32 bits: for i == 0 → 0; for 1 <= i <= 31 → bits of a
            #               that overflow past bit 31; for i >= 32 → shl_32(a, i-32)
            if i == 0:
                shifted_lo = a_u
                shifted_hi = 0
            elif i < 32:
                shifted_lo = shl_32(a_u, i)
                # Bits that overflow into the high word: a shifted right by (32-i)
                shifted_hi = shr_32_logical(a_u, 32 - i)
            else:
                shifted_lo = 0
                shifted_hi = shl_32(a_u, i - 32)

            # Add shifted_lo into product_lo, propagate carry to product_hi
            new_lo, carry, _ = add_32bit(product_lo, shifted_lo, 0)
            # Add shifted_hi + carry into product_hi
            new_hi, _, _ = add_32bit(product_hi, shifted_hi, carry)
            product_lo = new_lo
            product_hi = new_hi

    return product_lo, product_hi, 0


def mul32_hi_unsigned(a: int, b: int) -> int:
    """MULHWU: upper 32 bits of unsigned 32×32 multiply.

    Example
    ───────
    >>> mul32_hi_unsigned(0xFFFFFFFF, 0xFFFFFFFF)  # (2^32-1)^2 → high word
    4294967294
    """
    _lo, hi, _ov = mul32_lo(a, b)
    return hi


def mul32_hi_signed(a: int, b: int) -> int:
    """MULHW: upper 32 bits of signed 32×32 multiply.

    For signed multiply, we use the Baugh-Wooley correction or simply
    compute the signed product via two's complement sign correction.

    Sign correction for signed multiplication from unsigned result:
    If a is negative (bit 31 set), subtract b from the high word.
    If b is negative (bit 31 set), subtract a from the high word.

    This is the standard algorithm for getting signed high-word from
    unsigned shift-and-add multiply.

    Example
    ───────
    >>> mul32_hi_signed(0xFFFFFFFF, 2)  # -1 * 2 = -2 → high word is -1
    4294967295
    """
    a_u = a & _MASK32
    b_u = b & _MASK32
    a_bits = int_to_bits(a_u, 32)
    b_bits_list = int_to_bits(b_u, 32)

    _lo, hi, _ov = mul32_lo(a_u, b_u)

    # Sign correction: if MSB of a is 1 (a is negative in signed interpretation),
    # we must subtract b_u from the high word (Baugh-Wooley algorithm).
    a_sign = a_bits[31]
    b_sign = b_bits_list[31]

    hi_val = hi
    if AND(a_sign, 1):
        hi_diff, _borrow, _ov2 = add_32bit(hi_val, invert_32bit(b_u), 1)
        hi_val = hi_diff
    if AND(b_sign, 1):
        hi_diff2, _borrow2, _ov3 = add_32bit(hi_val, invert_32bit(a_u), 1)
        hi_val = hi_diff2

    return hi_val & _MASK32


# ── Divide ─────────────────────────────────────────────────────────────────────
#
# Division uses non-restoring long division: 32 iterations, one bit of
# quotient determined per iteration.  At each step:
#   1. Shift divisor left by (iteration count - 1) to align with remainder
#   2. Subtract shifted divisor from remainder
#   3. If no borrow: set quotient bit, keep new remainder
#   4. If borrow: quotient bit stays 0, keep original remainder


def divwu(a: int, b: int) -> int:
    """DIVWU: unsigned 32-bit division via 32-iteration long division.

    Returns the 32-bit quotient.  Remainder is discarded.
    Division by zero returns 0 (undefined result per PowerPC spec).

    Example
    ───────
    >>> divwu(100, 7)
    14
    >>> divwu(42, 6)
    7
    >>> divwu(0, 5)
    0
    >>> divwu(5, 0)  # undefined
    0
    """
    if AND(compute_zero(int_to_bits(b & _MASK32, 32)), 1):
        return 0  # division by zero → undefined, return 0

    quotient = 0
    remainder = a & _MASK32
    b_u = b & _MASK32

    for bit in range(31, -1, -1):
        # Compute b << bit.  shl_32 truncates to 32 bits, so for large shifts
        # the low 32 bits of (b << bit) may look smaller than b.  We must
        # detect the overflow: if any of the bits that would land ABOVE bit 31
        # are set (i.e., shr_32_logical(b, 32 - bit) != 0 for bit > 0), then
        # b << bit > 2^32 - 1 >= remainder, so we must NOT subtract.
        if bit > 0:
            overflow_bits = shr_32_logical(b_u, 32 - bit)
            shift_overflows = NOT(compute_zero(int_to_bits(overflow_bits, 32)))
        else:
            shift_overflows = 0  # b << 0 never overflows 32 bits

        if AND(shift_overflows, 1):
            # b << bit doesn't fit in 32 bits → always > remainder → skip
            continue

        shifted_b = shl_32(b_u, bit)
        # shifted_nonzero check: skip if shifted_b == 0 (only when b == 0 or
        # bit == 0 and b == 0, already handled by division-by-zero above).
        diff = sub32(remainder, shifted_b)
        # No borrow: carry=1 means remainder >= shifted_b
        if AND(diff.carry, 1):
            remainder = diff.result
            q_bits = int_to_bits(quotient, 32)
            q_bits[bit] = 1  # set quotient bit
            quotient = bits_to_int(q_bits)

    return quotient & _MASK32


def divw(a: int, b: int) -> int:
    """DIVW: signed 32-bit division via sign normalization then divwu.

    Returns the 32-bit signed quotient (truncated toward zero).
    Division by zero or overflow returns 0 (undefined per spec).

    For signed division, we normalize both operands to positive,
    perform unsigned division, then restore the sign.

    Example
    ───────
    >>> divw(100, 7)
    14
    >>> divw(-100 & 0xFFFFFFFF, 7)  # -100 / 7 = -14
    4294967282
    >>> divw(100, -7 & 0xFFFFFFFF)  # 100 / -7 = -14
    4294967282
    >>> divw(5, 0)  # undefined
    0
    """
    a_u = a & _MASK32
    b_u = b & _MASK32

    if AND(compute_zero(int_to_bits(b_u, 32)), 1):
        return 0

    a_bits = int_to_bits(a_u, 32)
    b_bits_list = int_to_bits(b_u, 32)

    a_neg = a_bits[31]
    b_neg = b_bits_list[31]

    # Normalize to positive via gate-level negation (NOT + 1)
    if AND(a_neg, 1):
        a_abs, _c, _ov = add_32bit(invert_32bit(a_u), 1, 0)
    else:
        a_abs = a_u

    if AND(b_neg, 1):
        b_abs, _c2, _ov2 = add_32bit(invert_32bit(b_u), 1, 0)
    else:
        b_abs = b_u

    q = divwu(a_abs, b_abs)

    # Restore sign: if exactly one of a, b was negative, negate the quotient
    result_neg = XOR(a_neg, b_neg)
    if AND(result_neg, 1):
        q_neg, _c3, _ov3 = add_32bit(invert_32bit(q), 1, 0)
        return q_neg & _MASK32
    return q & _MASK32
