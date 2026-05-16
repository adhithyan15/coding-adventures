"""alu.py — 64-bit gate-level ALU for the AArch64 (ARMv8-A) simulator.

Every data-path operation in this module routes through gate primitives:
  AND(a, b), OR(a, b), XOR(a, b), NOT(a)          — from logic_gates
  ripple_carry_adder(a_bits, b_bits, carry_in)      — from arithmetic (via bits.py)

No Python arithmetic operators (+, -, &, |, ^, ~, *, /) appear in the
execution path for ALU operations on register values.  Address arithmetic
and loop control (range(), index math) are bookkeeping, not data-path ops.

AArch64 architecture notes
──────────────────────────
The AArch64 ALU is a 64-bit unit with:
  - A 64-bit ripple-carry adder for ADD/SUB/compare
  - Bitwise logic units (AND/OR/XOR/NOT) — one gate per bit pair
  - A barrel shifter for LSL/LSR/ASR/ROR
  - A 64-cycle multiplier (shift-and-add) for MUL/MADD
  - A 64-iteration divider for UDIV/SDIV

NZCV flag conventions (AArch64/ARM)
────────────────────────────────────
  N = Negative: MSB of result (bit 63 for 64-bit, bit 31 for 32-bit)
  Z = Zero: NOR of all result bits
  C = Carry:
    For ADD: unsigned carry-out (overflow of unsigned addition)
    For SUB: carry_out of (A + NOT(B) + 1), which is 1 when A >= B (no borrow)
  V = Overflow: signed overflow (both operands same sign, result opposite sign)

Two's-complement subtraction
─────────────────────────────
SUB is implemented as:
  A - B = A + NOT(B) + 1   (two's complement identity)

The carry out from this operation represents the ARM carry convention:
  carry_out = 1 → no borrow (A >= B unsigned)
  carry_out = 0 → borrow occurred (A < B unsigned)

Logical flag computation (ANDS/TST/BICS)
─────────────────────────────────────────
For logical operations that set flags (ANDS, BICS, TST):
  N = MSB of result
  Z = (result == 0)
  C = 0 (always cleared)
  V = 0 (always cleared)

Gate counts per operation (approximate)
────────────────────────────────────────
  ADD64: 64 full adders = 384 gates (each FA = 2 XOR + 2 AND + 1 OR)
  SUB64: 64 NOT + 64 full adders ≈ 448 gates
  AND/OR/XOR: 64 gates
  CLZ64: up to 64 comparisons (sequential priority encoder)
  MUL64: 64 iterations × (shl + add_64bit) ≈ large
  DIV64: 64 iterations × (shl + sub_64bit) ≈ large
"""

from __future__ import annotations

from dataclasses import dataclass

from .bits import (
    add_32bit,
    add_64bit,
    and_32bit,
    and_64bit,
    bits_to_int,
    clz_32,
    clz_64,
    compute_zero,
    int_to_bits,
    mul_64,
    not_32bit,
    not_64bit,
    or_32bit,
    or_64bit,
    ror_32,
    ror_64,
    sdiv_64,
    shl_32,
    shl_64,
    shr_32_arith,
    shr_32_logical,
    shr_64_arith,
    shr_64_logical,
    smulh_64,
    sub_32bit,
    sub_64bit,
    udiv_64,
    umulh_64,
    xor_32bit,
    xor_64bit,
)

# Masks for bookkeeping
_MASK64: int = 0xFFFF_FFFF_FFFF_FFFF
_MASK32: int = 0xFFFF_FFFF

# ── ALU result type ────────────────────────────────────────────────────────────


@dataclass
class ALUResult64:
    """Result of a 64-bit ALU operation, including all NZCV flags.

    Fields
    ──────
    result   : the 64-bit result as a Python int (for convenience in the simulator)
    carry    : C flag (carry/borrow-complement from adder carry out)
    overflow : V flag (signed overflow)
    zero     : Z flag (1 if result == 0)
    negative : N flag (MSB of result; bit 63 for sf=1, bit 31 for sf=0)

    On AArch64, these flags are stored in PSTATE.NZCV and are only updated
    by S-suffix instructions (ADDS, SUBS, ANDS, BICS) and compare instructions
    (CMP, CMN, TST).
    """

    result: int      # unsigned integer result
    carry: int       # C flag
    overflow: int    # V flag
    zero: int        # Z flag
    negative: int    # N flag


def _alu64(
    result_bits: list[int],
    carry: int,
    overflow: int,
    sf: int = 1,
) -> ALUResult64:
    """Build an ALUResult64 from result bit list, carry, and overflow.

    Computes the zero and negative flags from the result bits using
    gate-level operations (compute_zero, bit extraction).

    Parameters
    ──────────
    result_bits : LSB-first bit list (64 elements for sf=1, 32 for sf=0)
    carry       : carry out of MSB position
    overflow    : signed overflow
    sf          : 1→64-bit, 0→32-bit (determines which bit is the sign bit)
    """
    zero = compute_zero(result_bits)
    # N = MSB of result: bit 63 for 64-bit, bit 31 for 32-bit
    msb_idx = 63 if sf else 31
    negative = result_bits[msb_idx]
    result_int = bits_to_int(result_bits)
    return ALUResult64(
        result=result_int,
        carry=carry,
        overflow=overflow,
        zero=zero,
        negative=negative,
    )


# ── 64-bit ADD / SUB ──────────────────────────────────────────────────────────


def add64(a: list[int], b: list[int], carry_in: int = 0) -> ALUResult64:
    """ADD: 64-bit add via ripple_carry_adder (gate-level).

    NZCV flags:
      N = sign bit (bit 63)
      Z = NOR of all 64 result bits
      C = carry out of bit 63 (unsigned overflow)
      V = XOR(carry_into_63, carry_out) — signed overflow

    Example
    ───────
    >>> a = int_to_bits(3, 64); b = int_to_bits(4, 64)
    >>> r = add64(a, b)
    >>> r.result
    7
    >>> r.carry
    0
    >>> # Unsigned overflow: MAX + 1 wraps
    >>> a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64); b = int_to_bits(1, 64)
    >>> r = add64(a, b)
    >>> r.carry
    1
    """
    sum_bits, carry_out, overflow = add_64bit(a, b, carry_in)
    return _alu64(sum_bits, carry_out, overflow, sf=1)


def sub64(a: list[int], b: list[int]) -> ALUResult64:
    """SUB: 64-bit subtract via two's complement (NOT(b) + 1).

    A - B = A + NOT(B) + 1

    ARM carry convention for subtraction:
      C=1 → no borrow (A >= B unsigned)
      C=0 → borrow occurred (A < B unsigned)

    Example
    ───────
    >>> a = int_to_bits(10, 64); b = int_to_bits(3, 64)
    >>> r = sub64(a, b)
    >>> r.result
    7
    >>> r.carry
    1
    >>> # 5 - 5 = 0
    >>> a = int_to_bits(5, 64); b = int_to_bits(5, 64)
    >>> r = sub64(a, b)
    >>> r.zero
    1
    """
    result_bits, carry_out, overflow = sub_64bit(a, b)
    return _alu64(result_bits, carry_out, overflow, sf=1)


def add32(a: list[int], b: list[int], carry_in: int = 0) -> ALUResult64:
    """ADD: 32-bit add for W-register operations.

    Returns ALUResult64 with the 32-bit result (zero-extended in result field)
    and flags derived from the 32-bit operation.

    Example
    ───────
    >>> a = int_to_bits(5, 32); b = int_to_bits(3, 32)
    >>> r = add32(a, b)
    >>> r.result
    8
    """
    sum_bits, carry_out, overflow = add_32bit(a, b, carry_in)
    return _alu64(sum_bits, carry_out, overflow, sf=0)


def sub32(a: list[int], b: list[int]) -> ALUResult64:
    """SUB: 32-bit subtract for W-register operations.

    Example
    ───────
    >>> a = int_to_bits(10, 32); b = int_to_bits(3, 32)
    >>> r = sub32(a, b)
    >>> r.result, r.carry
    (7, 1)
    """
    result_bits, carry_out, overflow = sub_32bit(a, b)
    return _alu64(result_bits, carry_out, overflow, sf=0)


# ── 64-bit logical operations (one gate per bit) ──────────────────────────────


def and64(a: list[int], b: list[int]) -> ALUResult64:
    """AND: 64 AND gates, one per bit pair.  No carry or overflow.

    Example
    ───────
    >>> a = int_to_bits(0b1010, 64); b = int_to_bits(0b1100, 64)
    >>> and64(a, b).result
    8
    """
    result_bits = and_64bit(a, b)
    return _alu64(result_bits, 0, 0, sf=1)


def or64(a: list[int], b: list[int]) -> ALUResult64:
    """OR: 64 OR gates, one per bit pair.  No carry or overflow.

    Example
    ───────
    >>> a = int_to_bits(0b1010, 64); b = int_to_bits(0b0101, 64)
    >>> or64(a, b).result
    15
    """
    result_bits = or_64bit(a, b)
    return _alu64(result_bits, 0, 0, sf=1)


def xor64(a: list[int], b: list[int]) -> ALUResult64:
    """XOR: 64 XOR gates, one per bit pair.  No carry or overflow.

    Example
    ───────
    >>> a = int_to_bits(0b1111, 64); b = int_to_bits(0b1010, 64)
    >>> xor64(a, b).result
    5
    """
    result_bits = xor_64bit(a, b)
    return _alu64(result_bits, 0, 0, sf=1)


def not64(a: list[int]) -> ALUResult64:
    """NOT: 64 NOT gates.  No carry or overflow.

    Example
    ───────
    >>> bits_to_int(not_64bit(int_to_bits(0, 64)))
    18446744073709551615
    """
    result_bits = not_64bit(a)
    return _alu64(result_bits, 0, 0, sf=1)


def and32(a: list[int], b: list[int]) -> ALUResult64:
    """AND: 32 AND gates for W-register operations."""
    result_bits = and_32bit(a, b)
    return _alu64(result_bits, 0, 0, sf=0)


def or32(a: list[int], b: list[int]) -> ALUResult64:
    """OR: 32 OR gates for W-register operations."""
    result_bits = or_32bit(a, b)
    return _alu64(result_bits, 0, 0, sf=0)


def xor32(a: list[int], b: list[int]) -> ALUResult64:
    """XOR: 32 XOR gates for W-register operations."""
    result_bits = xor_32bit(a, b)
    return _alu64(result_bits, 0, 0, sf=0)


def not32(a: list[int]) -> ALUResult64:
    """NOT: 32 NOT gates for W-register operations."""
    result_bits = not_32bit(a)
    return _alu64(result_bits, 0, 0, sf=0)


# ── Logical flags computation (for ANDS / TST / BICS) ─────────────────────────


def logical_flags_64(result_bits: list[int]) -> tuple[int, int, int, int]:
    """Compute NZCV flags for logical operations (ANDS/TST/BICS) — 64-bit.

    For logical operations: C=0, V=0 always.
    N = MSB of result (bit 63)
    Z = NOR of all bits

    Returns (N, Z, C, V) as individual 0/1 values.

    Example
    ───────
    >>> r = int_to_bits(0, 64)
    >>> logical_flags_64(r)
    (0, 1, 0, 0)
    >>> r = int_to_bits(0x8000000000000000, 64)
    >>> logical_flags_64(r)
    (1, 0, 0, 0)
    """
    n = result_bits[63]
    z = compute_zero(result_bits)
    return n, z, 0, 0


def logical_flags_32(result_bits: list[int]) -> tuple[int, int, int, int]:
    """Compute NZCV flags for logical operations (ANDS/TST/BICS) — 32-bit.

    Same as logical_flags_64 but N comes from bit 31.

    Returns (N, Z, C, V).
    """
    n = result_bits[31]
    z = compute_zero(result_bits)
    return n, z, 0, 0


def flags_to_nzcv(n: int, z: int, c: int, v: int) -> int:
    """Pack N, Z, C, V flags into a 4-bit NZCV nibble.

    Layout: N=bit3, Z=bit2, C=bit1, V=bit0

    Example
    ───────
    >>> flags_to_nzcv(1, 0, 1, 0)
    10
    """
    return (n << 3) | (z << 2) | (c << 1) | v


# ── Shift operations ──────────────────────────────────────────────────────────


def apply_shift(
    value_bits: list[int], shift_type: int, amount: int, sf: int
) -> list[int]:
    """Apply a shift or rotate operation to a bit list.

    shift_type: 0=LSL, 1=LSR (logical), 2=ASR (arithmetic), 3=ROR
    amount: shift amount
    sf: 1→64-bit, 0→32-bit

    This function dispatches to the appropriate shl/shr/ror function
    from bits.py (which operate on bit lists, not Python integers).

    Note: The amount is masked to the register width (modulo 64 or 32)
    per AArch64 spec for shifted-register operands.

    Example
    ───────
    >>> v = int_to_bits(8, 64)
    >>> bits_to_int(apply_shift(v, 1, 3, 1))  # LSR 3: 8 >> 3 = 1
    1
    """
    if sf:
        amount = amount & 63
        if shift_type == 0:   # LSL
            return shl_64(value_bits, amount)
        elif shift_type == 1:  # LSR
            return shr_64_logical(value_bits, amount)
        elif shift_type == 2:  # ASR
            return shr_64_arith(value_bits, amount)
        else:                  # ROR
            return ror_64(value_bits, amount)
    else:
        amount = amount & 31
        if shift_type == 0:   # LSL
            return shl_32(value_bits, amount)
        elif shift_type == 1:  # LSR
            return shr_32_logical(value_bits, amount)
        elif shift_type == 2:  # ASR
            return shr_32_arith(value_bits, amount)
        else:                  # ROR
            return ror_32(value_bits, amount)


# ── Count Leading Zeros ────────────────────────────────────────────────────────


def clz64(a_bits: list[int]) -> list[int]:
    """Count leading zeros in a 64-bit value.  Returns result as 64-bit bit list.

    Scans from MSB (bit 63) to LSB (bit 0) using gate-level AND checks.
    Returns int_to_bits(count, 64) so the result feeds back into the data path.

    CLZ is used in AArch64's CLZ instruction.

    Example
    ───────
    >>> bits_to_int(clz64(int_to_bits(0, 64)))
    64
    >>> bits_to_int(clz64(int_to_bits(1, 64)))
    63
    >>> bits_to_int(clz64(int_to_bits(0x8000000000000000, 64)))
    0
    """
    count = clz_64(a_bits)
    return int_to_bits(count, 64)


def clz32(a_bits: list[int]) -> list[int]:
    """Count leading zeros in a 32-bit value.  Returns result as 32-bit bit list.

    Example
    ───────
    >>> bits_to_int(clz32(int_to_bits(0, 32)))
    32
    >>> bits_to_int(clz32(int_to_bits(1, 32)))
    31
    """
    count = clz_32(a_bits)
    return int_to_bits(count, 32)


# ── Byte reversal for REV / REV32 / REV16 ────────────────────────────────────


def rev_bytes(bits_in: list[int], nbytes: int) -> list[int]:
    """Reverse the byte order of `nbytes` bytes within a 64-bit bit list.

    Used for REV (nbytes=8), REV32 (two 4-byte words reversed), REV16 (halfwords).

    The bit list is in LSB-first order.  Bytes within the list are:
      byte 0 = bits[7:0], byte 1 = bits[15:8], ..., byte 7 = bits[63:56]

    To reverse the byte order, we swap byte 0 with byte 7, byte 1 with byte 6, etc.

    Parameters
    ──────────
    bits_in : LSB-first 64-bit list
    nbytes  : number of bytes to reverse (must be power of 2, 1..8)

    Example
    ───────
    >>> v = int_to_bits(0x0102030405060708, 64)
    >>> bits_to_int(rev_bytes(v, 8))
    578437695752307201
    """
    result = bits_in[:]
    # Each byte occupies 8 bits in the LSB-first list
    # byte[i] occupies result[i*8 : i*8+8]
    # Reverse the order of the `nbytes` bytes
    for i in range(nbytes // 2):
        j = nbytes - 1 - i
        # Swap bytes i and j
        byte_i = result[i * 8 : i * 8 + 8]
        byte_j = result[j * 8 : j * 8 + 8]
        result[i * 8 : i * 8 + 8] = byte_j
        result[j * 8 : j * 8 + 8] = byte_i
    return result


def rev16_bytes(bits_in: list[int], width_bits: int) -> list[int]:
    """Byte-reverse within each 16-bit halfword of a 32- or 64-bit value.

    For REV16: swap the two bytes within each 16-bit halfword.
    width_bits: 32 or 64.

    Example
    ───────
    >>> v = int_to_bits(0x0102_0304, 64)
    >>> # REV16 on 32-bit: swap bytes within each halfword
    >>> hex(bits_to_int(rev16_bytes(v, 32)))
    '0x2010403'
    """
    result = bits_in[:]
    for hw_start in range(0, width_bits, 16):
        # Swap byte 0 and byte 1 within this halfword
        b0 = result[hw_start : hw_start + 8]
        b1 = result[hw_start + 8 : hw_start + 16]
        result[hw_start : hw_start + 8] = b1
        result[hw_start + 8 : hw_start + 16] = b0
    return result


def rev32_bytes(bits_in: list[int]) -> list[int]:
    """Byte-reverse within each 32-bit word of a 64-bit value (REV32 X).

    Swaps byte order within the low 32-bit word and within the high 32-bit word.

    Example
    ───────
    >>> v = int_to_bits(0x01020304_05060708, 64)
    >>> hex(bits_to_int(rev32_bytes(v)))
    '0x4030201080706050'
    """
    result = bits_in[:]
    # Reverse bytes within low word (bits 0..31)
    lo = rev_bytes(result[:32], 4)
    # Reverse bytes within high word (bits 32..63)
    hi = rev_bytes(result[32:], 4)
    return lo + hi


# ── Multiply operations ────────────────────────────────────────────────────────


def mul64(a: list[int], b: list[int]) -> list[int]:
    """64-bit × 64-bit unsigned multiply → low 64 bits.

    Used for MADD/MSUB/MUL instructions.

    Example
    ───────
    >>> a = int_to_bits(6, 64); b = int_to_bits(7, 64)
    >>> bits_to_int(mul64(a, b))
    42
    """
    return mul_64(a, b)


def umulh64(a: list[int], b: list[int]) -> list[int]:
    """64-bit × 64-bit unsigned multiply → upper 64 bits.

    Used for UMULH instruction.

    Example
    ───────
    >>> # 2^63 * 2 = 2^64 → upper 64 bits = 1
    >>> a = int_to_bits(0x8000000000000000, 64); b = int_to_bits(2, 64)
    >>> bits_to_int(umulh64(a, b))
    1
    """
    return umulh_64(a, b)


def smulh64(a: list[int], b: list[int]) -> list[int]:
    """64-bit × 64-bit signed multiply → upper 64 bits.

    Used for SMULH instruction.

    Example
    ───────
    >>> # -1 * 2 = -2; upper 64 bits of -2 as 128-bit = -1 = 0xFFFFFFFFFFFFFFFF
    >>> a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64); b = int_to_bits(2, 64)
    >>> bits_to_int(smulh64(a, b))
    18446744073709551615
    """
    return smulh_64(a, b)


# ── Divide operations ──────────────────────────────────────────────────────────


def udiv64(a: list[int], b: list[int]) -> list[int]:
    """64-bit unsigned division → quotient.

    Returns zero if divisor is zero (per AArch64 spec: UNDEFINED, return 0).

    Example
    ───────
    >>> a = int_to_bits(100, 64); b = int_to_bits(7, 64)
    >>> bits_to_int(udiv64(a, b))
    14
    """
    q, _ = udiv_64(a, b)
    return q


def sdiv64(a: list[int], b: list[int]) -> list[int]:
    """Signed 64-bit division → quotient (truncated toward zero).

    Returns zero if divisor is zero (per AArch64 spec: UNDEFINED, return 0).

    Example
    ───────
    >>> a = int_to_bits(-14 & 0xFFFFFFFFFFFFFFFF, 64); b = int_to_bits(3, 64)
    >>> import ctypes; ctypes.c_int64(bits_to_int(sdiv64(a, b))).value
    -4
    """
    q, _ = sdiv_64(a, b)
    return q
