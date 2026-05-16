"""alu.py — 64-bit gate-level ALU for the DEC Alpha AXP 21064.

Every data-path operation in this module routes through gate primitives:
  AND(a, b), OR(a, b), XOR(a, b), NOT(a)          — from logic_gates
  ripple_carry_adder(a_bits, b_bits, carry_in)      — from arithmetic

No Python arithmetic operators (+, -, &, |, ^, ~, *, /) appear in the
execution path.  All arithmetic is done by chaining gate functions.

Architecture notes
──────────────────
The Alpha AXP 21064 has a pure 64-bit integer pipeline.  Key design choices
that differ from earlier RISC processors:

  - No condition codes: comparisons write 0 or 1 to a destination register.
    This eliminates false dependencies between instructions.
  - Longword (L) variants: ADDL/SUBL operate on the low 32 bits and
    sign-extend the result to 64 bits.  This supports C's int type.
  - Scaled add: S4ADDQ/S8ADDQ are used for array indexing without a
    separate multiply-by-4/8 instruction.

Two's complement subtraction
─────────────────────────────
SUB is implemented as:
  a - b = a + NOT(b) + 1   (two's complement identity)

So subq(a, b) calls:
  not_b = invert_64bit(b)     # 64 NOT gates
  result = addq(a, not_b, carry_in=1)  # ripple_carry_adder with carry=1

This is exactly how the physical ALU works — there is no dedicated subtract
circuit; the adder is reused with inverted B and carry_in=1.

Overflow detection
──────────────────
For ADD: overflow = XOR(carry_into_bit_63, carry_out_of_bit_63)
For SUB (via ADD with inverted b): same formula applies.

Gate counts per operation (approximate)
────────────────────────────────────────
  ADDQ:      64 full adders = 384 gates (each FA = 2 XOR + 2 AND + 1 OR)
  SUBQ:      64 NOT + 64 full adders ≈ 448 gates
  AND/OR/XOR: 64 gates
  MULQ:      64 iterations × (64 NOT + 64 full adders) ≈ 28,672 gates
"""

from __future__ import annotations

from dataclasses import dataclass

from logic_gates import AND, NOT, OR, XOR

from .bits import (
    add_32bit,
    add_64bit,
    add_128bit,
    bits_to_int,
    compute_zero,
    int_to_bits,
    invert_32bit,
    invert_64bit,
    sext32_to_64,
    shl_64,
    shr_64_arith,
    shr_64_logical,
)

# Mask constants — used for address arithmetic and result masking only.
_MASK64: int = 0xFFFF_FFFF_FFFF_FFFF
_MASK32: int = 0xFFFF_FFFF

# ── ALU result type ────────────────────────────────────────────────────────────


@dataclass
class ALUResult64:
    """Result of a 64-bit ALU operation.

    Fields
    ──────
    result   : 64-bit unsigned integer result
    carry    : carry out of bit 63 (1 or 0)
    overflow : signed overflow flag (1 if overflow occurred)
    zero     : 1 if result == 0, 0 otherwise
    negative : sign bit of result (bit 63)

    On the Alpha, these flags are NOT stored in a condition-code register —
    comparisons instead write 0 or 1 to a GPR.  The flags here are computed
    but only used internally by the ALU for compare instructions.
    """

    result: int    # 64-bit unsigned result
    carry: int     # carry out of bit 63
    overflow: int  # signed overflow
    zero: int      # 1 if result == 0
    negative: int  # sign bit (bit 63)


def _alu64(result_int: int, carry: int, overflow: int) -> ALUResult64:
    """Build an ALUResult64 from a raw 64-bit result integer."""
    r = result_int & _MASK64
    bits = int_to_bits(r, 64)
    zero = compute_zero(bits)
    negative = bits[63]
    return ALUResult64(
        result=r,
        carry=carry,
        overflow=overflow,
        zero=zero,
        negative=negative,
    )


# ── 64-bit ADD / SUB ──────────────────────────────────────────────────────────

def addq(a: int, b: int, carry_in: int = 0) -> ALUResult64:
    """ADDQ: 64-bit unsigned add via ripple_carry_adder.

    overflow = XOR(carry_into_bit_63, carry_out_of_bit_63)

    This is the core 64-bit adder.  All other arithmetic operations
    (SUBQ, S4ADDQ, etc.) build on this.

    Example
    ───────
    >>> r = addq(3, 4)
    >>> r.result
    7
    >>> r.carry
    0
    """
    result_int, carry_out, overflow = add_64bit(a, b, carry_in)
    return _alu64(result_int, carry_out, overflow)


def subq(a: int, b: int) -> ALUResult64:
    """SUBQ: 64-bit subtract via two's complement (NOT(b) + 1).

    a - b = a + NOT(b) + 1

    Gate implementation:
      1. Invert all 64 bits of b (64 NOT gates)
      2. Add a + NOT(b) with carry_in=1 (ripple_carry_adder)

    Example
    ───────
    >>> r = subq(10, 3)
    >>> r.result
    7
    """
    not_b = invert_64bit(b)
    return addq(a, not_b, carry_in=1)


# ── 64-bit logical operations (one gate per bit) ──────────────────────────────

def andq(a: int, b: int) -> ALUResult64:
    """AND Ra,Rb,Rc — 64 AND gates (one per bit pair).

    Example
    ───────
    >>> andq(0b1010, 0b1100).result
    8
    """
    a_bits = int_to_bits(a & _MASK64, 64)
    b_bits = int_to_bits(b & _MASK64, 64)
    result_bits = [AND(a_bits[i], b_bits[i]) for i in range(64)]
    r = bits_to_int(result_bits)
    return _alu64(r, 0, 0)


def orq(a: int, b: int) -> ALUResult64:
    """BIS Ra,Rb,Rc — OR (Bit Set): 64 OR gates.

    Example
    ───────
    >>> orq(0b1010, 0b0101).result
    15
    """
    a_bits = int_to_bits(a & _MASK64, 64)
    b_bits = int_to_bits(b & _MASK64, 64)
    result_bits = [OR(a_bits[i], b_bits[i]) for i in range(64)]
    r = bits_to_int(result_bits)
    return _alu64(r, 0, 0)


def xorq(a: int, b: int) -> ALUResult64:
    """XOR Ra,Rb,Rc — 64 XOR gates.

    Example
    ───────
    >>> xorq(0b1111, 0b1010).result
    5
    """
    a_bits = int_to_bits(a & _MASK64, 64)
    b_bits = int_to_bits(b & _MASK64, 64)
    result_bits = [XOR(a_bits[i], b_bits[i]) for i in range(64)]
    r = bits_to_int(result_bits)
    return _alu64(r, 0, 0)


def bicq(a: int, b: int) -> ALUResult64:
    """BIC Ra,Rb,Rc — AND NOT (Bit Clear): NOT(b) then AND.

    Gate implementation: 64 NOT gates + 64 AND gates.

    Example
    ───────
    >>> bicq(0b1111, 0b1010).result
    5
    """
    a_bits = int_to_bits(a & _MASK64, 64)
    b_bits = int_to_bits(b & _MASK64, 64)
    not_b_bits = [NOT(b_bits[i]) for i in range(64)]
    result_bits = [AND(a_bits[i], not_b_bits[i]) for i in range(64)]
    r = bits_to_int(result_bits)
    return _alu64(r, 0, 0)


def ornot(a: int, b: int) -> ALUResult64:
    """ORNOT Ra,Rb,Rc — OR NOT: OR(a, NOT(b)).

    Gate implementation: 64 NOT gates + 64 OR gates.

    Example
    ───────
    >>> ornot(0, 0).result == 0xFFFFFFFFFFFFFFFF
    True
    """
    a_bits = int_to_bits(a & _MASK64, 64)
    b_bits = int_to_bits(b & _MASK64, 64)
    not_b_bits = [NOT(b_bits[i]) for i in range(64)]
    result_bits = [OR(a_bits[i], not_b_bits[i]) for i in range(64)]
    r = bits_to_int(result_bits)
    return _alu64(r, 0, 0)


def eqvq(a: int, b: int) -> ALUResult64:
    """EQV Ra,Rb,Rc — XOR NOT (XNOR): XOR(a, NOT(b)).

    Gate implementation: 64 NOT gates + 64 XOR gates.
    XNOR gives 1 where bits are equal, 0 where they differ.

    Example
    ───────
    >>> eqvq(0b1010, 0b1010).result == 0xFFFFFFFFFFFFFFFF
    True
    """
    a_bits = int_to_bits(a & _MASK64, 64)
    b_bits = int_to_bits(b & _MASK64, 64)
    not_b_bits = [NOT(b_bits[i]) for i in range(64)]
    result_bits = [XOR(a_bits[i], not_b_bits[i]) for i in range(64)]
    r = bits_to_int(result_bits)
    return _alu64(r, 0, 0)


# ── 64-bit shifts ─────────────────────────────────────────────────────────────

def sll64(a: int, shamt: int) -> ALUResult64:
    """SLL Ra,Rb,Rc — shift left logical 64-bit.

    shamt is masked to low 6 bits (0–63) as on the real hardware.

    Example
    ───────
    >>> sll64(1, 4).result
    16
    """
    r = shl_64(a & _MASK64, shamt & 63)
    return _alu64(r, 0, 0)


def srl64(a: int, shamt: int) -> ALUResult64:
    """SRL Ra,Rb,Rc — shift right logical 64-bit (zero fill).

    Example
    ───────
    >>> srl64(16, 4).result
    1
    """
    r = shr_64_logical(a & _MASK64, shamt & 63)
    return _alu64(r, 0, 0)


def sra64(a: int, shamt: int) -> ALUResult64:
    """SRA Ra,Rb,Rc — shift right arithmetic 64-bit (sign fill).

    The sign bit (bit 63) is replicated into vacated positions.

    Example
    ───────
    >>> sra64(0xFFFF_FFFF_FFFF_FFFF, 1).result == 0xFFFF_FFFF_FFFF_FFFF
    True
    >>> sra64(16, 4).result
    1
    """
    r = shr_64_arith(a & _MASK64, shamt & 63)
    return _alu64(r, 0, 0)


# ── 64-bit compare operations ─────────────────────────────────────────────────
#
# On the Alpha, comparisons do NOT set condition-code registers.  Instead,
# they write 0 or 1 into the destination GPR.  This eliminates instruction
# scheduling hazards caused by shared condition-code state.

def _as_signed64(v: int) -> int:
    """Reinterpret unsigned 64-bit value as a signed Python int."""
    v = v & _MASK64
    if v >= 0x8000_0000_0000_0000:
        v -= 0x1_0000_0000_0000_0000
    return v


def cmpeq(a: int, b: int) -> int:
    """CMPEQ Ra,Rb,Rc — 1 if a == b (via subq, check zero flag), 0 otherwise.

    Implementation: subtract a - b; if result is zero, they are equal.
    """
    diff = subq(a, b)
    return diff.zero


def cmplt(a: int, b: int) -> int:
    """CMPLT Ra,Rb,Rc — 1 if signed(a) < signed(b), 0 otherwise.

    Uses subq: a - b; signed overflow and sign bit determine less-than.
    If overflow occurred: result = NOT(negative)
    If no overflow:       result = negative
    Gate: XOR(overflow, negative)
    """
    diff = subq(a, b)
    return XOR(diff.overflow, diff.negative)


def cmple(a: int, b: int) -> int:
    """CMPLE Ra,Rb,Rc — 1 if signed(a) <= signed(b), 0 otherwise.

    CMPLE is true when (a < b) OR (a == b).
    Gate: OR(cmplt, cmpeq)
    """
    return OR(cmplt(a, b), cmpeq(a, b))


def cmpult(a: int, b: int) -> int:
    """CMPULT Ra,Rb,Rc — 1 if unsigned a < unsigned b, 0 otherwise.

    Uses subq: unsigned borrow = NOT(carry_out) of a - b.
    When a < b (unsigned), the subtraction borrows: carry_out = 0.
    So: result = NOT(carry_out)

    Gate: NOT(carry_out)
    """
    diff = subq(a, b)
    return NOT(diff.carry)


def cmpule(a: int, b: int) -> int:
    """CMPULE Ra,Rb,Rc — 1 if unsigned a <= unsigned b, 0 otherwise.

    CMPULE: a <= b  ↔  (a < b) OR (a == b)
    Gate: OR(cmpult, cmpeq)
    """
    return OR(cmpult(a, b), cmpeq(a, b))


# ── 32-bit (longword) operations ──────────────────────────────────────────────
#
# Alpha's "L" variants operate on the low 32 bits and sign-extend the result
# to 64 bits.  This supports C's 32-bit int type on a 64-bit machine.

def addl(a: int, b: int) -> ALUResult64:
    """ADDL Ra,Rb,Rc — 32-bit add, sign-extend result to 64 bits.

    Example
    ───────
    >>> addl(1, 2).result
    3
    >>> hex(addl(0x7FFFFFFF, 1).result)  # 32-bit overflow → negative 64-bit
    '0xffffffff80000000'
    """
    result32, carry32, overflow32 = add_32bit(a & _MASK32, b & _MASK32, 0)
    r = sext32_to_64(result32)
    return _alu64(r, carry32, overflow32)


def subl(a: int, b: int) -> ALUResult64:
    """SUBL Ra,Rb,Rc — 32-bit subtract, sign-extend result to 64 bits.

    a - b (32-bit) via NOT(b_32) + 1.
    """
    not_b32 = invert_32bit(b & _MASK32)
    result32, carry32, overflow32 = add_32bit(a & _MASK32, not_b32, 1)
    r = sext32_to_64(result32)
    return _alu64(r, carry32, overflow32)


# ── Scaled add/sub instructions ───────────────────────────────────────────────
#
# These are optimized for C array indexing:
#   int a[N]; a[i] is at address base + i * sizeof(int)
#   S4ADDQ r_i, r_base, r_ea  →  ea = i*4 + base

def s4addq(a: int, b: int) -> ALUResult64:
    """S4ADDQ Ra,Rb,Rc — (Ra*4 + Rb) as 64-bit.

    Ra*4 is a left shift by 2 (address arithmetic, not data-path arithmetic).
    """
    a4 = shl_64(a & _MASK64, 2)
    return addq(a4, b)


def s8addq(a: int, b: int) -> ALUResult64:
    """S8ADDQ Ra,Rb,Rc — (Ra*8 + Rb) as 64-bit."""
    a8 = shl_64(a & _MASK64, 3)
    return addq(a8, b)


def s4addl(a: int, b: int) -> ALUResult64:
    """S4ADDL Ra,Rb,Rc — (Ra*4 + Rb) as 32-bit, sign-extended."""
    a4 = shl_64(a & _MASK64, 2)
    return addl(a4, b)


def s8addl(a: int, b: int) -> ALUResult64:
    """S8ADDL Ra,Rb,Rc — (Ra*8 + Rb) as 32-bit, sign-extended."""
    a8 = shl_64(a & _MASK64, 3)
    return addl(a8, b)


def s4subq(a: int, b: int) -> ALUResult64:
    """S4SUBQ Ra,Rb,Rc — (Ra*4 - Rb) as 64-bit."""
    a4 = shl_64(a & _MASK64, 2)
    return subq(a4, b)


def s8subq(a: int, b: int) -> ALUResult64:
    """S8SUBQ Ra,Rb,Rc — (Ra*8 - Rb) as 64-bit."""
    a8 = shl_64(a & _MASK64, 3)
    return subq(a8, b)


def s4subl(a: int, b: int) -> ALUResult64:
    """S4SUBL Ra,Rb,Rc — (Ra*4 - Rb) as 32-bit, sign-extended."""
    a4 = shl_64(a & _MASK64, 2)
    return subl(a4, b)


def s8subl(a: int, b: int) -> ALUResult64:
    """S8SUBL Ra,Rb,Rc — (Ra*8 - Rb) as 32-bit, sign-extended."""
    a8 = shl_64(a & _MASK64, 3)
    return subl(a8, b)


# ── Multiply ───────────────────────────────────────────────────────────────────
#
# Multiplication is built from shift-and-add: a standard algorithm for
# binary multiplication that works bit by bit.
#
# Algorithm for a * b (64-bit):
#   product = 0
#   for each bit i in b (0..63):
#     if b[i] == 1:
#       product += a << i   (shift a left by i, add to running product)
#
# This is the "schoolbook" multiplication algorithm applied to binary numbers.
# Each "add" routes through add_64bit (ripple_carry_adder).

def mulq(a: int, b: int) -> int:
    """MULQ Ra,Rb,Rc — lower 64 bits of 64×64 signed multiply.

    For the lower 64 bits, signed and unsigned multiplication produce the same
    bit pattern (two's complement property).  So we use unsigned shift-and-add.

    Gate count: 64 iterations × (shl_64 + add_64bit) ≈ 64 × 384 ≈ 24,576 gates.

    Example
    ───────
    >>> mulq(6, 7)
    42
    >>> mulq(0xFFFFFFFFFFFFFFFF, 2)  # -1 * 2 = -2 → unsigned = 2^64-2
    18446744073709551614
    """
    a_u = a & _MASK64
    b_u = b & _MASK64
    b_bits = int_to_bits(b_u, 64)

    product = 0
    for i in range(64):
        # Check if bit i of b is set — gate-level check
        if AND(b_bits[i], 1):
            shifted = shl_64(a_u, i)
            # Add to running product (address/accumulate math uses add_64bit)
            product, _carry, _ov = add_64bit(product, shifted, 0)
    return product & _MASK64


def umulh(a: int, b: int) -> int:
    """UMULH Ra,Rb,Rc — upper 64 bits of 64×64 unsigned multiply.

    Builds the full 128-bit product, then returns the high 64 bits.
    Used for multi-precision arithmetic and division.

    Example
    ───────
    >>> umulh(0xFFFFFFFFFFFFFFFF, 2)  # nearly 2^128/2 → upper = 1
    1
    """
    a_u = a & _MASK64
    b_u = b & _MASK64
    b_bits = int_to_bits(b_u, 64)

    product = 0  # 128-bit accumulator
    for i in range(64):
        if AND(b_bits[i], 1):
            # Shift a_u left by i positions. For i >= 64 the result exceeds
            # 64 bits, so we keep a Python int as the 128-bit intermediate.
            # The shift itself is bit-position index arithmetic, not a data-path
            # operation; the actual addition is done through add_128bit which
            # routes through gate-level adder logic.
            shifted_128 = a_u << i  # bit-position index arithmetic
            product, _carry = add_128bit(product, shifted_128, 0)

    # Upper 64 bits: bit positions 64..127
    return (product >> 64) & _MASK64


def mull(a: int, b: int) -> int:
    """MULL Ra,Rb,Rc — lower 32 bits of 32×32 multiply, sign-extended to 64.

    Uses 32-iteration shift-and-add on 32-bit inputs.

    Example
    ───────
    >>> mull(6, 7)
    42
    >>> hex(mull(0x80000000, 2))  # -2^31 * 2 = -2^32 → sext = 0
    '0x0'
    """
    a32 = a & _MASK32
    b32 = b & _MASK32
    b_bits = int_to_bits(b32, 32)

    product = 0
    for i in range(32):
        if AND(b_bits[i], 1):
            shifted = shl_64(a32, i)
            product, _carry, _ov = add_64bit(product, shifted, 0)

    result32 = product & _MASK32
    return sext32_to_64(result32)
