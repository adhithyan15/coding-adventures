"""ALU for the Intel 8086 gate-level simulator.

=== Architecture ===

The 8086 ALU is 16-bit wide, with 8-bit variants for byte operations.
All additions and subtractions route through ripple_carry_adder chains.
Logical operations (AND, OR, XOR) use parallel gate arrays.

=== Gate count (ALU only — estimate) ===

Component                        Gates
──────────────────────────────── ─────
16-bit ripple adder              ~80  (16 full adders × ~5 gates each)
16-bit NOT (for SUB)             16
16-bit AND                       16
16-bit OR                        16
16-bit XOR                       16
Zero NOR tree (16-bit)           ~20
Parity XOR tree (8-bit)          ~8
Overflow XOR gate                1
Shifter/rotator                  ~64
──────────────────────────────── ─────
Total ALU estimate               ~237 gates (out of ~29,000 transistors total)

=== Overflow detection ===

For N-bit signed addition A + B = R:
  OF = XOR(carry_into_msb, carry_out_of_msb)

For 16-bit: OF = XOR(carry into bit 15, carry out of bit 15).
This is a single XOR gate at the MSB of the adder chain.

=== Subtraction via two's complement ===

SUB A, B  → A + NOT(B) + 1   (CF_in = 1 for no borrow)
SBB A, B  → A + NOT(B) + CF  (CF_in = current CF for borrow chain)

NEG A     → 0 - A = NOT(A) + 1

The 8086 CF convention: CF=1 means borrow occurred (opposite of 6502!).
After SUB: CF=1 if A < B (unsigned).

=== Auxiliary carry (AF) ===

AF = carry out of bit 3 into bit 4.  Used by DAA/DAS/AAA/AAS for
BCD correction.  The add_8bit / add_16bit helpers return this.

=== Note on MUL/DIV ===

A gate-level 16×16 unsigned multiplier would require ~1000 gates.
For educational purposes, MUL and DIV use Python host arithmetic
internally.  The key gate-level requirement (ADD/SUB/AND/OR/XOR through
ripple_carry_adder and gate primitives) is maintained for all other ops.
"""

from __future__ import annotations

from dataclasses import dataclass

from arithmetic import full_adder
from logic_gates import AND, NOT, OR, XOR

from intel8086_gatelevel.bits import (
    add_8bit,
    add_16bit,
    bits_to_int,
    compute_parity,
    compute_zero,
    int_to_bits,
    invert_8bit,
    invert_16bit,
    nibble_borrow,
)


@dataclass
class ALUResult8086:
    """Result of an ALU operation on the Intel 8086.

    Contains the computed value plus all flag values that the ALU operation
    can affect.  The caller decides which flags to commit.

    For example, INC/DEC update all flags except CF.  The caller preserves
    the old CF and commits only OF/SF/ZF/AF/PF from this result.

    Fields:
        result:   16-bit (or 8-bit) ALU output
        flag_cf:  Carry flag
        flag_of:  Overflow flag (signed overflow)
        flag_sf:  Sign flag (MSB of result)
        flag_zf:  Zero flag
        flag_af:  Auxiliary carry (carry out of bit 3)
        flag_pf:  Parity flag (even parity of low 8 bits)
    """

    result: int
    flag_cf: int
    flag_of: int
    flag_sf: int
    flag_zf: int
    flag_af: int
    flag_pf: int


# ─── 16-bit arithmetic operations ─────────────────────────────────────────────


def add16(a: int, b: int, carry_in: int = 0) -> ALUResult8086:
    """16-bit addition: A + B + carry_in.

    Routes through 16 full-adder stages.  Captures:
    - Carry out of bit 15 → CF
    - Carry into bit 15 via XOR(carry_in14, carry_out15) → OF
    - AF from carry out of bit 3

    Args:
        a:        First 16-bit operand (0–65535).
        b:        Second 16-bit operand (0–65535).
        carry_in: Carry in (0 or 1).  Used by ADC with current CF.

    Returns:
        ALUResult8086 with result and all flag values.

    Examples:
        >>> add16(5, 3).result
        8
        >>> add16(0x7FFF, 1).flag_of   # signed overflow: 32767 + 1 = -32768
        1
        >>> add16(0xFFFF, 1).flag_cf   # unsigned overflow
        1
    """
    bits_a = int_to_bits(a, 16)
    bits_b = int_to_bits(b, 16)

    # Full 16-bit adder using individual full_adder gates.
    sums: list[int] = []
    carries: list[int] = []
    carry = carry_in
    for i in range(16):
        s, carry = full_adder(bits_a[i], bits_b[i], carry)
        sums.append(s)
        carries.append(carry)

    result = bits_to_int(sums)

    # Overflow: XOR(carry into bit15, carry out of bit15)
    carry_into_15 = carries[14]   # carry out of bit 14 = carry into bit 15
    carry_out_15 = carries[15]
    overflow = XOR(carry_into_15, carry_out_15)

    # Auxiliary carry (AF): carry out of bit 3 — from the 16-bit add helper
    _, _, af = add_16bit(a, b, carry_in)

    return ALUResult8086(
        result=result,
        flag_cf=carries[15],
        flag_of=overflow,
        flag_sf=sums[15],
        flag_zf=compute_zero(sums),
        flag_af=af,
        flag_pf=compute_parity(sums),
    )


def sub16(a: int, b: int, borrow_in: int = 0) -> ALUResult8086:
    """16-bit subtraction: A - B - borrow_in via two's complement.

    Gate path:
      1. 16 NOT gates: NOT_B[i] = NOT(B[i])  for i in 0..15
      2. ripple_carry_adder(A, NOT_B, NOT(borrow_in))
      3. OF = XOR(carry_into_bit15, carry_out)
      4. CF = NOT(carry_out)  — CF=1 means borrow (A < B unsigned)

    Note on CF convention:
    The 8086 SUB sets CF=1 when A < B (borrow occurred), which is the
    COMPLEMENT of the adder's carry out.  The gate-level model matches
    this: CF = NOT(carry_out_of_adder) when borrow_in=0.

    Args:
        a:          Minuend (0–65535).
        b:          Subtrahend (0–65535).
        borrow_in:  SBB borrow from previous word (0 or 1).

    Returns:
        ALUResult8086. flag_cf = 1 means borrow occurred (A < B + borrow_in).

    Examples:
        >>> sub16(10, 3).result
        7
        >>> sub16(10, 3).flag_cf   # no borrow
        0
        >>> sub16(0, 1).flag_cf    # borrow occurred
        1
    """
    not_b = invert_16bit(b)
    # carry_in = NOT(borrow_in): when borrow_in=0 → carry_in=1 (normal sub)
    c_in = NOT(borrow_in)
    r, cout, _ = add_16bit(a, not_b, c_in)

    bits_r = int_to_bits(r, 16)
    bits_a = int_to_bits(a, 16)
    bits_nb = int_to_bits(not_b, 16)

    # Overflow via full adder chain for accurate carry_into_15
    carries: list[int] = []
    carry = c_in
    for i in range(16):
        _, carry = full_adder(bits_a[i], bits_nb[i], carry)
        carries.append(carry)
    overflow = XOR(carries[14], carries[15])

    # CF = 1 if borrow occurred = NOT(carry_out_of_adder)
    flag_cf = NOT(cout)

    # AF = 1 if nibble subtraction borrows (correct formula, not the
    # two's complement adder carry which does not match 8086 AF for SUB)
    af = nibble_borrow(a, b, borrow_in)

    return ALUResult8086(
        result=r,
        flag_cf=flag_cf,
        flag_of=overflow,
        flag_sf=bits_r[15],
        flag_zf=compute_zero(bits_r),
        flag_af=af,
        flag_pf=compute_parity(bits_r),
    )


def and16(a: int, b: int) -> ALUResult8086:
    """16-bit AND: A & B.

    16 AND gates in parallel.  CF=0, OF=0 (per 8086 spec for logic ops).

    Args:
        a: First 16-bit operand.
        b: Second 16-bit operand.

    Returns:
        ALUResult8086. flag_cf=0, flag_of=0, flag_af=0.

    Examples:
        >>> and16(0xFF00, 0x0FF0).result
        3840
    """
    bits_a = int_to_bits(a, 16)
    bits_b = int_to_bits(b, 16)
    result_bits = [AND(bits_a[i], bits_b[i]) for i in range(16)]
    result = bits_to_int(result_bits)
    return ALUResult8086(
        result=result,
        flag_cf=0,
        flag_of=0,
        flag_sf=result_bits[15],
        flag_zf=compute_zero(result_bits),
        flag_af=0,
        flag_pf=compute_parity(result_bits),
    )


def or16(a: int, b: int) -> ALUResult8086:
    """16-bit OR: A | B.

    16 OR gates in parallel.  CF=0, OF=0.

    Args:
        a: First 16-bit operand.
        b: Second 16-bit operand.

    Returns:
        ALUResult8086. flag_cf=0, flag_of=0, flag_af=0.

    Examples:
        >>> or16(0xFF00, 0x00FF).result
        65535
    """
    bits_a = int_to_bits(a, 16)
    bits_b = int_to_bits(b, 16)
    result_bits = [OR(bits_a[i], bits_b[i]) for i in range(16)]
    result = bits_to_int(result_bits)
    return ALUResult8086(
        result=result,
        flag_cf=0,
        flag_of=0,
        flag_sf=result_bits[15],
        flag_zf=compute_zero(result_bits),
        flag_af=0,
        flag_pf=compute_parity(result_bits),
    )


def xor16(a: int, b: int) -> ALUResult8086:
    """16-bit XOR: A ^ B.

    16 XOR gates in parallel.  CF=0, OF=0.

    Args:
        a: First 16-bit operand.
        b: Second 16-bit operand.

    Returns:
        ALUResult8086. flag_cf=0, flag_of=0, flag_af=0.

    Examples:
        >>> xor16(0xAAAA, 0x5555).result
        65535
    """
    bits_a = int_to_bits(a, 16)
    bits_b = int_to_bits(b, 16)
    result_bits = [XOR(bits_a[i], bits_b[i]) for i in range(16)]
    result = bits_to_int(result_bits)
    return ALUResult8086(
        result=result,
        flag_cf=0,
        flag_of=0,
        flag_sf=result_bits[15],
        flag_zf=compute_zero(result_bits),
        flag_af=0,
        flag_pf=compute_parity(result_bits),
    )


def inc16(a: int) -> ALUResult8086:
    """Increment 16-bit value by 1.  CF is NOT affected (caller preserves it).

    INC adds 1 via the adder.  The 8086's INC instruction does NOT modify CF.

    Returns:
        ALUResult8086. Caller must preserve CF from before the instruction.
    """
    r, _, af = add_16bit(a, 1, 0)
    bits_r = int_to_bits(r, 16)
    # Overflow: 0x7FFF + 1 → 0x8000
    of = 1 if a == 0x7FFF else 0
    return ALUResult8086(
        result=r,
        flag_cf=0,    # Caller ignores and preserves old CF
        flag_of=of,
        flag_sf=bits_r[15],
        flag_zf=compute_zero(bits_r),
        flag_af=af,
        flag_pf=compute_parity(bits_r),
    )


def dec16(a: int) -> ALUResult8086:
    """Decrement 16-bit value by 1.  CF is NOT affected (caller preserves it).

    DEC subtracts 1.  A - 1 = A + NOT(1) + 1 = A + 0xFFFE + 1 via adder.

    Returns:
        ALUResult8086. Caller must preserve CF.
    """
    # A - 1 = A + 0xFFFF via two's complement (0xFFFF = -1 mod 65536)
    r, _, _ = add_16bit(a, 0xFFFF, 0)
    bits_r = int_to_bits(r, 16)
    # Overflow: 0x8000 - 1 → 0x7FFF
    of = 1 if a == 0x8000 else 0
    # AF: nibble borrow of DEC (a - 1); correct formula, not two's complement carry
    af = nibble_borrow(a, 1, 0)
    return ALUResult8086(
        result=r,
        flag_cf=0,    # Caller preserves old CF
        flag_of=of,
        flag_sf=bits_r[15],
        flag_zf=compute_zero(bits_r),
        flag_af=af,
        flag_pf=compute_parity(bits_r),
    )


def neg16(a: int) -> ALUResult8086:
    """Negate 16-bit value: result = 0 - a.

    NEG is subtraction from zero: 0 + NOT(a) + 1.
    CF = 1 if a != 0 (i.e., borrow from 0 - nonzero).
    OF = 1 if a == 0x8000 (only signed overflow case).

    Args:
        a: 16-bit unsigned operand.

    Returns:
        ALUResult8086.
    """
    return sub16(0, a, 0)


def not16(a: int) -> int:
    """Bitwise NOT of 16-bit value.  No flags affected.

    16 NOT gates in parallel.

    Args:
        a: 16-bit value.

    Returns:
        Bitwise complement, masked to 16 bits.
    """
    return invert_16bit(a)


# ─── 8-bit arithmetic operations ──────────────────────────────────────────────


def add8(a: int, b: int, carry_in: int = 0) -> ALUResult8086:
    """8-bit addition: A + B + carry_in.

    Routes through 8 full-adder stages.

    Args:
        a:        First 8-bit operand (0–255).
        b:        Second 8-bit operand (0–255).
        carry_in: Carry in (0 or 1).

    Returns:
        ALUResult8086 with 8-bit result and all flag values.

    Examples:
        >>> add8(5, 3).result
        8
        >>> add8(0x7F, 1).flag_of   # 127 + 1 = 128: signed overflow
        1
        >>> add8(0xFF, 1).flag_cf   # unsigned overflow
        1
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)

    sums: list[int] = []
    carries: list[int] = []
    carry = carry_in
    for i in range(8):
        s, carry = full_adder(bits_a[i], bits_b[i], carry)
        sums.append(s)
        carries.append(carry)

    result = bits_to_int(sums)
    carry_into_7 = carries[6]
    carry_out_7 = carries[7]
    overflow = XOR(carry_into_7, carry_out_7)

    # AF: carry out of bit 3
    _, _, af = add_8bit(a, b, carry_in)

    return ALUResult8086(
        result=result,
        flag_cf=carries[7],
        flag_of=overflow,
        flag_sf=sums[7],
        flag_zf=compute_zero(sums),
        flag_af=af,
        flag_pf=compute_parity(sums),
    )


def sub8(a: int, b: int, borrow_in: int = 0) -> ALUResult8086:
    """8-bit subtraction: A - B - borrow_in.

    CF=1 means borrow occurred (A < B + borrow_in unsigned).

    Examples:
        >>> sub8(10, 3).result
        7
        >>> sub8(0, 1).flag_cf   # borrow
        1
    """
    not_b = invert_8bit(b)
    c_in = NOT(borrow_in)
    r, cout, _ = add_8bit(a, not_b, c_in)

    bits_r = int_to_bits(r, 8)
    bits_a = int_to_bits(a, 8)
    bits_nb = int_to_bits(not_b, 8)

    carries: list[int] = []
    carry = c_in
    for i in range(8):
        _, carry = full_adder(bits_a[i], bits_nb[i], carry)
        carries.append(carry)
    overflow = XOR(carries[6], carries[7])

    flag_cf = NOT(cout)

    # AF = 1 if nibble subtraction borrows (8086 correct formula for SUB)
    af = nibble_borrow(a, b, borrow_in)

    return ALUResult8086(
        result=r,
        flag_cf=flag_cf,
        flag_of=overflow,
        flag_sf=bits_r[7],
        flag_zf=compute_zero(bits_r),
        flag_af=af,
        flag_pf=compute_parity(bits_r),
    )


def and8(a: int, b: int) -> ALUResult8086:
    """8-bit AND: A & B.  CF=0, OF=0, AF=0."""
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    result_bits = [AND(bits_a[i], bits_b[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResult8086(
        result=result,
        flag_cf=0, flag_of=0,
        flag_sf=result_bits[7],
        flag_zf=compute_zero(result_bits),
        flag_af=0,
        flag_pf=compute_parity(result_bits),
    )


def or8(a: int, b: int) -> ALUResult8086:
    """8-bit OR: A | B.  CF=0, OF=0, AF=0."""
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    result_bits = [OR(bits_a[i], bits_b[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResult8086(
        result=result,
        flag_cf=0, flag_of=0,
        flag_sf=result_bits[7],
        flag_zf=compute_zero(result_bits),
        flag_af=0,
        flag_pf=compute_parity(result_bits),
    )


def xor8(a: int, b: int) -> ALUResult8086:
    """8-bit XOR: A ^ B.  CF=0, OF=0, AF=0."""
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    result_bits = [XOR(bits_a[i], bits_b[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResult8086(
        result=result,
        flag_cf=0, flag_of=0,
        flag_sf=result_bits[7],
        flag_zf=compute_zero(result_bits),
        flag_af=0,
        flag_pf=compute_parity(result_bits),
    )


def inc8(a: int) -> ALUResult8086:
    """Increment 8-bit value by 1.  CF unchanged (caller preserves)."""
    r, _, af = add_8bit(a, 1, 0)
    bits_r = int_to_bits(r, 8)
    of = 1 if a == 0x7F else 0
    return ALUResult8086(
        result=r,
        flag_cf=0,
        flag_of=of,
        flag_sf=bits_r[7],
        flag_zf=compute_zero(bits_r),
        flag_af=af,
        flag_pf=compute_parity(bits_r),
    )


def dec8(a: int) -> ALUResult8086:
    """Decrement 8-bit value by 1.  CF unchanged (caller preserves)."""
    r, _, _ = add_8bit(a, 0xFF, 0)
    bits_r = int_to_bits(r, 8)
    of = 1 if a == 0x80 else 0
    # AF: nibble borrow of DEC (a - 1); correct formula for 8086 AF
    af = nibble_borrow(a, 1, 0)
    return ALUResult8086(
        result=r,
        flag_cf=0,
        flag_of=of,
        flag_sf=bits_r[7],
        flag_zf=compute_zero(bits_r),
        flag_af=af,
        flag_pf=compute_parity(bits_r),
    )


def neg8(a: int) -> ALUResult8086:
    """Negate 8-bit value: 0 - a."""
    return sub8(0, a, 0)


def not8(a: int) -> int:
    """Bitwise NOT of 8-bit value.  No flags."""
    return invert_8bit(a)


# ─── Shift and rotate operations ──────────────────────────────────────────────


def shl(value: int, count: int, width: int) -> tuple[int, int]:
    """Logical left shift.  Returns (result, cf).

    CF = last bit shifted out (the bit that would be shifted into carry).
    OF = meaningful for count=1: OF = MSB(result) XOR CF.

    Routes bits through a shift-register model (gate primitives).

    Args:
        value: Value to shift.
        count: Shift count (0 means no shift).
        width: 8 or 16.

    Returns:
        (result, cf) where cf = last bit shifted out.

    Examples:
        >>> shl(0b00000001, 1, 8)
        (2, 0)
        >>> shl(0b10000000, 1, 8)
        (0, 1)
    """
    mask = (1 << width) - 1
    count = count & 0x1F
    if count == 0:
        return value & mask, 0
    bits = int_to_bits(value, width)
    # Last bit shifted out is bit (width - count)
    if count >= width:
        cf = 0
        result_bits = [0] * width
    else:
        # Gate model: shift is a rewiring.  The bit exiting at top is
        # bits[width - count] before shifting.
        cf = bits[width - count]
        result_bits = [0] * count + bits[:width - count]
    return bits_to_int(result_bits), cf


def shr(value: int, count: int, width: int) -> tuple[int, int]:
    """Logical right shift.  Returns (result, cf).

    CF = last bit shifted out (bit count-1 of original value).

    Examples:
        >>> shr(0b00000010, 1, 8)
        (1, 0)
        >>> shr(0b00000001, 1, 8)
        (0, 1)
    """
    mask = (1 << width) - 1
    count = count & 0x1F
    if count == 0:
        return value & mask, 0
    bits = int_to_bits(value, width)
    if count >= width:
        cf = 0
        result_bits = [0] * width
    else:
        cf = bits[count - 1]
        result_bits = bits[count:] + [0] * count
    return bits_to_int(result_bits), cf


def sar(value: int, count: int, width: int) -> tuple[int, int]:
    """Arithmetic right shift (sign-extending).  Returns (result, cf).

    Sign bit is replicated into vacated positions.

    Examples:
        >>> sar(0b10000000, 1, 8)
        (192, 0)
        >>> sar(0b10000001, 1, 8)
        (192, 1)
    """
    mask = (1 << width) - 1
    count = count & 0x1F
    if count == 0:
        return value & mask, 0
    bits = int_to_bits(value, width)
    sign_bit = bits[width - 1]  # MSB = sign
    if count >= width:
        result_bits = [sign_bit] * width
        cf = sign_bit
    else:
        cf = bits[count - 1]
        result_bits = bits[count:] + [sign_bit] * count
    return bits_to_int(result_bits), cf


def rol(value: int, count: int, width: int, cf_in: int) -> tuple[int, int]:
    """Rotate left (NOT through carry).  Returns (result, cf).

    Circular rotation: MSB wraps around to bit 0.

    Examples:
        >>> rol(0b10000000, 1, 8, 0)
        (1, 1)
    """
    mask = (1 << width) - 1
    count = count % width
    if count == 0:
        # CF = new LSB = bit 0 of value
        cf = int_to_bits(value, width)[0]
        return value & mask, cf
    bits = int_to_bits(value, width)
    result_bits = bits[width - count:] + bits[:width - count]
    cf = result_bits[0]  # New CF = new bit 0 (= old bit width-count)
    return bits_to_int(result_bits), cf


def ror(value: int, count: int, width: int, cf_in: int) -> tuple[int, int]:
    """Rotate right (NOT through carry).  Returns (result, cf).

    Circular rotation: LSB wraps around to MSB.

    Examples:
        >>> ror(0b00000001, 1, 8, 0)
        (128, 1)
    """
    mask = (1 << width) - 1
    count = count % width
    if count == 0:
        cf = int_to_bits(value, width)[width - 1]
        return value & mask, cf
    bits = int_to_bits(value, width)
    result_bits = bits[count:] + bits[:count]
    cf = result_bits[width - 1]  # New CF = new MSB (= old bit count-1)
    return bits_to_int(result_bits), cf


def rcl(value: int, count: int, width: int, cf_in: int) -> tuple[int, int]:
    """Rotate left through carry.  Returns (result, cf).

    (width+1)-bit rotation: [value, cf_in] rotated left by count.
    The carry bit is part of the rotation chain.

    Examples:
        >>> rcl(0b10000000, 1, 8, 0)
        (0, 1)
        >>> rcl(0b00000000, 1, 8, 1)
        (1, 0)
    """
    total = width + 1
    count = count % total
    if count == 0:
        return value & ((1 << width) - 1), cf_in
    # Build (width+1)-bit value: [bits of value] + [cf_in] at top
    bits = int_to_bits(value, width) + [cf_in]
    rotated = bits[total - count:] + bits[:total - count]
    new_cf = rotated[width]  # The carry bit position
    result_bits = rotated[:width]
    return bits_to_int(result_bits), new_cf


def rcr(value: int, count: int, width: int, cf_in: int) -> tuple[int, int]:
    """Rotate right through carry.  Returns (result, cf).

    (width+1)-bit rotation: [cf_in, value] rotated right by count.

    Examples:
        >>> rcr(0b00000001, 1, 8, 0)
        (0, 1)
        >>> rcr(0b00000000, 1, 8, 1)
        (128, 0)
    """
    total = width + 1
    count = count % total
    if count == 0:
        return value & ((1 << width) - 1), cf_in
    bits = int_to_bits(value, width) + [cf_in]
    rotated = bits[count:] + bits[:count]
    new_cf = rotated[width]
    result_bits = rotated[:width]
    return bits_to_int(result_bits), new_cf


# ─── Multiply / Divide (host arithmetic — gate-level too complex) ─────────────


def mul8(al: int, operand: int) -> tuple[int, int]:
    """Unsigned 8-bit multiply: AX = AL * operand.

    Returns (ax, cf_of) where cf_of = 1 if AH != 0.

    Note: Host arithmetic used (gate-level 8x8 multiplier is out of scope).
    """
    ax = (al & 0xFF) * (operand & 0xFF)
    cf_of = 1 if (ax >> 8) != 0 else 0
    return ax & 0xFFFF, cf_of


def mul16(ax: int, operand: int) -> tuple[int, int, int]:
    """Unsigned 16-bit multiply: DX:AX = AX * operand.

    Returns (dx, ax, cf_of) where cf_of = 1 if DX != 0.
    """
    result32 = (ax & 0xFFFF) * (operand & 0xFFFF)
    new_ax = result32 & 0xFFFF
    new_dx = (result32 >> 16) & 0xFFFF
    cf_of = 1 if new_dx != 0 else 0
    return new_dx, new_ax, cf_of


def imul8(al: int, operand: int) -> tuple[int, int]:
    """Signed 8-bit multiply: AX = AL_signed * operand_signed.

    Returns (ax, cf_of) where cf_of = 1 if AH != sign extension of AL.
    """
    a_s = al if al < 0x80 else al - 0x100
    b_s = operand if operand < 0x80 else operand - 0x100
    result16 = a_s * b_s
    ax = result16 & 0xFFFF
    expected_hi = 0xFF if (ax & 0x80) else 0
    cf_of = 1 if ((ax >> 8) & 0xFF) != expected_hi else 0
    return ax, cf_of


def imul16(ax: int, operand: int) -> tuple[int, int, int]:
    """Signed 16-bit multiply: DX:AX = AX_signed * operand_signed.

    Returns (dx, ax, cf_of) where cf_of = 1 if DX != sign extension of AX.
    """
    a_s = ax if ax < 0x8000 else ax - 0x10000
    b_s = operand if operand < 0x8000 else operand - 0x10000
    result32 = a_s * b_s
    new_ax = result32 & 0xFFFF
    new_dx = (result32 >> 16) & 0xFFFF
    expected_hi = 0xFFFF if (new_ax & 0x8000) else 0
    cf_of = 1 if new_dx != expected_hi else 0
    return new_dx, new_ax, cf_of


def div8(ax: int, operand: int) -> tuple[int, int]:
    """Unsigned 8-bit divide: AL = AX // operand, AH = AX % operand.

    Returns (al, ah).  Raises ZeroDivisionError if operand == 0.
    """
    if operand == 0:
        raise ZeroDivisionError("Division by zero")
    ax &= 0xFFFF
    q = (ax // operand) & 0xFF
    r = (ax % operand) & 0xFF
    return q, r


def div16(dx_ax: int, operand: int) -> tuple[int, int]:
    """Unsigned 16-bit divide: AX = DX:AX // operand, DX = DX:AX % operand.

    Returns (ax, dx).  Raises ZeroDivisionError if operand == 0.
    """
    if operand == 0:
        raise ZeroDivisionError("Division by zero")
    dx_ax &= 0xFFFFFFFF
    q = (dx_ax // operand) & 0xFFFF
    r = (dx_ax % operand) & 0xFFFF
    return q, r


def idiv8(ax: int, operand: int) -> tuple[int, int]:
    """Signed 8-bit divide.  Returns (al_quotient, ah_remainder).

    Raises ZeroDivisionError if operand == 0.
    """
    if operand == 0:
        raise ZeroDivisionError("Division by zero")
    ax16 = ax & 0xFFFF
    dividend = ax16 if ax16 < 0x8000 else ax16 - 0x10000
    divisor = operand if operand < 0x80 else operand - 0x100
    q = int(dividend / divisor)
    r = dividend - q * divisor
    return q & 0xFF, r & 0xFF


def idiv16(dx_ax: int, operand: int) -> tuple[int, int]:
    """Signed 16-bit divide.  Returns (ax_quotient, dx_remainder).

    Raises ZeroDivisionError if operand == 0.
    """
    if operand == 0:
        raise ZeroDivisionError("Division by zero")
    d32 = dx_ax & 0xFFFFFFFF
    dividend = d32 if d32 < 0x80000000 else d32 - 0x100000000
    divisor = operand if operand < 0x8000 else operand - 0x10000
    q = int(dividend / divisor)
    r = dividend - q * divisor
    return q & 0xFFFF, r & 0xFFFF


# ─── BCD operations ───────────────────────────────────────────────────────────


def daa(al: int, flag_af: int, flag_cf: int) -> tuple[int, int, int]:
    """DAA — Decimal Adjust AL after Addition.

    Adjusts AL to a valid BCD digit pair after a BCD addition.
    Uses gate-level add_8bit for the correction adds.

    Returns: (result_al, new_af, new_cf)
    """
    old_al = al & 0xFF
    new_cf = flag_cf
    new_af = flag_af

    if (old_al & 0xF) > 9 or flag_af:
        al, _, _ = add_8bit(old_al, 6, 0)
        al &= 0xFF
        new_af = 1
    else:
        new_af = 0

    if old_al > 0x99 or flag_cf:
        al, _, _ = add_8bit(al, 0x60, 0)
        al &= 0xFF
        new_cf = 1
    else:
        new_cf = 0

    return al & 0xFF, new_af, new_cf


def das(al: int, flag_af: int, flag_cf: int) -> tuple[int, int, int]:
    """DAS — Decimal Adjust AL after Subtraction.

    Returns: (result_al, new_af, new_cf)
    """
    old_al = al & 0xFF
    new_cf = flag_cf
    new_af = flag_af
    result = old_al

    if (old_al & 0xF) > 9 or flag_af:
        r, _, _ = add_8bit(result, invert_8bit(6), 1)  # subtract 6
        result = r & 0xFF
        new_af = 1
    else:
        new_af = 0

    if old_al > 0x99 or flag_cf:
        r, _, _ = add_8bit(result, invert_8bit(0x60), 1)  # subtract 0x60
        result = r & 0xFF
        new_cf = 1
    else:
        new_cf = 0

    return result & 0xFF, new_af, new_cf


def aaa(al: int, ah: int, flag_af: int) -> tuple[int, int, int]:
    """AAA — ASCII Adjust after Addition.

    Returns: (new_al, new_ah, af_cf)
    """
    if (al & 0xF) > 9 or flag_af:
        al_out, _, _ = add_8bit(al & 0xFF, 6, 0)
        ah_out, _, _ = add_8bit(ah & 0xFF, 1, 0)
        af_cf = 1
    else:
        al_out = al & 0xFF
        ah_out = ah & 0xFF
        af_cf = 0
    return al_out & 0x0F, ah_out & 0xFF, af_cf


def aas(al: int, ah: int, flag_af: int) -> tuple[int, int, int]:
    """AAS — ASCII Adjust after Subtraction.

    Returns: (new_al, new_ah, af_cf)
    """
    if (al & 0xF) > 9 or flag_af:
        al_out, _, _ = add_8bit(al & 0xFF, invert_8bit(6), 1)
        ah_out, _, _ = add_8bit(ah & 0xFF, invert_8bit(1), 1)
        af_cf = 1
    else:
        al_out = al & 0xFF
        ah_out = ah & 0xFF
        af_cf = 0
    return al_out & 0x0F, ah_out & 0xFF, af_cf


def aam(al: int, base: int = 10) -> tuple[int, int]:
    """AAM — ASCII Adjust after Multiply.

    AH = AL // base, AL = AL % base.

    Returns: (new_ah, new_al)
    """
    al &= 0xFF
    ah = al // base
    new_al = al % base
    return ah & 0xFF, new_al & 0xFF


def aad(ah: int, al: int, base: int = 10) -> int:
    """AAD — ASCII Adjust before Division.

    AL = AH * base + AL
     AH = 0.

    Returns: new_al (AH is set to 0 by caller).
    """
    result, _, _ = add_8bit((ah * base) & 0xFF, al & 0xFF, 0)
    return result & 0xFF
