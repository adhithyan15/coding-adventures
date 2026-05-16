"""ALU for the Motorola 68000 gate-level simulator.

=== Architecture ===

The 68000 is a 32-bit CPU internally.  The ALU processes byte (8), word (16),
and longword (32) operands.  All additions and subtractions route through
ripple_carry_adder chains.  Logical operations (AND, OR, XOR, NOT) use
parallel gate arrays.  Shifts and rotates are implemented as bit-array rewiring.

=== Gate count estimate for this ALU ===

Component                        Gates
──────────────────────────────── ─────
32-bit ripple adder              ~160  (32 full adders × ~5 gates each)
32-bit NOT (for SUB)             32
32-bit AND/OR/XOR                32 each
Zero NOR tree (32-bit)           ~40
Overflow XOR gate                1
Shifter/rotator                  ~128
──────────────────────────────── ─────
Total ALU estimate               ~600+ gates

=== 68000 Flag Rules ===

ADD/ADDX:
  C = X = carry_out of adder
  N = MSB(result)
  Z = (result == 0)
  V = XOR(carry into MSB, carry out of MSB)

  For ADDX: Z is only CLEARED, never set:
    z_out = flag_z AND compute_zero(result)
    (Z stays 1 only if it was 1 AND new result is also zero)

SUB/SUBX:
  C = X = NOT(carry_out of adder)  [borrow convention]
  N = MSB(result)
  Z = (result == 0)
  V = XOR(carry into MSB, carry out of MSB)

  For SUBX: Same Z rule as ADDX.

AND/OR/XOR:
  C = 0
  V = 0
  X = unchanged
  N = MSB(result)
  Z = (result == 0)

MOVE:
  C = 0
  V = 0
  X = unchanged
  N = MSB(result)
  Z = (result == 0)

CMP: Same as SUB but X is not modified.

NEG: 0 - operand.  C = (result != 0), X = C.

=== Overflow Detection ===

For N-bit signed addition A + B = R:
  OF = XOR(carry_into_bit[N-1], carry_out_of_bit[N-1])

This is a single XOR gate at the MSB stage of the adder.

=== Subtraction via Two's Complement ===

SUB A, B   →  A + NOT(B) + 1         (carry_in = 1)
SUBX A, B, X  →  A + NOT(B) + NOT(X) (carry_in = NOT(extend))

The 68000 CF convention: CF=1 means borrow occurred (A < B unsigned).
After SUB: CF = NOT(carry_out_of_adder).

=== MUL/DIV Note ===

A gate-level 16×16 signed multiplier requires ~1000 gates.  For educational
purposes, MULS/MULU/DIVS/DIVU use Python host arithmetic.  All other ops
(ADD/SUB/AND/OR/XOR/NOT/shifts/rotates) use the full gate-level path.
"""

from __future__ import annotations

from dataclasses import dataclass

from arithmetic import full_adder
from logic_gates import AND, NOT, OR, XOR

from motorola68k_gatelevel.bits import (
    bits_to_int,
    compute_zero,
    int_to_bits,
    invert_8bit,
    invert_16bit,
    invert_32bit,
)


@dataclass
class ALUResult68k:
    """Result of an ALU operation on the Motorola 68000.

    Contains the computed value plus all five condition-code flag values
    that the 68000 ALU can affect.  The caller decides which flags to commit
    (e.g. CMP does not update X; AND/OR/XOR clear V and C; ADDX/SUBX have
    special Z-flag rules).

    Fields:
        result:   ALU output (8/16/32-bit, caller knows which).
        flag_c:   Carry (unsigned overflow/borrow from MSB).
        flag_x:   Extend (same as C for ADD/SUB; unchanged for logic ops).
        flag_n:   Negative (copy of MSB of result).
        flag_z:   Zero (1 if result == 0).
        flag_v:   Overflow (signed overflow occurred).

    Examples:
        >>> r = ALUResult68k(result=8, flag_c=0, flag_x=0, flag_n=0, flag_z=0, flag_v=0)
        >>> r.result
        8
    """

    result: int
    flag_c: int
    flag_x: int
    flag_n: int
    flag_z: int
    flag_v: int


# ─── Internal adder helpers ────────────────────────────────────────────────────


def _add_with_overflow(a: int, b: int, carry_in: int, width: int) -> ALUResult68k:
    """Generic signed-add through a ripple-carry chain of ``width`` bits.

    Computes A + B + carry_in, tracking the carry into the MSB and out of
    the MSB so we can derive the overflow flag via a single XOR gate.

    This is the heart of the gate-level ALU: individual full_adder gates
    are invoked bit-by-bit, exactly as they would be on the silicon.

    Args:
        a:        First operand (unsigned, ``width`` bits).
        b:        Second operand (unsigned, ``width`` bits).
        carry_in: Initial carry (0 or 1).
        width:    Number of bits (8, 16, or 32).

    Returns:
        ALUResult68k with result, carry_out, and correct overflow.
    """
    bits_a = int_to_bits(a, width)
    bits_b = int_to_bits(b, width)

    sums: list[int] = []
    carries: list[int] = []
    carry = carry_in
    for i in range(width):
        s, carry = full_adder(bits_a[i], bits_b[i], carry)
        sums.append(s)
        carries.append(carry)

    result = bits_to_int(sums)

    # Overflow: XOR(carry into MSB, carry out of MSB).
    # carry into MSB = carries[width-2]  (carry out of bit width-2)
    # carry out of MSB = carries[width-1]
    carry_into_msb = carries[width - 2] if width >= 2 else carry_in
    carry_out_msb = carries[width - 1]
    overflow = XOR(carry_into_msb, carry_out_msb)

    return ALUResult68k(
        result=result,
        flag_c=carry_out_msb,
        flag_x=carry_out_msb,
        flag_n=sums[width - 1],
        flag_z=compute_zero(sums),
        flag_v=overflow,
    )


def _sub_with_overflow(a: int, b: int, extend_in: int, width: int) -> ALUResult68k:
    """Generic subtraction A - B - extend_in via two's complement.

    Gate path:
      1. width NOT gates: NOT_B[i] = NOT(B[i])
      2. carry_in = NOT(extend_in)  — one NOT gate
      3. ripple_carry_adder(A, NOT_B, carry_in)
      4. OF = XOR(carry_into_msb, carry_out_of_msb)
      5. CF = NOT(carry_out)  — borrow occurred when carry did NOT propagate

    Args:
        a:         Minuend (unsigned, ``width`` bits).
        b:         Subtrahend (unsigned, ``width`` bits).
        extend_in: Incoming borrow/extend (0 or 1).
        width:     Number of bits (8, 16, or 32).

    Returns:
        ALUResult68k. flag_c = 1 means borrow (A < B + extend_in unsigned).
    """
    # NOT(B) via width NOT gates
    not_b = invert_8bit(b) if width == 8 else (
        invert_16bit(b) if width == 16 else invert_32bit(b)
    )
    # carry_in = NOT(extend_in): with extend_in=0 → carry_in=1 (normal sub)
    c_in = NOT(extend_in)

    bits_a = int_to_bits(a, width)
    bits_nb = int_to_bits(not_b, width)

    sums: list[int] = []
    carries: list[int] = []
    carry = c_in
    for i in range(width):
        s, carry = full_adder(bits_a[i], bits_nb[i], carry)
        sums.append(s)
        carries.append(carry)

    result = bits_to_int(sums)

    carry_into_msb = carries[width - 2] if width >= 2 else c_in
    carry_out_msb = carries[width - 1]
    overflow = XOR(carry_into_msb, carry_out_msb)

    # CF = 1 if borrow occurred = NOT(carry_out_of_adder)
    flag_c = NOT(carry_out_msb)

    return ALUResult68k(
        result=result,
        flag_c=flag_c,
        flag_x=flag_c,
        flag_n=sums[width - 1],
        flag_z=compute_zero(sums),
        flag_v=overflow,
    )


# ─── 32-bit operations (primary — 68k is 32-bit internally) ───────────────────


def add32(a: int, b: int, extend_in: int = 0) -> ALUResult68k:
    """32-bit addition: A + B + extend_in.

    Routes through 32 full-adder stages.  Used by ADD.L and ADDX.L.

    For ADDX, the Z flag has special behavior: Z is only cleared (never
    set).  The caller must apply: ``result.flag_z = old_z AND result.flag_z``.

    Args:
        a:          First 32-bit operand (0–0xFFFFFFFF).
        b:          Second 32-bit operand (0–0xFFFFFFFF).
        extend_in:  Extend flag (for ADDX; 0 for normal ADD).

    Returns:
        ALUResult68k with all flags.

    Examples:
        >>> add32(5, 3).result
        8
        >>> add32(0x7FFFFFFF, 1).flag_v   # signed overflow: max positive + 1
        1
        >>> add32(0xFFFFFFFF, 1).flag_c   # unsigned overflow
        1
    """
    return _add_with_overflow(a, b, extend_in, 32)


def sub32(a: int, b: int, extend_in: int = 0) -> ALUResult68k:
    """32-bit subtraction: A - B - extend_in.

    Two's complement subtraction via gate chain.

    Args:
        a:          Minuend (0–0xFFFFFFFF).
        b:          Subtrahend (0–0xFFFFFFFF).
        extend_in:  Borrow/extend for SUBX (0 for normal SUB).

    Returns:
        ALUResult68k. flag_c = 1 means borrow (A < B unsigned).

    Examples:
        >>> sub32(10, 3).result
        7
        >>> sub32(10, 3).flag_c   # no borrow
        0
        >>> sub32(0, 1).flag_c    # borrow
        1
        >>> sub32(0x80000000, 1).flag_v   # signed overflow: min negative - 1
        1
    """
    return _sub_with_overflow(a, b, extend_in, 32)


def and32(a: int, b: int) -> ALUResult68k:
    """32-bit AND: A & B.

    32 AND gates in parallel.  V=0, C=0, X unchanged, N=MSB, Z=(result==0).

    Args:
        a: First 32-bit operand.
        b: Second 32-bit operand.

    Returns:
        ALUResult68k. flag_c=0, flag_v=0, flag_x=0 (caller must preserve X).

    Examples:
        >>> and32(0xFF000000, 0x0F000000).result
        251658240
        >>> and32(0, 0xFFFFFFFF).flag_z
        1
    """
    bits_a = int_to_bits(a, 32)
    bits_b = int_to_bits(b, 32)
    result_bits = [AND(bits_a[i], bits_b[i]) for i in range(32)]
    result = bits_to_int(result_bits)
    return ALUResult68k(
        result=result,
        flag_c=0,
        flag_x=0,  # caller preserves X
        flag_n=result_bits[31],
        flag_z=compute_zero(result_bits),
        flag_v=0,
    )


def or32(a: int, b: int) -> ALUResult68k:
    """32-bit OR: A | B.

    32 OR gates in parallel.  V=0, C=0, X unchanged.

    Args:
        a: First 32-bit operand.
        b: Second 32-bit operand.

    Returns:
        ALUResult68k. flag_c=0, flag_v=0.

    Examples:
        >>> or32(0xFF000000, 0x00FFFFFF).result
        4294967295
        >>> or32(0, 0).flag_z
        1
    """
    bits_a = int_to_bits(a, 32)
    bits_b = int_to_bits(b, 32)
    result_bits = [OR(bits_a[i], bits_b[i]) for i in range(32)]
    result = bits_to_int(result_bits)
    return ALUResult68k(
        result=result,
        flag_c=0,
        flag_x=0,
        flag_n=result_bits[31],
        flag_z=compute_zero(result_bits),
        flag_v=0,
    )


def xor32(a: int, b: int) -> ALUResult68k:
    """32-bit XOR: A ^ B.

    32 XOR gates in parallel.  V=0, C=0, X unchanged.

    Args:
        a: First 32-bit operand.
        b: Second 32-bit operand.

    Returns:
        ALUResult68k. flag_c=0, flag_v=0.

    Examples:
        >>> xor32(0xAAAAAAAA, 0x55555555).result
        4294967295
        >>> xor32(0xDEAD, 0xDEAD).flag_z
        1
    """
    bits_a = int_to_bits(a, 32)
    bits_b = int_to_bits(b, 32)
    result_bits = [XOR(bits_a[i], bits_b[i]) for i in range(32)]
    result = bits_to_int(result_bits)
    return ALUResult68k(
        result=result,
        flag_c=0,
        flag_x=0,
        flag_n=result_bits[31],
        flag_z=compute_zero(result_bits),
        flag_v=0,
    )


def not32(a: int) -> int:
    """Bitwise NOT of a 32-bit value.  32 NOT gates.  No flag effects.

    The 68000 NOT instruction sets N and Z based on the result, but
    the ALU NOT operation itself has no flag-setting logic; the caller
    handles flags.

    Args:
        a: 32-bit value (0–0xFFFFFFFF).

    Returns:
        Bitwise complement, masked to 32 bits.

    Examples:
        >>> not32(0)
        4294967295
        >>> not32(0xFFFFFFFF)
        0
        >>> not32(0xAAAAAAAA)
        1431655765
    """
    return invert_32bit(a)


def neg32(a: int) -> ALUResult68k:
    """Negate 32-bit value: result = 0 - a.

    NEG is subtraction from zero.  Gate path: 0 + NOT(a) + 1.

    Special case: C = 1 if a != 0, C = 0 if a == 0.
    This differs from sub32(0, a) where CF convention applies.

    Args:
        a: 32-bit unsigned operand.

    Returns:
        ALUResult68k.

    Examples:
        >>> neg32(1).result
        4294967295
        >>> neg32(0).flag_c
        0
        >>> neg32(1).flag_c
        1
    """
    r = sub32(0, a, 0)
    # NEG: C = (a != 0); sub32 gives CF = NOT(carry_out) = 1 when borrow
    # sub32(0, 0) → no borrow → CF=0; sub32(0, nonzero) → borrow → CF=1
    # This matches: C=0 when a==0, C=1 when a!=0. ✓
    return r


def cmp32(a: int, b: int) -> ALUResult68k:
    """32-bit compare: flags set as if (A - B) but result discarded.

    X flag is NOT modified by CMP (caller must not commit flag_x).

    Args:
        a: Minuend (value to compare from).
        b: Subtrahend (value to compare against).

    Returns:
        ALUResult68k. Caller discards result and does not update X.

    Examples:
        >>> cmp32(5, 3).flag_c   # 5 >= 3, no borrow
        0
        >>> cmp32(3, 5).flag_c   # 3 < 5, borrow
        1
        >>> cmp32(5, 5).flag_z   # equal
        1
    """
    return sub32(a, b, 0)


# ─── 16-bit operations (word) ─────────────────────────────────────────────────


def add16(a: int, b: int, extend_in: int = 0) -> ALUResult68k:
    """16-bit addition: A + B + extend_in.

    Routes through 16 full-adder stages.

    Examples:
        >>> add16(5, 3).result
        8
        >>> add16(0x7FFF, 1).flag_v
        1
        >>> add16(0xFFFF, 1).flag_c
        1
    """
    return _add_with_overflow(a, b, extend_in, 16)


def sub16(a: int, b: int, extend_in: int = 0) -> ALUResult68k:
    """16-bit subtraction: A - B - extend_in.

    Examples:
        >>> sub16(10, 3).result
        7
        >>> sub16(0, 1).flag_c
        1
    """
    return _sub_with_overflow(a, b, extend_in, 16)


def and16(a: int, b: int) -> ALUResult68k:
    """16-bit AND: A & B.  V=0, C=0, X unchanged.

    Examples:
        >>> and16(0xFF00, 0x0FF0).result
        3840
    """
    bits_a = int_to_bits(a, 16)
    bits_b = int_to_bits(b, 16)
    result_bits = [AND(bits_a[i], bits_b[i]) for i in range(16)]
    result = bits_to_int(result_bits)
    return ALUResult68k(
        result=result, flag_c=0, flag_x=0,
        flag_n=result_bits[15], flag_z=compute_zero(result_bits), flag_v=0,
    )


def or16(a: int, b: int) -> ALUResult68k:
    """16-bit OR: A | B.  V=0, C=0, X unchanged.

    Examples:
        >>> or16(0xFF00, 0x00FF).result
        65535
    """
    bits_a = int_to_bits(a, 16)
    bits_b = int_to_bits(b, 16)
    result_bits = [OR(bits_a[i], bits_b[i]) for i in range(16)]
    result = bits_to_int(result_bits)
    return ALUResult68k(
        result=result, flag_c=0, flag_x=0,
        flag_n=result_bits[15], flag_z=compute_zero(result_bits), flag_v=0,
    )


def xor16(a: int, b: int) -> ALUResult68k:
    """16-bit XOR: A ^ B.  V=0, C=0, X unchanged.

    Examples:
        >>> xor16(0xAAAA, 0x5555).result
        65535
    """
    bits_a = int_to_bits(a, 16)
    bits_b = int_to_bits(b, 16)
    result_bits = [XOR(bits_a[i], bits_b[i]) for i in range(16)]
    result = bits_to_int(result_bits)
    return ALUResult68k(
        result=result, flag_c=0, flag_x=0,
        flag_n=result_bits[15], flag_z=compute_zero(result_bits), flag_v=0,
    )


def not16(a: int) -> int:
    """Bitwise NOT of 16-bit value.  16 NOT gates.  No flags.

    Examples:
        >>> not16(0)
        65535
        >>> not16(0xFFFF)
        0
    """
    return invert_16bit(a)


def neg16(a: int) -> ALUResult68k:
    """Negate 16-bit value: 0 - a.

    Examples:
        >>> neg16(1).result
        65535
        >>> neg16(0).flag_c
        0
    """
    return sub16(0, a, 0)


def cmp16(a: int, b: int) -> ALUResult68k:
    """16-bit compare.  X not modified.

    Examples:
        >>> cmp16(5, 3).flag_c
        0
        >>> cmp16(3, 5).flag_c
        1
    """
    return sub16(a, b, 0)


# ─── 8-bit operations (byte) ──────────────────────────────────────────────────


def add8(a: int, b: int, extend_in: int = 0) -> ALUResult68k:
    """8-bit addition: A + B + extend_in.

    Routes through 8 full-adder stages.

    Examples:
        >>> add8(5, 3).result
        8
        >>> add8(0x7F, 1).flag_v
        1
        >>> add8(0xFF, 1).flag_c
        1
    """
    return _add_with_overflow(a, b, extend_in, 8)


def sub8(a: int, b: int, extend_in: int = 0) -> ALUResult68k:
    """8-bit subtraction: A - B - extend_in.

    Examples:
        >>> sub8(10, 3).result
        7
        >>> sub8(0, 1).flag_c
        1
    """
    return _sub_with_overflow(a, b, extend_in, 8)


def and8(a: int, b: int) -> ALUResult68k:
    """8-bit AND: A & B.  V=0, C=0, X unchanged.

    Examples:
        >>> and8(0xF0, 0x0F).result
        0
        >>> and8(0xFF, 0xFF).result
        255
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    result_bits = [AND(bits_a[i], bits_b[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResult68k(
        result=result, flag_c=0, flag_x=0,
        flag_n=result_bits[7], flag_z=compute_zero(result_bits), flag_v=0,
    )


def or8(a: int, b: int) -> ALUResult68k:
    """8-bit OR: A | B.  V=0, C=0, X unchanged.

    Examples:
        >>> or8(0xF0, 0x0F).result
        255
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    result_bits = [OR(bits_a[i], bits_b[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResult68k(
        result=result, flag_c=0, flag_x=0,
        flag_n=result_bits[7], flag_z=compute_zero(result_bits), flag_v=0,
    )


def xor8(a: int, b: int) -> ALUResult68k:
    """8-bit XOR: A ^ B.  V=0, C=0, X unchanged.

    Examples:
        >>> xor8(0xFF, 0xFF).result
        0
        >>> xor8(0xAA, 0x55).result
        255
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    result_bits = [XOR(bits_a[i], bits_b[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResult68k(
        result=result, flag_c=0, flag_x=0,
        flag_n=result_bits[7], flag_z=compute_zero(result_bits), flag_v=0,
    )


def not8(a: int) -> int:
    """Bitwise NOT of 8-bit value.  8 NOT gates.  No flags.

    Examples:
        >>> not8(0)
        255
        >>> not8(0xFF)
        0
    """
    return invert_8bit(a)


def neg8(a: int) -> ALUResult68k:
    """Negate 8-bit value: 0 - a.

    Examples:
        >>> neg8(1).result
        255
        >>> neg8(0).flag_c
        0
    """
    return sub8(0, a, 0)


def cmp8(a: int, b: int) -> ALUResult68k:
    """8-bit compare.  X not modified.

    Examples:
        >>> cmp8(5, 3).flag_c
        0
        >>> cmp8(3, 5).flag_c
        1
    """
    return sub8(a, b, 0)


# ─── Shift and rotate operations ──────────────────────────────────────────────


def asl(value: int, count: int, width: int) -> tuple[int, int, int]:
    """Arithmetic shift left.  Returns (result, c, v).

    ASL shifts the operand left by count positions.  Vacated bits are
    filled with zeros.  The last bit shifted out of the MSB is captured
    as C.

    V (overflow) is set if ANY bit shifted out differed from the original
    MSB (sign change occurred during shift).  For count==0: C=0, V=0.

    Gate model: rewiring of bits.  For count=1, this is a single pass
    of AND-gate-based conditional routing.

    Args:
        value: Operand (unsigned, ``width`` bits).
        count: Shift count (0–63; 68k uses mod 64 or 8 for memory).
        width: 8, 16, or 32.

    Returns:
        (result, c_flag, v_flag)

    Examples:
        >>> asl(0b00000001, 1, 8)
        (2, 0, 0)
        >>> asl(0b10000000, 1, 8)
        (0, 1, 0)
        >>> asl(0b01000000, 1, 8)   # bit 7 changes: V=1
        (128, 0, 1)
    """
    mask = (1 << width) - 1
    if count == 0:
        return value & mask, 0, 0

    bits = int_to_bits(value, width)
    original_msb = bits[width - 1]

    # Track V: set if any bit shifted through the MSB position changes the sign
    v = 0
    for i in range(count):
        if count - 1 - i < width:
            bit_leaving = bits[width - 1 - i] if i < width else 0
            if bit_leaving != original_msb:
                v = 1

    if count >= width:
        cf = 0
        result_bits = [0] * width
    else:
        # Last bit to exit is bits[width - count]
        cf = bits[width - count]
        result_bits = [0] * count + bits[:width - count]

    # Final V: did any bit exit that != original MSB, OR does new MSB differ?
    # Simpler: V = 1 if any exited bit != original_msb
    # Also check if count > 0 and value != 0: just check the shifted bits
    v = 0
    if count > 0:
        # V = 1 if MSB changed at any point during the shift.
        # Equivalently: any of the top (count+1) bits of original differ.
        top_bits = bits[width - count - 1: width] if count < width else bits
        if len(set(top_bits + [original_msb])) > 1:
            v = 1

    return bits_to_int(result_bits), cf, v


def asr(value: int, count: int, width: int) -> tuple[int, int]:
    """Arithmetic shift right (sign-extending).  Returns (result, c).

    Shifts right by count positions.  Sign bit (MSB) is replicated into
    vacated positions.  C = last bit shifted out.

    For count==0: C=0, result unchanged.

    Args:
        value: Operand (unsigned, ``width`` bits).
        count: Shift count.
        width: 8, 16, or 32.

    Returns:
        (result, c_flag)

    Examples:
        >>> asr(0b10000000, 1, 8)
        (192, 0)
        >>> asr(0b10000001, 1, 8)
        (192, 1)
        >>> asr(0b01000000, 1, 8)
        (32, 0)
    """
    mask = (1 << width) - 1
    if count == 0:
        return value & mask, 0

    bits = int_to_bits(value, width)
    sign_bit = bits[width - 1]

    if count >= width:
        result_bits = [sign_bit] * width
        cf = sign_bit
    else:
        cf = bits[count - 1]
        result_bits = bits[count:] + [sign_bit] * count

    return bits_to_int(result_bits), cf


def lsl(value: int, count: int, width: int) -> tuple[int, int]:
    """Logical shift left.  Returns (result, c).

    Same as ASL but V flag handling is the caller's concern.
    C = last bit shifted out (bit at position width-count).

    Args:
        value: Operand (unsigned, ``width`` bits).
        count: Shift count.
        width: 8, 16, or 32.

    Returns:
        (result, c_flag)

    Examples:
        >>> lsl(0b00000001, 1, 8)
        (2, 0)
        >>> lsl(0b10000000, 1, 8)
        (0, 1)
    """
    mask = (1 << width) - 1
    if count == 0:
        return value & mask, 0

    bits = int_to_bits(value, width)

    if count >= width:
        cf = 0
        result_bits = [0] * width
    else:
        cf = bits[width - count]
        result_bits = [0] * count + bits[:width - count]

    return bits_to_int(result_bits), cf


def lsr(value: int, count: int, width: int) -> tuple[int, int]:
    """Logical shift right.  Returns (result, c).

    Zeros fill from the left.  C = last bit shifted out.

    Args:
        value: Operand (unsigned, ``width`` bits).
        count: Shift count.
        width: 8, 16, or 32.

    Returns:
        (result, c_flag)

    Examples:
        >>> lsr(0b00000010, 1, 8)
        (1, 0)
        >>> lsr(0b00000001, 1, 8)
        (0, 1)
    """
    mask = (1 << width) - 1
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


def rol(value: int, count: int, width: int) -> tuple[int, int]:
    """Rotate left (not through carry).  Returns (result, c).

    Circular rotation: MSB wraps around to bit 0.
    C = MSB of result (i.e. the bit that was just wrapped).

    For count==0: C = MSB of original value, result unchanged.

    Args:
        value: Operand (unsigned, ``width`` bits).
        count: Rotation count.
        width: 8, 16, or 32.

    Returns:
        (result, c_flag)

    Examples:
        >>> rol(0b10000000, 1, 8)
        (1, 0)
        >>> rol(0b01000000, 1, 8)
        (128, 0)
        >>> rol(0b10000001, 1, 8)
        (3, 1)
    """
    mask = (1 << width) - 1
    count = count % width if width > 0 else 0
    bits = int_to_bits(value, width)

    if count == 0:
        # C = new bit 0 (which is the old MSB in the rotate path)
        # Per 68k manual: C = bit shifted out = what would wrap around
        cf = bits[width - 1]
        return value & mask, cf

    result_bits = bits[width - count:] + bits[:width - count]
    # C = bit that just wrapped into bit 0 = old bit (width - count)
    # Which is now result_bits[0]... no: C = last bit rotated out = old MSB
    cf = result_bits[0]  # bit 0 of result = old bit (width-count) = last wrapped
    return bits_to_int(result_bits), cf


def ror(value: int, count: int, width: int) -> tuple[int, int]:
    """Rotate right (not through carry).  Returns (result, c).

    Circular rotation: LSB wraps around to MSB.
    C = LSB of result (the bit that was just wrapped to MSB position).

    For count==0: C = LSB of original value.

    Args:
        value: Operand (unsigned, ``width`` bits).
        count: Rotation count.
        width: 8, 16, or 32.

    Returns:
        (result, c_flag)

    Examples:
        >>> ror(0b00000001, 1, 8)
        (128, 0)
        >>> ror(0b10000000, 1, 8)
        (64, 1)
    """
    mask = (1 << width) - 1
    count = count % width if width > 0 else 0
    bits = int_to_bits(value, width)

    if count == 0:
        cf = bits[0]  # C = what would have wrapped = bit 0
        return value & mask, cf

    result_bits = bits[count:] + bits[:count]
    # C = last bit rotated out = old bit (count-1) = now at MSB
    cf = result_bits[width - 1]
    return bits_to_int(result_bits), cf


def roxl(value: int, count: int, width: int, x: int) -> tuple[int, int]:
    """Rotate left through X (extend) flag.  Returns (result, c).

    The X flag is part of the rotation chain: (width+1)-bit rotation.
    Bit order during rotation: [x, bit(width-1), ..., bit(0)] → left shift.

    For count==0: C = X flag (unchanged), result unchanged.

    Args:
        value: Operand (unsigned, ``width`` bits).
        count: Rotation count.
        width: 8, 16, or 32.
        x:     Current X (extend) flag.

    Returns:
        (result, c_flag)  — caller must set X = c_flag after each step.

    Examples:
        >>> roxl(0b10000000, 1, 8, 0)
        (0, 1)
        >>> roxl(0b00000000, 1, 8, 1)
        (1, 0)
    """
    mask = (1 << width) - 1
    total = width + 1
    count = count % total
    bits = int_to_bits(value, width) + [x]  # bit 0..width-1 = value, bit width = X

    if count == 0:
        # C=X, result unchanged per 68k spec
        return value & mask, x

    rotated = bits[total - count:] + bits[:total - count]
    new_x = rotated[width]   # X is the top bit of the (width+1) ring
    result_bits = rotated[:width]
    # C = new X
    cf = new_x
    return bits_to_int(result_bits), cf


def roxr(value: int, count: int, width: int, x: int) -> tuple[int, int]:
    """Rotate right through X (extend) flag.  Returns (result, c).

    (width+1)-bit rotation to the right.  X enters from the top (MSB side).

    For count==0: C = X flag, result unchanged.

    Args:
        value: Operand (unsigned, ``width`` bits).
        count: Rotation count.
        width: 8, 16, or 32.
        x:     Current X (extend) flag.

    Returns:
        (result, c_flag)  — caller sets X = c_flag.

    Examples:
        >>> roxr(0b00000001, 1, 8, 0)
        (0, 1)
        >>> roxr(0b00000000, 1, 8, 1)
        (128, 0)
    """
    mask = (1 << width) - 1
    total = width + 1
    count = count % total
    bits = int_to_bits(value, width) + [x]

    if count == 0:
        return value & mask, x

    rotated = bits[count:] + bits[:count]
    new_x = rotated[width]
    result_bits = rotated[:width]
    cf = new_x
    return bits_to_int(result_bits), cf


# ─── Multiply / Divide (host arithmetic — gate-level too complex) ─────────────


def muls(d_val: int, src_val: int) -> tuple[int, int, int]:
    """Signed 16×16 → 32 multiply.  Returns (result32, n, z).

    MULS: Dn[31:0] = Dn[15:0] × src[15:0] (signed).
    V=0, C=0 always.  X unchanged.

    Gate-level 16×16 signed multiplier requires ~1000 gates; host arithmetic
    is used here as a principled exception.

    Args:
        d_val:   Data register value (only low 16 bits used).
        src_val: Source operand (16 bits, treated as signed).

    Returns:
        (result32, n_flag, z_flag)

    Examples:
        >>> muls(5, 3)
        (15, 0, 0)
        >>> muls(0xFFFF, 1)   # -1 × 1 = -1 = 0xFFFFFFFF
        (4294967295, 1, 0)
    """
    a = d_val & 0xFFFF
    b = src_val & 0xFFFF
    # Sign-extend to signed integers
    a_s = a if a < 0x8000 else a - 0x10000
    b_s = b if b < 0x8000 else b - 0x10000
    result32 = (a_s * b_s) & 0xFFFF_FFFF
    n = (result32 >> 31) & 1
    z = 1 if result32 == 0 else 0
    return result32, n, z


def mulu(d_val: int, src_val: int) -> tuple[int, int, int]:
    """Unsigned 16×16 → 32 multiply.  Returns (result32, n, z).

    MULU: Dn[31:0] = Dn[15:0] × src[15:0] (unsigned).
    V=0, C=0 always.  X unchanged.

    Args:
        d_val:   Data register value (only low 16 bits used).
        src_val: Source operand (16 bits, unsigned).

    Returns:
        (result32, n_flag, z_flag)

    Examples:
        >>> mulu(5, 3)
        (15, 0, 0)
        >>> mulu(0xFFFF, 0xFFFF)
        (4294836225, 1, 0)
    """
    a = d_val & 0xFFFF
    b = src_val & 0xFFFF
    result32 = (a * b) & 0xFFFF_FFFF
    n = (result32 >> 31) & 1
    z = 1 if result32 == 0 else 0
    return result32, n, z


def divs(d_val: int, src_val: int) -> tuple[int, int, bool]:
    """Signed 32÷16 division.  Returns (packed_result, overflow, div_by_zero).

    DIVS: Dn[31:16] = remainder (signed 16-bit),
          Dn[15:0]  = quotient (signed 16-bit).

    overflow=True if quotient doesn't fit in 16 bits (V flag set).

    Args:
        d_val:   Data register (32-bit dividend).
        src_val: Source operand (16-bit signed divisor).

    Returns:
        (packed_result, remainder, overflow_flag)
        packed_result = (remainder << 16) | (quotient & 0xFFFF)

    Raises:
        ZeroDivisionError: if src_val == 0 (caller should take exception).

    Examples:
        >>> divs(10, 3)
        (65542, False)
        >>> divs(0xFFFFFFFF, 1)   # -1 / 1 = -1, remainder 0
        (4294901759, False)
    """
    if src_val == 0:
        raise ZeroDivisionError("DIVS: division by zero")
    dividend = d_val & 0xFFFF_FFFF
    divisor = src_val & 0xFFFF
    # Sign-extend
    if dividend >= 0x8000_0000:
        dividend -= 0x1_0000_0000
    if divisor >= 0x8000:
        divisor -= 0x10000
    q = int(dividend / divisor)
    r = dividend - q * divisor
    # Overflow: quotient must fit in signed 16 bits [-32768, 32767]
    overflow = q < -32768 or q > 32767
    if overflow:
        return 0, False  # V flag set, registers unchanged
    q16 = q & 0xFFFF
    r16 = r & 0xFFFF
    packed = ((r16 << 16) | q16) & 0xFFFF_FFFF
    return packed, overflow


def divu(d_val: int, src_val: int) -> tuple[int, bool]:
    """Unsigned 32÷16 division.  Returns (packed_result, overflow_flag).

    DIVU: Dn[31:16] = remainder (unsigned 16-bit),
          Dn[15:0]  = quotient (unsigned 16-bit).

    Args:
        d_val:   Data register (32-bit unsigned dividend).
        src_val: Source operand (16-bit unsigned divisor).

    Returns:
        (packed_result, overflow_flag)

    Raises:
        ZeroDivisionError: if src_val == 0.

    Examples:
        >>> divu(10, 3)
        (65539, False)
        >>> divu(0xFFFFFFFF, 2)
        (32768, True)
    """
    if src_val == 0:
        raise ZeroDivisionError("DIVU: division by zero")
    dividend = d_val & 0xFFFF_FFFF
    divisor = src_val & 0xFFFF
    q = dividend // divisor
    r = dividend % divisor
    # Overflow: quotient must fit in unsigned 16 bits [0, 65535]
    overflow = q > 0xFFFF
    if overflow:
        return 0, True
    packed = ((r << 16) | q) & 0xFFFF_FFFF
    return packed, False
