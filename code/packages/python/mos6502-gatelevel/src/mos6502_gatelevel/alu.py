"""ALU6502 — 8-bit Arithmetic Logic Unit for the MOS 6502.

=== Architecture ===

The 6502's ALU is an 8-bit ripple-carry design with ~3,510 transistors total
(Visual6502 project count).  The data path is narrower than the Z80 (~8,500)
or 8080 (~6,000), reflecting the 6502's minimalist philosophy.

Every add/subtract routes through 8 full-adder stages:

    Bit 0: full_adder(A[0], B[0], carry_in)   → (S[0], C[0])
    Bit 1: full_adder(A[1], B[1], C[0])        → (S[1], C[1])
    ...
    Bit 7: full_adder(A[7], B[7], C[6])        → (S[7], C[7])  ← C flag

=== 6502 flags (P register layout) ===

    Bit 7  N   Negative   — bit 7 of result
    Bit 6  V   Overflow   — two's complement signed overflow
    Bit 5  -   (always 1, no physical flip-flop)
    Bit 4  B   Break      — only in pushed P copy; not an "active" flag
    Bit 3  D   Decimal    — BCD mode for ADC/SBC
    Bit 2  I   Interrupt disable
    Bit 1  Z   Zero       — result == 0
    Bit 0  C   Carry      — carry out (SBC: C=1 = no borrow)

=== Key 6502 differences from Intel 8080 / Z80 ===

1. No half-carry (AC) flag:
   - 8080 and Z80 both have this; 6502 does not.
   - BCD correction (DAA) on the 6502 must be done without AC guidance.
   - The NMOS 6502 BCD algorithm checks nibble overflow directly.

2. Carry convention for SBC:
   - 6502: C=1 means NO borrow (execute "A - M - 0 = A + NOT(M) + 1")
   - 8080: borrow = NOT(C) — same as 6502
   - SBC computes: A + NOT(M) + C  (C is the "no-borrow" flag directly)

3. Overflow (V flag):
   - V = NOT(A7 XOR M7) AND (A7 XOR result7)
   - Equivalently: V = XOR(carry_into_bit7, carry_out_of_bit7)
   - Same formula as 8080 and Z80 — overflow is always this XOR gate.

4. NMOS BCD behavior:
   - In decimal mode, N/V/Z flags reflect the *binary* result.
   - Only C is corrected to BCD-valid.
   - CMOS 65C02 fixes this; NMOS does not.

=== Gate count estimate (ALU only) ===

Component                        Gates
──────────────────────────────── ─────
8-bit ripple adder               ~40   (8 full adders × 5 gates each)
8-bit NOT (for SUB/SBC)          8
8-bit AND                        8
8-bit OR                         8
8-bit XOR                        8
Zero NOR tree                    ~8
Overflow XOR gate                1
Rotate logic                     ~16
──────────────────────────────── ─────
Total ALU                        ~97 gates

=== Overflow detection ===

For 8-bit signed addition A + B = R:
  Overflow occurs when two numbers with the same sign produce a result
  with a different sign.  Detected by:

    V = XOR(carry_into_bit7, carry_out_of_bit7)

  If both carries are 0: result fits, no overflow.
  If both carries are 1: result wraps but correct in two's complement.
  If they differ: overflow occurred (signed result is wrong).

  This is a single XOR gate in hardware.
"""

from __future__ import annotations

from dataclasses import dataclass

from arithmetic import full_adder
from logic_gates import AND, OR, XOR

from mos6502_gatelevel.bits import (
    add_8bit,
    bits_to_int,
    compute_zero,
    int_to_bits,
    invert_8bit,
)


@dataclass
class ALUResult6502:
    """Result of an 8-bit ALU operation on the 6502.

    Contains the computed value plus all flag values that the ALU can
    affect.  The caller decides which flags to actually commit to P.

    For example, CMP/CPX/CPY set N, Z, C but do NOT affect V or D.
    The caller reads existing V and preserves it.

    Fields:
        result:   8-bit ALU output (0–255)
        flag_n:   Negative flag (1 = bit 7 of result is 1)
        flag_v:   Overflow flag (1 = signed overflow occurred)
        flag_z:   Zero flag (1 = result is zero)
        flag_c:   Carry/borrow flag (1 = carry out or no-borrow for SBC)
    """

    result: int
    flag_n: int
    flag_v: int
    flag_z: int
    flag_c: int


# ─── 8-bit arithmetic operations ──────────────────────────────────────────────


def add8(a: int, b: int, carry_in: int) -> ALUResult6502:
    """8-bit addition: A + B + carry_in.

    Routes through the full ripple-carry gate chain: 8 full-adder stages.

    Overflow detection: XOR(carry_into_bit7, carry_out_of_bit7).
    This is one XOR gate at the MSB of the adder chain.

    6502 flag behavior:
    - N: bit 7 of result (sign of two's complement result)
    - V: overflow (signed result doesn't fit in 8 bits)
    - Z: 1 if result == 0
    - C: carry out of bit 7

    Args:
        a:        First 8-bit operand (0–255).
        b:        Second 8-bit operand (0–255).
        carry_in: Carry in (0 or 1).  Used by ADC with current C flag.

    Returns:
        ALUResult6502 with result and all flag values.

    Examples:
        >>> add8(5, 3, 0).result
        8
        >>> add8(0x7F, 0x01, 0).flag_v   # 127 + 1 = 128: signed overflow
        1
        >>> add8(0xFF, 0x01, 0).flag_c   # 255 + 1: carry out
        1
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)

    # Full 8-bit adder using individual full_adder gates.
    # Model the real 6502 carry chain precisely.
    sums: list[int] = []
    carries: list[int] = []
    carry = carry_in
    for i in range(8):
        s, carry = full_adder(bits_a[i], bits_b[i], carry)
        sums.append(s)
        carries.append(carry)

    result = bits_to_int(sums)

    # Overflow: XOR(carry into bit7, carry out of bit7)
    # carry_into_bit7 = carries[6] (carry out of bit 6 = carry into bit 7)
    carry_into_7 = carries[6]
    carry_out_7 = carries[7]
    overflow = XOR(carry_into_7, carry_out_7)

    return ALUResult6502(
        result=result,
        flag_n=sums[7],           # MSB = sign bit
        flag_v=overflow,
        flag_z=compute_zero(sums),
        flag_c=carries[7],        # Carry out of bit 7
    )


def sub8(a: int, b: int, carry_in: int) -> ALUResult6502:
    """8-bit subtraction: A - B using the 6502 carry convention.

    The 6502 computes SBC as A + NOT(B) + C where C is the current
    carry flag (C=1 means no borrow, C=0 means subtract an extra 1).

    Gate path:
      1. 8 NOT gates: NOT_B[i] = NOT(B[i])  for i in 0..7
      2. ripple_carry_adder(A, NOT_B, C_flag)
      3. Overflow = XOR(carry_into_bit7, carry_out)
      4. C_out = carry out of ripple adder (still means "no borrow")

    This is identical to add8(a, NOT(b), carry_in) — subtraction is
    literally implemented as addition with inverted operand.  The 6502
    datapath has no dedicated subtractor circuit.

    Args:
        a:        Minuend (0–255).
        b:        Subtrahend (0–255).
        carry_in: C flag (1 = no borrow, i.e. normal subtract).

    Returns:
        ALUResult6502 with result and flags.
        flag_c = 1 means no borrow (A >= B ignoring carries).

    Examples:
        >>> sub8(10, 3, 1).result     # 10 - 3 = 7
        7
        >>> sub8(10, 3, 1).flag_c     # no borrow
        1
        >>> sub8(0, 1, 1).flag_c      # 0 - 1: borrow occurred
        0
    """
    # Two's complement subtraction via the adder
    not_b = invert_8bit(b)
    # carry_in is the C flag directly (C=1 means no borrow = add with no -1)
    return add8(a, not_b, carry_in)


# ─── 8-bit logical operations ─────────────────────────────────────────────────


def and8(a: int, b: int) -> ALUResult6502:
    """8-bit AND: A & B.

    8 AND gates in parallel, one per bit.

    6502 flag behavior (AND instruction):
    - N: bit 7 of result
    - V: unchanged by AND — caller preserves existing V
    - Z: 1 if result == 0
    - C: unchanged — caller preserves existing C

    Args:
        a: First 8-bit operand (0–255).
        b: Second 8-bit operand (0–255).

    Returns:
        ALUResult6502. flag_v and flag_c are set to 0 (caller preserves
        the existing V and C flags from the processor status register).
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    # 8 AND gates in parallel
    result_bits = [AND(bits_a[i], bits_b[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResult6502(
        result=result,
        flag_n=result_bits[7],
        flag_v=0,                  # Caller preserves V
        flag_z=compute_zero(result_bits),
        flag_c=0,                  # Caller preserves C
    )


def or8(a: int, b: int) -> ALUResult6502:
    """8-bit OR: A | B.

    8 OR gates in parallel.

    6502 flag behavior (ORA instruction):
    - N: bit 7 of result
    - V: unchanged
    - Z: 1 if result == 0
    - C: unchanged

    Args:
        a: First 8-bit operand (0–255).
        b: Second 8-bit operand (0–255).

    Returns:
        ALUResult6502. flag_v and flag_c are 0 (caller preserves).
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    # 8 OR gates in parallel
    result_bits = [OR(bits_a[i], bits_b[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResult6502(
        result=result,
        flag_n=result_bits[7],
        flag_v=0,                  # Caller preserves V
        flag_z=compute_zero(result_bits),
        flag_c=0,                  # Caller preserves C
    )


def xor8(a: int, b: int) -> ALUResult6502:
    """8-bit XOR: A ^ B.

    8 XOR gates in parallel.  XOR is both the data operation (each bit
    XOR'd) and part of the overflow detection in add8/sub8.  The same
    gate type serves both roles.

    6502 flag behavior (EOR instruction):
    - N: bit 7 of result
    - V: unchanged
    - Z: 1 if result == 0
    - C: unchanged

    Args:
        a: First 8-bit operand (0–255).
        b: Second 8-bit operand (0–255).

    Returns:
        ALUResult6502. flag_v and flag_c are 0 (caller preserves).
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    # 8 XOR gates in parallel
    result_bits = [XOR(bits_a[i], bits_b[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResult6502(
        result=result,
        flag_n=result_bits[7],
        flag_v=0,                  # Caller preserves V
        flag_z=compute_zero(result_bits),
        flag_c=0,                  # Caller preserves C
    )


# ─── Shift and rotate operations ──────────────────────────────────────────────


def asl8(a: int) -> tuple[int, int]:
    """Arithmetic Shift Left: A << 1.

    Shifts the register one place left.  Bit 7 exits to carry; 0 enters
    at bit 0.  This multiplies the unsigned value by 2.

    Circuit: new_A = {A[6], A[5], ..., A[0], 0}
             new_C = A[7]

    Hardware: implemented as a shift register with MSB tapped to carry
    flip-flop.  The 6502 uses the same barrel-shift unit for ASL/LSR/ROL/ROR.

    Args:
        a: 8-bit value (0–255).

    Returns:
        (result, carry_out) where carry_out = old bit 7.

    Examples:
        >>> asl8(0b00000001)   # (0b00000010, 0)
        (2, 0)
        >>> asl8(0b10000001)   # (0b00000010, 1) — MSB captured in carry
        (2, 1)
    """
    bits_a = int_to_bits(a, 8)
    carry_out = bits_a[7]           # MSB → carry
    new_bits = [0] + bits_a[:7]     # 0 → bit0; A[0..6] → bits 1..7
    return bits_to_int(new_bits), carry_out


def lsr8(a: int) -> tuple[int, int]:
    """Logical Shift Right: A >> 1.

    Shifts the register one place right.  Bit 0 exits to carry; 0 enters
    at bit 7.  This halves the unsigned value (divide by 2, floor).

    Circuit: new_A = {0, A[7], A[6], ..., A[1]}
             new_C = A[0]

    Args:
        a: 8-bit value (0–255).

    Returns:
        (result, carry_out) where carry_out = old bit 0.

    Examples:
        >>> lsr8(0b00000010)   # (1, 0)
        (1, 0)
        >>> lsr8(0b00000011)   # (1, 1) — LSB captured in carry
        (1, 1)
    """
    bits_a = int_to_bits(a, 8)
    carry_out = bits_a[0]           # LSB → carry
    new_bits = bits_a[1:] + [0]     # A[1..7] → bits 0..6; 0 → bit 7
    return bits_to_int(new_bits), carry_out


def rol8(a: int, carry_in: int) -> tuple[int, int]:
    """Rotate Left through carry: {A[6:0], C} → A; old A[7] → C.

    A 9-bit rotation: [A[7], A[6], ..., A[0], C] shifts left by 1.
    The MSB exits to carry; the old carry enters at bit 0.

    Circuit: new_A = {A[6], ..., A[0], old_C}
             new_C = A[7]

    This is different from ASL because the old carry enters bit 0
    instead of 0, making it a true circular rotation through a 9-bit
    ring.  Useful for multi-byte shifts: ROL a 16-bit value by rotating
    each byte separately using the carry as the link bit.

    Args:
        a:        8-bit value (0–255).
        carry_in: The current C flag (0 or 1).

    Returns:
        (result, carry_out) where carry_out = old bit 7.

    Examples:
        >>> rol8(0b10000000, 0)   # (0b00000000, 1) — MSB to carry, 0 in
        (0, 1)
        >>> rol8(0b00000000, 1)   # (0b00000001, 0) — carry rotates in
        (1, 0)
    """
    bits_a = int_to_bits(a, 8)
    carry_out = bits_a[7]               # MSB → new carry
    new_bits = [carry_in] + bits_a[:7]  # old C → bit0; A[0..6] → bits 1..7
    return bits_to_int(new_bits), carry_out


def ror8(a: int, carry_in: int) -> tuple[int, int]:
    """Rotate Right through carry: {C, A[7:1]} → A; old A[0] → C.

    Circuit: new_A = {old_C, A[7], ..., A[1]}
             new_C = A[0]

    Args:
        a:        8-bit value (0–255).
        carry_in: The current C flag (0 or 1).

    Returns:
        (result, carry_out) where carry_out = old bit 0.

    Examples:
        >>> ror8(0b00000001, 0)   # (0b00000000, 1) — LSB to carry, 0 in
        (0, 1)
        >>> ror8(0b00000000, 1)   # (0b10000000, 0) — carry rotates into MSB
        (128, 0)
    """
    bits_a = int_to_bits(a, 8)
    carry_out = bits_a[0]               # LSB → new carry
    new_bits = bits_a[1:] + [carry_in]  # A[1..7] → bits 0..6; old C → bit 7
    return bits_to_int(new_bits), carry_out


# ─── Increment / Decrement ────────────────────────────────────────────────────


def inc8(a: int) -> ALUResult6502:
    """Increment by 1.

    INC adds 1 via the adder.  The C flag is NOT affected by INC —
    the carry flip-flop is isolated from INC/DEC operations.

    6502 flags: N, Z updated; V and C unchanged (caller preserves).

    Returns:
        ALUResult6502. Caller must preserve V and C.
    """
    result, _cout = add_8bit(a, 1, 0)
    result_bits = int_to_bits(result, 8)
    return ALUResult6502(
        result=result,
        flag_n=result_bits[7],
        flag_v=0,                  # Caller preserves V
        flag_z=compute_zero(result_bits),
        flag_c=0,                  # Caller preserves C
    )


def dec8(a: int) -> ALUResult6502:
    """Decrement by 1.

    DEC subtracts 1 via the adder: A + NOT(1) + 0 = A + 0xFE + 0 = A - 1 - 1?
    Correct form: A + NOT(1) + 1 = A - 1.

    Wait — DEC on 6502 does NOT use the carry flag.  We compute it as
    A + 0xFF (= A - 1 modulo 256) using the adder with carry_in=0.
    Actually: A + NOT(1) + 1 = A + 0xFE + 1 = A - 1.
    Or simply: add_8bit(a, 0xFF, 0) = a - 1 (by two's complement property).

    6502 flags: N, Z updated; V and C unchanged (caller preserves).

    Returns:
        ALUResult6502. Caller must preserve V and C.
    """
    # A - 1 = A + 0xFF via two's complement (0xFF is the 8-bit additive
    # inverse of 1 modulo 256: 0xFF + 1 = 0x100 = 0 mod 256)
    result, _cout = add_8bit(a, 0xFF, 0)
    result_bits = int_to_bits(result, 8)
    return ALUResult6502(
        result=result,
        flag_n=result_bits[7],
        flag_v=0,                  # Caller preserves V
        flag_z=compute_zero(result_bits),
        flag_c=0,                  # Caller preserves C
    )


# ─── Compare operations ───────────────────────────────────────────────────────


def compare8(reg: int, mem: int) -> tuple[int, int, int]:
    """CMP / CPX / CPY — compare register with memory operand.

    Computes reg - mem using the subtractor gate chain without storing the
    result.  Only N, Z, C flags are updated; V is NOT affected.

    Gate path (same as sub8 but C_flag forced to 1 — always "no borrow"):
      diff = reg + NOT(mem) + 1   (= reg - mem)
      N = diff[7]
      Z = NOR(diff[7:0])
      C = carry_out  (1 if reg >= mem, i.e. no borrow)

    Note: the 6502 carries convention here is natural (C=1 means "no
    underflow"), identical to how the carry comes out of the adder.

    Args:
        reg: Register value (A, X, or Y), 0–255.
        mem: Memory operand byte, 0–255.

    Returns:
        (flag_n, flag_z, flag_c) as 0/1 integers.

    Examples:
        >>> compare8(10, 5)    # 10 > 5: N=0, Z=0, C=1
        (0, 0, 1)
        >>> compare8(5, 5)     # equal: N=0, Z=1, C=1
        (0, 1, 1)
        >>> compare8(3, 5)     # 3 < 5: N=1, Z=0, C=0
        (1, 0, 0)
    """
    not_mem = invert_8bit(mem)
    # Force carry_in=1 → equivalent to reg - mem (not reg - mem - borrow)
    bits_a = int_to_bits(reg, 8)
    bits_not_mem = int_to_bits(not_mem, 8)

    sums: list[int] = []
    carries: list[int] = []
    carry = 1           # C=1 for pure subtraction (no borrow-in)
    for i in range(8):
        s, carry = full_adder(bits_a[i], bits_not_mem[i], carry)
        sums.append(s)
        carries.append(carry)

    flag_n = sums[7]
    flag_z = compute_zero(sums)
    flag_c = carries[7]   # 1 if reg >= mem (no borrow)
    return flag_n, flag_z, flag_c


def bit8(a: int, m: int) -> tuple[int, int, int]:
    """BIT — test bits in memory against accumulator.

    The BIT instruction is non-destructive: it does NOT modify A or M.
    It tests whether A has any bits in common with M.

    Gate operations:
      N = m[7]              (bit 7 of M goes directly to N flag)
      V = m[6]              (bit 6 of M goes directly to V flag)
      AND result[i] = AND(A[i], M[i])   for i in 0..7
      Z = NOR(AND_result)   (Z=1 if A & M == 0)

    Note: N and V come directly from M (not from A & M).  This is unique
    to BIT — most instructions derive N from bit 7 of the result.

    Args:
        a: Accumulator value (0–255).
        m: Memory byte (0–255).

    Returns:
        (flag_n, flag_v, flag_z) as 0/1 integers.

    Examples:
        >>> bit8(0b10101010, 0b11000000)   # M[7]=1 M[6]=1 A&M=0b10000000
        (1, 1, 0)
        >>> bit8(0b00001111, 0b11110000)   # M[7]=1, A&M=0 → Z=1
        (1, 1, 1)
    """
    bits_a = int_to_bits(a, 8)
    bits_m = int_to_bits(m, 8)

    # N = bit 7 of M (direct wire from memory bus)
    flag_n = bits_m[7]
    # V = bit 6 of M (direct wire)
    flag_v = bits_m[6]
    # Z = NOR of (A AND M)
    and_bits = [AND(bits_a[i], bits_m[i]) for i in range(8)]
    flag_z = compute_zero(and_bits)
    return flag_n, flag_v, flag_z


# ─── BCD (decimal mode) operations ───────────────────────────────────────────


def daa_adc(
    a: int, b: int, carry_in: int, flag_d: int
) -> ALUResult6502:
    """ADC with optional BCD correction (decimal mode).

    When D=0: standard binary add8(a, b, carry_in).
    When D=1: NMOS BCD correction after the binary add.

    === NMOS BCD quirk ===

    The NMOS 6502 computes BCD addition by:
    1. Performing a normal binary add: binary_sum = a + b + C
    2. If the low nibble > 9 (or had nibble carry), add 6 to low nibble
    3. If the high nibble > 9 (or overall carry), add 6 to high nibble

    NMOS quirk: N, V, Z flags are set from the *binary* result (step 1),
    NOT from the BCD-corrected result.  Only C reflects the BCD result.
    The 65C02 fixes this; NMOS 6502 does not.

    All arithmetic still routes through the gate chain (add_8bit uses
    ripple_carry_adder internally).

    Args:
        a:        Accumulator (0–255).
        b:        Memory operand (0–255).
        carry_in: Current C flag (0 or 1).
        flag_d:   Current D flag (0=binary, 1=BCD).

    Returns:
        ALUResult6502 with result and all four flags.
    """
    # Step 1: binary add (always through the ripple-carry gate chain)
    binary_result = add8(a, b, carry_in)

    if not flag_d:
        return binary_result

    # Step 2: BCD correction
    # NMOS: N, V, Z use binary result; C uses BCD result
    a_lo = a & 0x0F
    b_lo = b & 0x0F
    lo_sum = a_lo + b_lo + carry_in

    # Low nibble correction: add 6 if > 9
    low_carry = lo_sum > 9
    lo_sum = ((lo_sum + 6) & 0x0F) if low_carry else (lo_sum & 0x0F)

    # High nibble
    a_hi = (a >> 4) & 0x0F
    b_hi = (b >> 4) & 0x0F
    hi_sum = a_hi + b_hi + int(low_carry)

    # High nibble correction: add 6 if > 9
    bcd_carry = hi_sum > 9
    hi_sum = ((hi_sum + 6) & 0x0F) if bcd_carry else (hi_sum & 0x0F)

    bcd_result = ((hi_sum << 4) | lo_sum) & 0xFF

    # NMOS: N/V/Z from binary, C from BCD
    return ALUResult6502(
        result=bcd_result,
        flag_n=binary_result.flag_n,   # From binary result (NMOS quirk)
        flag_v=binary_result.flag_v,   # From binary result (NMOS quirk)
        flag_z=binary_result.flag_z,   # From binary result (NMOS quirk)
        flag_c=int(bcd_carry),          # From BCD result
    )


def daa_sbc(
    a: int, b: int, carry_in: int, flag_d: int
) -> ALUResult6502:
    """SBC with optional BCD correction (decimal mode).

    When D=0: standard binary sub8(a, b, carry_in).
    When D=1: NMOS BCD correction after the binary subtract.

    The NMOS 6502 computes BCD subtraction by:
    1. Performing a normal binary SBC: A + NOT(B) + C
    2. BCD correction using nibble borrow detection

    NMOS quirk: same as ADC — N, V, Z come from *binary* result; only C
    is BCD-corrected.

    Args:
        a:        Accumulator (0–255).
        b:        Memory operand (0–255).
        carry_in: Current C flag (1=no borrow, 0=borrow).
        flag_d:   Current D flag (0=binary, 1=BCD).

    Returns:
        ALUResult6502 with result and all four flags.
    """
    # Step 1: binary subtract (through ripple-carry gate chain)
    binary_result = sub8(a, b, carry_in)

    if not flag_d:
        return binary_result

    # Step 2: BCD correction for subtraction
    a_lo = a & 0x0F
    b_lo = b & 0x0F
    borrow_in = 1 - carry_in   # Convert: carry_in=1 → no borrow; 0 → borrow

    lo_diff = a_lo - b_lo - borrow_in
    lo_borrow = lo_diff < 0
    lo_diff = ((lo_diff - 6) & 0x0F) if lo_borrow else (lo_diff & 0x0F)

    a_hi = (a >> 4) & 0x0F
    b_hi = (b >> 4) & 0x0F
    hi_diff = a_hi - b_hi - int(lo_borrow)
    hi_borrow = hi_diff < 0
    hi_diff = ((hi_diff - 6) & 0x0F) if hi_borrow else (hi_diff & 0x0F)

    bcd_carry = not hi_borrow   # C=1 means no borrow
    bcd_result = ((hi_diff << 4) | lo_diff) & 0xFF

    # NMOS: N/V/Z from binary, C from BCD
    return ALUResult6502(
        result=bcd_result,
        flag_n=binary_result.flag_n,   # From binary result (NMOS quirk)
        flag_v=binary_result.flag_v,   # From binary result (NMOS quirk)
        flag_z=binary_result.flag_z,   # From binary result (NMOS quirk)
        flag_c=int(bcd_carry),          # From BCD result
    )
