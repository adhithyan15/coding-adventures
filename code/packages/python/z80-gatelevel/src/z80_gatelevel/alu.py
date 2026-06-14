"""ALUZ80 — 8-bit and 16-bit ALU for the Zilog Z80.

=== Architecture ===

The Z80's ALU is an 8-bit ripple-carry design, compatible with the Intel 8080
but with extra flag logic for the additional Z80 flags: H (half-carry) and N
(add/subtract indicator).

Every add/subtract routes through 8 full-adder stages:

    Bit 0: full_adder(A[0], B[0], carry_in)   → (S[0], C[0])
    Bit 1: full_adder(A[1], B[1], C[0])        → (S[1], C[1])
    ...
    Bit 3: full_adder(A[3], B[3], C[2])        → (S[3], C[3])  ← H carry
    ...
    Bit 7: full_adder(A[7], B[7], C[6])        → (S[7], C[7])  ← C flag

=== Z80 flags (F register layout) ===

    Bit 7  S   Sign       — bit 7 of result
    Bit 6  Z   Zero       — result == 0
    Bit 5  Y   (undocumented) — copy of result bit 5
    Bit 4  H   Half-carry — carry from bit 3 (ADD) / borrow (SUB)
    Bit 3  X   (undocumented) — copy of result bit 3
    Bit 2  P/V Parity (logical) / Overflow (arithmetic)
    Bit 1  N   Subtract   — 1 after SUB/SBC/DEC/CP/NEG, 0 after ADD/ADC/INC
    Bit 0  C   Carry

=== Key Z80 differences from Intel 8080 ===

1. Flag N (not in 8080):
   - Set to 1 for subtraction (SUB, SBC, DEC, CP, NEG)
   - Cleared to 0 for addition (ADD, ADC, INC, AND, OR, XOR)
   - Used by DAA to adjust correctly after both add and subtract

2. Flag P/V dual purpose (8080 had separate P and CY):
   - After AND/OR/XOR: P/V = even parity of result (XOR-tree)
   - After ADD/ADC/INC/SUB/SBC/DEC: P/V = signed overflow

3. Flag H (same concept as 8080 AC):
   - H = carry from bit 3 to bit 4 (addition)
   - H = borrow from bit 4 into bit 3 (subtraction)
   - For subtraction in hardware: H = NOT(carry_from_adder_bit3)
     because A - B = A + NOT(B) + 1, so the adder's bit-3 carry is inverted

4. 16-bit arithmetic (ED-prefixed):
   - ADD HL,rp: only updates H, N, C (not S, Z, P/V)
   - ADC HL,rp: updates all flags including S, Z, P/V
   - SBC HL,rp: updates all flags including S, Z, P/V

=== Gate count estimate ===

Component                        Gates
──────────────────────────────── ─────
8-bit ripple adder               ~40   (8 full adders × 5 gates each)
8-bit NOT (for SUB)              8
8-bit AND                        8
8-bit OR                         8
8-bit XOR                        8
Parity XOR tree                  8
Zero NOR tree                    ~8
Overflow XOR gate                1
Rotate logic                     ~16
16-bit adder (extra stages)      ~40
──────────────────────────────── ─────
Total ALU                        ~145 gates

=== Overflow detection ===

For 8-bit signed addition A + B = R:
  Overflow occurs when two numbers with the same sign produce a result
  with a different sign. This is detected by:

    overflow = XOR(carry_into_bit7, carry_out_of_bit7)

  If both carries are 0: result fits, no overflow.
  If both carries are 1: result wraps but is still correct in two's complement.
  If they differ: overflow occurred (signed result is wrong).

  This is a single XOR gate in hardware.

=== Subtraction half-carry ===

The Z80 computes A - B as A + NOT(B) + 1. When the adder produces a
carry out of bit 3, that means the LOWER nibble did NOT borrow, so:
    H_sub = NOT(adder_half_carry)

This is the standard behavior for all Z80 subtract/compare operations.
"""

from __future__ import annotations

from dataclasses import dataclass

from arithmetic import full_adder
from logic_gates import AND, NOT, OR, XOR

from z80_gatelevel.bits import (
    add_8bit,
    add_16bit,
    bits_to_int,
    compute_parity,
    compute_zero,
    int_to_bits,
    invert_8bit,
    invert_16bit,
)


@dataclass
class ALUResultZ80:
    """Result of an 8-bit or 16-bit ALU operation on the Z80.

    Contains the computed value plus all six Z80 flags that the ALU can
    affect. The caller decides which flags to actually commit to the flag
    register (e.g., INC/DEC preserve C; ADD HL,rp does not update S/Z/PV).

    Fields:
        result:   8-bit or 16-bit ALU output
        flag_s:   Sign flag (1 = negative)
        flag_z:   Zero flag (1 = zero)
        flag_h:   Half-carry flag
        flag_pv:  Parity/Overflow flag (parity for logical, overflow for arith)
        flag_n:   Subtract flag (1 = subtraction, 0 = addition/logical)
        flag_c:   Carry/borrow flag
    """

    result: int
    flag_s: int
    flag_z: int
    flag_h: int
    flag_pv: int
    flag_n: int
    flag_c: int


# ─── 8-bit operations ────────────────────────────────────────────────────────


def add8(a: int, b: int, carry_in: int = 0) -> ALUResultZ80:
    """8-bit addition: A + B + carry_in.

    Routes through the full ripple-carry gate chain: 8 full-adder stages.

    Overflow detection: XOR(carry_into_bit7, carry_out_of_bit7).
    This is one XOR gate at the MSB of the adder chain.

    Z80 flag behavior:
    - S: bit 7 of result (sign of two's complement result)
    - Z: 1 if result == 0
    - H: carry from bit 3 (= adder half-carry output)
    - P/V: overflow (signed result doesn't fit in 8 bits)
    - N: 0 (this is an addition)
    - C: carry out of bit 7

    Args:
        a:        First 8-bit operand (0–255).
        b:        Second 8-bit operand (0–255).
        carry_in: Carry in (0 or 1, default 0). Used by ADC instruction.

    Returns:
        ALUResultZ80 with result and all flag values.

    Examples:
        >>> add8(5, 3)
        ALUResultZ80(result=8, flag_s=0, flag_z=0, flag_h=0, flag_pv=0,
            flag_n=0, flag_c=0)
        >>> add8(0x7F, 0x01)  # 127 + 1 = 128 = -128 in signed: OVERFLOW
        ALUResultZ80(result=128, ..., flag_pv=1, ...)
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)

    # Full 8-bit adder using individual full_adder gates
    # This models the real Z80 carry chain precisely
    sums: list[int] = []
    carries: list[int] = []
    carry = carry_in
    for i in range(8):
        s, carry = full_adder(bits_a[i], bits_b[i], carry)
        sums.append(s)
        carries.append(carry)

    result = bits_to_int(sums)

    # Half-carry: carry out of bit 3
    hc = carries[3]

    # Overflow: XOR(carry into bit7, carry out of bit7)
    # carry_into_bit7 = carries[6] (carry out of bit 6 = carry into bit 7)
    carry_into_7 = carries[6]
    carry_out_7 = carries[7]
    overflow = XOR(carry_into_7, carry_out_7)

    return ALUResultZ80(
        result=result,
        flag_s=sums[7],          # MSB of result = sign bit
        flag_z=compute_zero(sums),
        flag_h=hc,
        flag_pv=overflow,
        flag_n=0,                # Addition: N = 0
        flag_c=carries[7],       # Carry out of bit 7
    )


def sub8(a: int, b: int, borrow_in: int = 0) -> ALUResultZ80:
    """8-bit subtraction: A - B - borrow_in.

    Implemented as A + NOT(B) + NOT(borrow_in) via two's complement:
        A - B     = A + NOT(B) + 1          (borrow_in = 0)
        A - B - 1 = A + NOT(B) + 0          (borrow_in = 1)

    Z80 flag behavior:
    - S: bit 7 of result
    - Z: 1 if result == 0
    - H: borrow from bit 4 into bit 3 = NOT(adder_half_carry)
    - P/V: signed overflow for subtraction
    - N: 1 (this is a subtraction)
    - C: 1 if borrow occurred (= NOT(adder_carry_out))

    The H flag inversion: since we implement A - B as A + NOT(B) + 1,
    the adder's half-carry is for the addition of NOT(B). When the adder
    produces carry_3=1, it means the low nibble did NOT borrow, so
    H_sub = NOT(adder_half_carry_from_addition).

    Args:
        a:          Minuend (0–255).
        b:          Subtrahend (0–255).
        borrow_in:  Incoming borrow (0 or 1). Used by SBC instruction.

    Returns:
        ALUResultZ80 with result and flags set for subtraction.
    """
    # Two's complement subtraction via the adder
    not_b = invert_8bit(b)
    # carry_in to adder = NOT(borrow_in): borrow_in=0 → cin=1 (two's complement)
    cin = NOT(borrow_in)

    bits_a = int_to_bits(a, 8)
    bits_not_b = int_to_bits(not_b, 8)

    sums: list[int] = []
    carries: list[int] = []
    carry = cin
    for i in range(8):
        s, carry = full_adder(bits_a[i], bits_not_b[i], carry)
        sums.append(s)
        carries.append(carry)

    result = bits_to_int(sums)

    # Overflow for subtraction: XOR(carry into bit7, carry out of bit7)
    # (same XOR-gate trick as addition, but in the two's-complement adder)
    carry_into_7 = carries[6]
    carry_out_7 = carries[7]
    overflow = XOR(carry_into_7, carry_out_7)

    # For subtraction: C = 1 means "borrow" = NOT(adder_carry_out)
    # For subtraction: H = NOT(adder_half_carry) = borrow from bit 4 into 3
    return ALUResultZ80(
        result=result,
        flag_s=sums[7],
        flag_z=compute_zero(sums),
        flag_h=NOT(carries[3]),    # Invert: adder carry_3=1 means NO borrow
        flag_pv=overflow,
        flag_n=1,                  # Subtraction: N = 1
        flag_c=NOT(carries[7]),    # Invert: adder carry_out=1 means NO borrow
    )


def and8(a: int, b: int) -> ALUResultZ80:
    """8-bit AND: A & B.

    8 AND gates in parallel, one per bit. The Z80 manual specifies:
    - H always set to 1 (distinguishes AND from OR/XOR in flag tracing)
    - N cleared to 0
    - C cleared to 0
    - P/V = parity of result (even parity → P/V=1)

    The H=1 rule was introduced in Z80 (the 8080's AND always set AC to
    OR of bit-3 of the two operands; Z80 simplifies this to just H=1).

    Args:
        a: First 8-bit operand (0–255).
        b: Second 8-bit operand (0–255).

    Returns:
        ALUResultZ80 with AND result.
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    # 8 AND gates in parallel
    result_bits = [AND(bits_a[i], bits_b[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResultZ80(
        result=result,
        flag_s=result_bits[7],
        flag_z=compute_zero(result_bits),
        flag_h=1,                       # H always 1 for AND
        flag_pv=compute_parity(result_bits),
        flag_n=0,
        flag_c=0,
    )


def or8(a: int, b: int) -> ALUResultZ80:
    """8-bit OR: A | B.

    8 OR gates in parallel. Z80 manual: H=0, N=0, C=0, P/V=parity.

    Args:
        a: First 8-bit operand (0–255).
        b: Second 8-bit operand (0–255).

    Returns:
        ALUResultZ80 with OR result.
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    result_bits = [OR(bits_a[i], bits_b[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResultZ80(
        result=result,
        flag_s=result_bits[7],
        flag_z=compute_zero(result_bits),
        flag_h=0,
        flag_pv=compute_parity(result_bits),
        flag_n=0,
        flag_c=0,
    )


def xor8(a: int, b: int) -> ALUResultZ80:
    """8-bit XOR: A ^ B.

    8 XOR gates in parallel. Z80 manual: H=0, N=0, C=0, P/V=parity.

    XOR is both the data operation (each bit XOR'd) and part of the parity
    computation (the parity tree is itself an XOR tree). Elegant reuse of
    the same gate type.

    Args:
        a: First 8-bit operand (0–255).
        b: Second 8-bit operand (0–255).

    Returns:
        ALUResultZ80 with XOR result.
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    result_bits = [XOR(bits_a[i], bits_b[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResultZ80(
        result=result,
        flag_s=result_bits[7],
        flag_z=compute_zero(result_bits),
        flag_h=0,
        flag_pv=compute_parity(result_bits),
        flag_n=0,
        flag_c=0,
    )


def inc8(a: int) -> ALUResultZ80:
    """Increment A by 1 (INC instruction).

    INC adds 1 via the adder. Importantly, the C flag is NOT affected by INC.
    This models the real Z80 hardware: the carry flip-flop is isolated from
    the INC/DEC operations.

    Z80 manual flags: S, Z, H, P/V set; N=0; C unchanged (caller preserves it).
    Overflow: A == 0x7F → adding 1 gives 0x80 (flips sign bit) → overflow.

    Returns:
        ALUResultZ80. Caller must preserve C flag (not apply flag_c here).
    """
    res = add8(a, 1, 0)
    # N = 0 for increment (addition)
    return ALUResultZ80(
        result=res.result,
        flag_s=res.flag_s,
        flag_z=res.flag_z,
        flag_h=res.flag_h,
        flag_pv=res.flag_pv,
        flag_n=0,
        flag_c=res.flag_c,  # Not used by caller — C preserved
    )


def dec8(a: int) -> ALUResultZ80:
    """Decrement A by 1 (DEC instruction).

    DEC subtracts 1. Like INC, C flag is NOT affected.

    Z80 manual flags: S, Z, H, P/V set; N=1; C unchanged.
    Overflow: A == 0x80 → subtracting 1 gives 0x7F (flips sign bit) → overflow.

    Returns:
        ALUResultZ80. Caller must preserve C flag.
    """
    res = sub8(a, 1, 0)
    # N = 1 for decrement (subtraction)
    return ALUResultZ80(
        result=res.result,
        flag_s=res.flag_s,
        flag_z=res.flag_z,
        flag_h=res.flag_h,
        flag_pv=res.flag_pv,
        flag_n=1,
        flag_c=res.flag_c,  # Not used by caller — C preserved
    )


def neg8(a: int) -> ALUResultZ80:
    """Negate accumulator (NEG instruction): A = 0 - A = NOT(A) + 1.

    NEG is equivalent to sub8(0, a, 0). The zero operand is the minuend;
    A is the subtrahend. Only A=0x80 overflows (0x80 two's complement is
    still 0x80 = -128, there is no +128 in signed 8-bit).

    Z80 manual: C=1 unless A=0 (borrow always except when negating 0).

    Returns:
        ALUResultZ80 with negated result and full flag set.
    """
    return sub8(0, a, 0)


def cpl8(a: int) -> ALUResultZ80:
    """Complement accumulator (CPL instruction): A = NOT(A).

    8 NOT gates in parallel. H and N are set (distinguishes CPL from other
    operations in BCD-aware code). S, Z, P/V, C are NOT affected.

    Z80 manual: "H and N flags are set; other flags are unaffected."
    This is unusual — most instructions either set or clear all flags.
    The caller must read back existing S, Z, P/V, C and keep them.

    Returns:
        ALUResultZ80 with complemented result. Only flag_h and flag_n are valid.
        Caller must preserve S, Z, P/V, C.
    """
    bits_a = int_to_bits(a, 8)
    result_bits = [NOT(b) for b in bits_a]
    result = bits_to_int(result_bits)
    # Only H=1, N=1 are set; other flags not affected (caller preserves)
    return ALUResultZ80(
        result=result,
        flag_s=0,   # caller preserves
        flag_z=0,   # caller preserves
        flag_h=1,   # H set by CPL
        flag_pv=0,  # caller preserves
        flag_n=1,   # N set by CPL
        flag_c=0,   # caller preserves
    )


def daa8(a: int, flag_n: int, flag_h: int, flag_c: int) -> ALUResultZ80:
    """Decimal Adjust Accumulator (DAA instruction).

    BCD (Binary Coded Decimal) stores each decimal digit in 4 bits.
    After a binary addition or subtraction of two BCD numbers, the result
    may not be valid BCD. DAA corrects it.

    === Z80 DAA differs from 8080 DAA ===

    The Z80's DAA must handle BOTH addition and subtraction (the N flag
    indicates which). The 8080 only supports DAA after addition.

    After ADDITION (N=0):
        if (A & 0x0F) > 9 or H == 1:  add 0x06
        if A > 0x99 or C == 1:        add 0x60, set C

    After SUBTRACTION (N=1):
        if H == 1:       subtract 0x06
        if C == 1:       subtract 0x60

    The correction values route through the same ripple-carry adder as
    the main ALU, re-using the gate chain.

    Args:
        a:      8-bit accumulator value after the last operation (0–255).
        flag_n: Current N flag (0 = after ADD, 1 = after SUB).
        flag_h: Current H flag.
        flag_c: Current C flag.

    Returns:
        ALUResultZ80 with corrected BCD result and updated flags.
    """
    correction = 0
    new_c = flag_c

    if flag_n == 0:  # After addition
        if (a & 0x0F) > 9 or flag_h:
            correction |= 0x06
        # Check the corrected value for high nibble overflow
        temp = (a + correction) & 0xFF
        if temp > 0x99 or flag_c:
            correction |= 0x60
            new_c = 1

        result, _, hc = add_8bit(a, correction, 0)
        # H = 1 if low nibble corrected and overflowed
        new_h = hc
    else:  # After subtraction
        if flag_h:
            correction |= 0x06
        if flag_c:
            correction |= 0x60
            new_c = 1

        # Subtract the correction: A - correction = A + NOT(correction) + 1
        if correction:
            result, _, hc_raw = add_8bit(a, invert_8bit(correction), 1)
            new_h = NOT(hc_raw)
        else:
            result = a
            new_h = 0

    result_bits = int_to_bits(result, 8)
    return ALUResultZ80(
        result=result,
        flag_s=result_bits[7],
        flag_z=compute_zero(result_bits),
        flag_h=new_h,
        flag_pv=compute_parity(result_bits),
        flag_n=flag_n,     # N is preserved from the previous operation
        flag_c=new_c,
    )


# ─── Rotate / Shift operations ────────────────────────────────────────────────


def rlc8(a: int) -> ALUResultZ80:
    """Rotate Left Circular (RLC): bit 7 → CY, bit 7 → bit 0.

    The MSB wraps around to the LSB AND is also captured in C.
    This is the Z80's CB-prefixed RLC operation (not RLCA).

    CB-prefixed RLC differs from RLCA:
    - RLC: sets S, Z, P/V flags based on result
    - RLCA: does NOT set S, Z, P/V (only C, H=0, N=0)

    Hardware: shift register with MSB feedback to input.

    Circuit: new_A = {A[6], A[5], ..., A[0], A[7]}
             new_C = A[7]
    """
    bits_a = int_to_bits(a, 8)
    msb = bits_a[7]
    new_bits = [msb] + bits_a[:7]   # [old_b7, old_b0, ..., old_b6]
    result = bits_to_int(new_bits)
    return ALUResultZ80(
        result=result,
        flag_s=new_bits[7],
        flag_z=compute_zero(new_bits),
        flag_h=0,
        flag_pv=compute_parity(new_bits),
        flag_n=0,
        flag_c=msb,
    )


def rrc8(a: int) -> ALUResultZ80:
    """Rotate Right Circular (RRC): bit 0 → CY, bit 0 → bit 7.

    The LSB wraps around to the MSB AND is captured in C.

    Circuit: new_A = {A[0], A[7], A[6], ..., A[1]}
             new_C = A[0]
    """
    bits_a = int_to_bits(a, 8)
    lsb = bits_a[0]
    new_bits = bits_a[1:] + [lsb]   # [old_b1, ..., old_b7, old_b0]
    result = bits_to_int(new_bits)
    return ALUResultZ80(
        result=result,
        flag_s=new_bits[7],
        flag_z=compute_zero(new_bits),
        flag_h=0,
        flag_pv=compute_parity(new_bits),
        flag_n=0,
        flag_c=lsb,
    )


def rl8(a: int, carry_in: int) -> ALUResultZ80:
    """Rotate Left through carry (RL): A7 → CY, old_CY → A0.

    A 9-bit rotation: [CY, A7, A6, ..., A0] shifts left by 1.
    The MSB exits to CY; the old CY enters at bit 0.

    Circuit: new_A = {A[6], ..., A[0], old_CY}
             new_C = A[7]
    """
    bits_a = int_to_bits(a, 8)
    msb = bits_a[7]
    new_bits = [carry_in] + bits_a[:7]   # old_CY → bit0; A0..A6 → bits 1..7
    result = bits_to_int(new_bits)
    return ALUResultZ80(
        result=result,
        flag_s=new_bits[7],
        flag_z=compute_zero(new_bits),
        flag_h=0,
        flag_pv=compute_parity(new_bits),
        flag_n=0,
        flag_c=msb,
    )


def rr8(a: int, carry_in: int) -> ALUResultZ80:
    """Rotate Right through carry (RR): A0 → CY, old_CY → A7.

    Circuit: new_A = {old_CY, A[7], ..., A[1]}
             new_C = A[0]
    """
    bits_a = int_to_bits(a, 8)
    lsb = bits_a[0]
    new_bits = bits_a[1:] + [carry_in]   # A1..A7 → bits 0..6; old_CY → bit7
    result = bits_to_int(new_bits)
    return ALUResultZ80(
        result=result,
        flag_s=new_bits[7],
        flag_z=compute_zero(new_bits),
        flag_h=0,
        flag_pv=compute_parity(new_bits),
        flag_n=0,
        flag_c=lsb,
    )


def sla8(a: int) -> ALUResultZ80:
    """Shift Left Arithmetic (SLA): A7 → CY, 0 → A0.

    Multiply by 2 (signed). Bit 7 exits to carry; 0 enters at bit 0.

    Circuit: new_A = {A[6], A[5], ..., A[0], 0}
             new_C = A[7]
    """
    bits_a = int_to_bits(a, 8)
    msb = bits_a[7]
    new_bits = [0] + bits_a[:7]   # 0 → bit0; A0..A6 → bits 1..7
    result = bits_to_int(new_bits)
    return ALUResultZ80(
        result=result,
        flag_s=new_bits[7],
        flag_z=compute_zero(new_bits),
        flag_h=0,
        flag_pv=compute_parity(new_bits),
        flag_n=0,
        flag_c=msb,
    )


def sra8(a: int) -> ALUResultZ80:
    """Shift Right Arithmetic (SRA): A0 → CY, A7 preserved (sign extension).

    Divide by 2 (signed). Bit 0 exits to carry; bit 7 is replicated.
    This is "arithmetic" because the sign bit is preserved, making it
    a proper signed right-shift (division by 2 with rounding toward -∞).

    Circuit: new_A = {A[7], A[7], A[6], ..., A[1]}
             new_C = A[0]
    """
    bits_a = int_to_bits(a, 8)
    lsb = bits_a[0]
    msb = bits_a[7]   # preserved sign bit
    new_bits = bits_a[1:] + [msb]   # A1..A7 → bits 0..6; A7 stays at bit 7
    result = bits_to_int(new_bits)
    return ALUResultZ80(
        result=result,
        flag_s=new_bits[7],
        flag_z=compute_zero(new_bits),
        flag_h=0,
        flag_pv=compute_parity(new_bits),
        flag_n=0,
        flag_c=lsb,
    )


def srl8(a: int) -> ALUResultZ80:
    """Shift Right Logical (SRL): A0 → CY, 0 → A7.

    Unsigned right-shift. Bit 0 exits to carry; 0 enters at bit 7.
    Halves the unsigned value (divide by 2, rounding down).

    Circuit: new_A = {0, A[7], A[6], ..., A[1]}
             new_C = A[0]
    """
    bits_a = int_to_bits(a, 8)
    lsb = bits_a[0]
    new_bits = bits_a[1:] + [0]   # A1..A7 → bits 0..6; 0 → bit 7
    result = bits_to_int(new_bits)
    return ALUResultZ80(
        result=result,
        flag_s=new_bits[7],   # Always 0 (SRL clears bit 7)
        flag_z=compute_zero(new_bits),
        flag_h=0,
        flag_pv=compute_parity(new_bits),
        flag_n=0,
        flag_c=lsb,
    )


def bit_test(a: int, bit_n: int) -> ALUResultZ80:
    """BIT b, r — test bit n of A via AND gate.

    The BIT instruction is read-only: it tests a single bit without
    modifying the register. Z uses the AND of (A & (1<<n)):
        Z = NOT(A[n])   (Z=1 means bit is 0)

    Z80 manual flags:
    - Z = NOT(tested_bit)  (Z=1 if bit is 0)
    - H = 1
    - N = 0
    - S, P/V: set based on result (Z80 puts tested bit in S for bit 7)
    - C: unchanged (caller preserves)

    The AND gate: result_bit = AND(A[bit_n], 1)  → just reads A[bit_n].

    Args:
        a:     8-bit value to test.
        bit_n: Bit position to test (0–7).

    Returns:
        ALUResultZ80. result is 0 (BIT does not update the register).
        Only Z, H, N are valid; caller preserves S, PV, C.
    """
    bits_a = int_to_bits(a, 8)
    # Test the bit via AND gate: AND(A[bit_n], 1) = A[bit_n]
    tested = AND(bits_a[bit_n], 1)
    z = NOT(tested)   # Z=1 if bit is 0

    return ALUResultZ80(
        result=0,            # BIT doesn't write the register
        flag_s=tested if bit_n == 7 else 0,  # S = tested bit (for bit 7)
        flag_z=z,
        flag_h=1,
        flag_pv=compute_parity(bits_a),  # P/V = parity of result byte
        flag_n=0,
        flag_c=0,            # C unchanged (caller preserves)
    )


def set_bit(a: int, bit_n: int) -> int:
    """SET b, r — set bit n of A using OR gate.

    OR(A[n], 1) = 1 always: the OR gate is a simple "force-to-1" for the
    selected bit. No flags are affected by SET.

    Args:
        a:     8-bit value.
        bit_n: Bit to set (0–7).

    Returns:
        8-bit result with bit n set.
    """
    bits_a = int_to_bits(a, 8)
    bits_a[bit_n] = OR(bits_a[bit_n], 1)
    return bits_to_int(bits_a)


def res_bit(a: int, bit_n: int) -> int:
    """RES b, r — reset bit n of A using AND gate.

    AND(A[n], 0) = 0 always: the AND gate is a simple "force-to-0" for
    the selected bit. No flags are affected by RES.

    Args:
        a:     8-bit value.
        bit_n: Bit to reset (0–7).

    Returns:
        8-bit result with bit n cleared.
    """
    bits_a = int_to_bits(a, 8)
    bits_a[bit_n] = AND(bits_a[bit_n], 0)
    return bits_to_int(bits_a)


# ─── Accumulator rotate variants (different flag behavior from CB rotates) ────


def rlca8(a: int) -> ALUResultZ80:
    """RLCA — Rotate Left Circular Accumulator (unprefixed 0x07).

    Same rotation as RLC but only C is updated (S, Z, P/V unchanged).
    Z80 manual: H=0, N=0; other flags unchanged.
    """
    bits_a = int_to_bits(a, 8)
    msb = bits_a[7]
    new_bits = [msb] + bits_a[:7]
    result = bits_to_int(new_bits)
    return ALUResultZ80(
        result=result,
        flag_s=0, flag_z=0, flag_h=0, flag_pv=0,
        flag_n=0, flag_c=msb,
    )


def rrca8(a: int) -> ALUResultZ80:
    """RRCA — Rotate Right Circular Accumulator (unprefixed 0x0F).

    Same rotation as RRC but only C is updated.
    """
    bits_a = int_to_bits(a, 8)
    lsb = bits_a[0]
    new_bits = bits_a[1:] + [lsb]
    result = bits_to_int(new_bits)
    return ALUResultZ80(
        result=result,
        flag_s=0, flag_z=0, flag_h=0, flag_pv=0,
        flag_n=0, flag_c=lsb,
    )


def rla8(a: int, carry_in: int) -> ALUResultZ80:
    """RLA — Rotate Left Accumulator through carry (unprefixed 0x17).

    Same rotation as RL but only C is updated (S, Z, P/V unchanged).
    """
    bits_a = int_to_bits(a, 8)
    msb = bits_a[7]
    new_bits = [carry_in] + bits_a[:7]
    result = bits_to_int(new_bits)
    return ALUResultZ80(
        result=result,
        flag_s=0, flag_z=0, flag_h=0, flag_pv=0,
        flag_n=0, flag_c=msb,
    )


def rra8(a: int, carry_in: int) -> ALUResultZ80:
    """RRA — Rotate Right Accumulator through carry (unprefixed 0x1F).

    Same rotation as RR but only C is updated.
    """
    bits_a = int_to_bits(a, 8)
    lsb = bits_a[0]
    new_bits = bits_a[1:] + [carry_in]
    result = bits_to_int(new_bits)
    return ALUResultZ80(
        result=result,
        flag_s=0, flag_z=0, flag_h=0, flag_pv=0,
        flag_n=0, flag_c=lsb,
    )


# ─── 16-bit operations ────────────────────────────────────────────────────────


def add16(hl: int, rp: int) -> ALUResultZ80:
    """ADD HL, rp — 16-bit addition: HL = HL + rp.

    This is the Z80 ADD HL,rp instruction (unprefixed 0x09/0x19/0x29/0x39).

    Z80 manual: only H, N, C are affected (S, Z, P/V UNCHANGED).
    - H: carry from bit 11 (the half-carry of the high byte)
    - N: 0 (addition)
    - C: carry from bit 15

    Args:
        hl: Current HL value (0–65535).
        rp: Register pair value (0–65535).

    Returns:
        ALUResultZ80 with 16-bit result. flag_s, flag_z, flag_pv not valid
        (caller preserves existing values).
    """
    result, cout, hc16 = add_16bit(hl, rp, 0)
    return ALUResultZ80(
        result=result,
        flag_s=0,     # Not changed by ADD HL,rp (caller preserves)
        flag_z=0,     # Not changed by ADD HL,rp (caller preserves)
        flag_h=hc16,  # Carry from bit 11
        flag_pv=0,    # Not changed by ADD HL,rp (caller preserves)
        flag_n=0,     # Addition
        flag_c=cout,
    )


def adc16(hl: int, rp: int, carry_in: int) -> ALUResultZ80:
    """ADC HL, rp — 16-bit add with carry: HL = HL + rp + C.

    This is the Z80 ED-prefix ADC HL,rp instruction (0xED 0x4A/0x5A/0x6A/0x7A).

    Z80 manual: ALL flags affected (unlike ADD HL which preserves S, Z, P/V).
    - S: bit 15 of result
    - Z: 1 if result == 0
    - H: carry from bit 11
    - P/V: signed 16-bit overflow
    - N: 0
    - C: carry from bit 15

    Args:
        hl:       HL value (0–65535).
        rp:       Register pair value (0–65535).
        carry_in: Carry flag (0 or 1).

    Returns:
        ALUResultZ80 with all valid flags.
    """
    result, cout, hc16 = add_16bit(hl, rp, carry_in)
    result_bits = int_to_bits(result, 16)

    # 16-bit overflow: XOR(carry into bit15, carry out of bit15)
    # Approximate via high-byte overflow: compare sign of hl, rp, and result
    hl_sign = (hl >> 15) & 1
    rp_sign = (rp >> 15) & 1
    res_sign = result_bits[15]
    # Overflow: same signs of inputs but different sign of result
    overflow = AND(
        NOT(XOR(hl_sign, rp_sign)),  # inputs have same sign
        XOR(hl_sign, res_sign),      # result has different sign
    )

    return ALUResultZ80(
        result=result,
        flag_s=result_bits[15],
        flag_z=compute_zero(result_bits),
        flag_h=hc16,
        flag_pv=overflow,
        flag_n=0,
        flag_c=cout,
    )


def sbc16(hl: int, rp: int, borrow_in: int) -> ALUResultZ80:
    """SBC HL, rp — 16-bit subtract with borrow: HL = HL - rp - C.

    This is the Z80 ED-prefix SBC HL,rp instruction (0xED 0x42/0x52/0x62/0x72).

    Z80 manual: ALL flags affected.
    Implemented as HL + NOT(rp) + NOT(borrow_in) via 16-bit ripple adder.

    Args:
        hl:        HL value (0–65535).
        rp:        Register pair value (0–65535).
        borrow_in: Borrow flag (current C flag, 0 or 1).

    Returns:
        ALUResultZ80 with all valid flags.
    """
    not_rp = invert_16bit(rp)
    cin = NOT(borrow_in)   # Two's complement: borrow_in=0 → cin=1

    result, cout, hc16 = add_16bit(hl, not_rp, cin)
    result_bits = int_to_bits(result, 16)

    # Overflow for subtraction
    hl_sign = (hl >> 15) & 1
    rp_sign = (rp >> 15) & 1
    res_sign = result_bits[15]
    # Subtraction overflow: opposite signs of inputs, result sign differs from hl
    overflow = AND(
        XOR(hl_sign, rp_sign),    # inputs have opposite sign
        XOR(hl_sign, res_sign),   # result differs from hl in sign
    )

    return ALUResultZ80(
        result=result,
        flag_s=result_bits[15],
        flag_z=compute_zero(result_bits),
        flag_h=NOT(hc16),         # Borrow from bit 12: invert like 8-bit sub
        flag_pv=overflow,
        flag_n=1,                 # Subtraction
        flag_c=NOT(cout),         # C = 1 means borrow: invert adder carry
    )
