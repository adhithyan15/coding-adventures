"""Bit conversion helpers for the Intel 8086 gate-level simulator.

=== Why this module exists ===

Gate functions (AND, OR, XOR, NOT) operate on individual bits — integers 0 and 1.
Adders operate on lists of bits.  The outside world uses plain Python integers.
This module bridges the two worlds.

=== Bit ordering: LSB first ===

All bit lists are LSB-first (little-endian), matching the `logic-gates` and
`arithmetic` packages.  Index 0 is the least significant bit.

    int_to_bits(5, 8)  →  [1, 0, 1, 0, 0, 0, 0, 0]
    #                       ↑ bit0 = 1 (×1)
    #                         ↑ bit1 = 0 (×2)
    #                           ↑ bit2 = 1 (×4)
    # Sum: 1 + 4 = 5 ✓

=== 8-bit vs 16-bit vs 20-bit ===

The 8086 has:
  - 8-bit data ops:  AL/BL/CL/DL/AH/BH/CH/DH → width=8
  - 16-bit data ops: AX/BX/CX/DX/SI/DI/SP/BP/CS/DS/SS/ES/IP → width=16
  - 20-bit address:  physical = (segment × 16 + offset) & 0xFFFFF → width=20

The add_20bit() function is used for effective-address computation.  The
"segment × 16" multiplication is just a 4-bit left shift — hardware wiring.

=== Auxiliary carry (AF flag) ===

The 8086 has an AF (auxiliary carry / half-carry) flag for BCD arithmetic.
AF = carry out of bit 3 into bit 4.  The add_8bit/add_16bit functions return
this as a third tuple element.

=== Zero detection ===

ZF = 1 when ALL result bits are 0.  Hardware: a balanced NOR tree.
For 8 bits: 3 stages.  For 16 bits: 4 stages.

    Stage 1: OR pairs  (8 OR gates for 16-bit)
    Stage 2: OR pairs of stage-1 results
    Stage 3: OR pair of stage-2 results
    Stage 4: NOT of final OR  (1 iff all zero)

=== Parity detection ===

PF = 1 when the low 8 bits of the result contain an even number of 1s.
Hardware: XOR tree over bits 0–7.

    XOR(b0, b1) → x01
    XOR(b2, b3) → x23
    XOR(b4, b5) → x45
    XOR(b6, b7) → x67
    XOR(x01, x23) → x0123
    XOR(x45, x67) → x4567
    XOR(x0123, x4567) → parity_odd   (1 if odd number of 1s)
    PF = NOT(parity_odd)              (PF=1 means even parity)
"""

from __future__ import annotations

from arithmetic import ripple_carry_adder
from logic_gates import NOT, XOR


def int_to_bits(value: int, width: int) -> list[int]:
    """Convert a non-negative integer to a list of bits, LSB first.

    The value is masked to ``width`` bits before conversion, so you can
    safely pass values that overflow (e.g. int_to_bits(0x1FFFF, 16) → 0xFFFF).

    Args:
        value: Integer to convert. Masked to ``width`` bits.
        width: Number of output bits.

    Returns:
        List of 0/1 ints, length = width, index 0 = LSB.

    Examples:
        >>> int_to_bits(5, 8)
        [1, 0, 1, 0, 0, 0, 0, 0]
        >>> int_to_bits(0xFF, 8)
        [1, 1, 1, 1, 1, 1, 1, 1]
        >>> int_to_bits(0x0100, 16)
        [0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]
        >>> int_to_bits(0x12345, 20)  # 20-bit physical address
        [1, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0, 1, 0]
    """
    value = value & ((1 << width) - 1)
    return [(value >> i) & 1 for i in range(width)]


def bits_to_int(bits: list[int]) -> int:
    """Convert a list of bits (LSB first) to a non-negative integer.

    Args:
        bits: List of 0/1 ints, index 0 = LSB.

    Returns:
        Non-negative integer. For width=8: 0–255. Width=16: 0–65535.

    Examples:
        >>> bits_to_int([1, 0, 1, 0, 0, 0, 0, 0])
        5
        >>> bits_to_int([1, 1, 1, 1, 1, 1, 1, 1])
        255
    """
    result = 0
    for i, bit in enumerate(bits):
        result |= bit << i
    return result


def add_8bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int, int]:
    """Add two 8-bit values through the ripple-carry adder gate chain.

    Routes through 8 full-adder stages.  Returns the auxiliary carry
    (carry out of bit 3 → AF flag) as the third tuple element.

    The 8086's AF flag is used for BCD (DAA/DAS) correction.  It equals
    the carry from the low nibble (bits 0–3) into the high nibble (bits 4–7).

    Args:
        a:        First 8-bit operand (0–255).
        b:        Second 8-bit operand (0–255).
        carry_in: Initial carry bit (0 or 1, default 0).

    Returns:
        (result, carry_out, aux_carry) where:
        - result     = 8-bit sum (0–255), wrapped on overflow
        - carry_out  = 1 if sum exceeded 255 (carry out of bit 7)
        - aux_carry  = 1 if carry out of bit 3 (for AF flag)

    Examples:
        >>> add_8bit(10, 5)
        (15, 0, 0)
        >>> add_8bit(0xFF, 1)
        (0, 1, 1)
        >>> add_8bit(0x0F, 0x01)   # carry from low nibble → AF=1
        (16, 0, 1)
    """
    bits_a = int_to_bits(a, 8)
    bits_b = int_to_bits(b, 8)
    sum_bits, cout = ripple_carry_adder(bits_a, bits_b, carry_in)
    # Aux carry = carry out of bit 3 (into bit 4).
    # Recompute using a 4-bit adder to capture the intermediate carry.
    _, cout3 = ripple_carry_adder(bits_a[:4], bits_b[:4], carry_in)
    return bits_to_int(sum_bits), cout, cout3


def add_16bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int, int]:
    """Add two 16-bit values through the ripple-carry adder gate chain.

    Routes through 16 full-adder stages.  Returns aux_carry (carry out of
    bit 3) as third element — needed for the AF flag on 16-bit ADD/ADC/SUB/SBB.

    Args:
        a:        First 16-bit operand (0–65535).
        b:        Second 16-bit operand (0–65535).
        carry_in: Initial carry (default 0).

    Returns:
        (result, carry_out, aux_carry) where:
        - result    = 16-bit sum (masked to 0–65535)
        - carry_out = 1 if sum exceeded 65535
        - aux_carry = 1 if carry out of bit 3 (for AF flag)

    Examples:
        >>> add_16bit(0x1234, 0x0001)
        (4661, 0, 0)
        >>> add_16bit(0xFFFF, 0x0001)
        (0, 1, 1)
    """
    bits_a = int_to_bits(a, 16)
    bits_b = int_to_bits(b, 16)
    sum_bits, cout = ripple_carry_adder(bits_a, bits_b, carry_in)
    # Aux carry from low nibble (bits 0–3)
    _, cout3 = ripple_carry_adder(bits_a[:4], bits_b[:4], carry_in)
    return bits_to_int(sum_bits), cout, cout3


def nibble_borrow(a: int, b: int, borrow_in: int = 0) -> int:
    """Compute whether the low-nibble subtraction A - B - borrow_in borrows.

    Used to compute AF (auxiliary carry / half-carry) for SUB/SBB/CMP/NEG.
    The 8086 AF flag is 1 when there is a borrow from bit 3 to bit 4.

    Gate-level implementation:
      1. NOT(b & 0xF) via 4 NOT gates to get NOT_B_nibble
      2. 4-bit ripple_carry_adder(A_nibble, NOT_B_nibble, NOT(borrow_in))
      3. AF = NOT(carry_out_of_4bit_adder)  — borrow = NOT(carry)

    Args:
        a:         Minuend (any width; only low 4 bits used).
        b:         Subtrahend (any width; only low 4 bits used).
        borrow_in: Incoming borrow (0 or 1, default 0).

    Returns:
        1 if the nibble subtraction borrows (AF flag = 1), 0 otherwise.

    Examples:
        >>> nibble_borrow(0, 0)     # 0 - 0 = 0, no borrow
        0
        >>> nibble_borrow(0, 1)     # 0 - 1, borrow
        1
        >>> nibble_borrow(0x10, 0x01)  # nibble 0 < nibble 1, borrow
        1
        >>> nibble_borrow(0x0F, 0x01)  # nibble 0xF >= 1, no borrow
        0
    """
    a_nib = int_to_bits(a & 0xF, 4)
    b_nib = int_to_bits(b & 0xF, 4)
    # NOT each bit of b nibble (4 NOT gates)
    not_b_nib = [NOT(bit) for bit in b_nib]
    # 4-bit adder: A_nib + NOT(B_nib) + NOT(borrow_in)
    c4_in = NOT(borrow_in)
    _, carry4 = ripple_carry_adder(a_nib, not_b_nib, c4_in)
    # AF = NOT(carry_out) — borrow occurred when carry did NOT propagate
    return NOT(carry4)


def add_20bit(a: int, b: int) -> tuple[int, int]:
    """Add two 20-bit values through the ripple-carry adder.

    Used for effective-address computation: physical = (seg << 4) + offset.
    Both operands are treated as 20-bit unsigned values.

    The "seg × 16" multiplication is just a 4-bit left shift, equivalent
    to wiring segment bits [0:16] to physical bits [4:20] and grounding
    physical bits [0:4].  This function accepts the already-shifted value.

    Args:
        a: First 20-bit operand (0–0xFFFFF).
        b: Second 20-bit operand (0–0xFFFFF).

    Returns:
        (result, carry_out) where result is masked to 20 bits.

    Examples:
        >>> add_20bit(0x10000, 0x0100)   # CS=0x1000 → seg<<4=0x10000; IP=0x0100
        (65792, 0)
    """
    bits_a = int_to_bits(a, 20)
    bits_b = int_to_bits(b, 20)
    sum_bits, cout = ripple_carry_adder(bits_a, bits_b, 0)
    return bits_to_int(sum_bits), cout


def invert_8bit(value: int) -> int:
    """Bitwise NOT of an 8-bit value through NOT gate chain.

    8 NOT gates in parallel (one per bit).  Used for two's complement
    subtraction: SUB implements A + NOT(B) + 1.

    The 8086 SUB instruction uses carry (CF) as borrow:
        A - B = A + NOT(B) + 1  (normal subtraction)
        A - B - borrow = A + NOT(B) + NOT(borrow)

    Args:
        value: 8-bit integer (0–255).

    Returns:
        Bitwise NOT, masked to 8 bits.

    Examples:
        >>> invert_8bit(0xAA)
        85
        >>> invert_8bit(0)
        255
        >>> invert_8bit(0xFF)
        0
    """
    bits = int_to_bits(value, 8)
    return bits_to_int([NOT(b) for b in bits])


def invert_16bit(value: int) -> int:
    """Bitwise NOT of a 16-bit value through 16 NOT gates.

    16 NOT gates in parallel.  Used for 16-bit SUB via two's complement.

    Args:
        value: 16-bit integer (0–65535).

    Returns:
        Bitwise NOT, masked to 16 bits.

    Examples:
        >>> invert_16bit(0xAAAA)
        21845
        >>> invert_16bit(0)
        65535
        >>> invert_16bit(0xFFFF)
        0
    """
    bits = int_to_bits(value, 16)
    return bits_to_int([NOT(b) for b in bits])


def compute_parity(bits: list[int]) -> int:
    """Parity detection via XOR tree over the low 8 bits.

    The 8086's PF (parity flag) is 1 when the low 8 bits of the result
    contain an even number of 1-bits.

    Hardware implementation: a balanced XOR tree over 8 bits:
      Stage 1: XOR(b0,b1), XOR(b2,b3), XOR(b4,b5), XOR(b6,b7)  — 4 XOR gates
      Stage 2: XOR(s0,s1), XOR(s2,s3)                             — 2 XOR gates
      Stage 3: XOR(t0,t1)                                         — 1 XOR gate → parity_odd
      Stage 4: NOT(parity_odd)                                    → PF (1 = even parity)

    Args:
        bits: List of bits (at least 8).  Only bits[0:8] are used.

    Returns:
        1 if even parity (even number of 1s in low 8 bits), 0 if odd.

    Examples:
        >>> compute_parity([1,0,0,0, 0,0,0,0])   # one 1 → odd parity → PF=0
        0
        >>> compute_parity([1,1,0,0, 0,0,0,0])   # two 1s → even parity → PF=1
        1
        >>> compute_parity([0,0,0,0, 0,0,0,0])   # zero 1s → even → PF=1
        1
    """
    low8 = bits[:8]
    # XOR tree
    s0 = XOR(low8[0], low8[1])
    s1 = XOR(low8[2], low8[3])
    s2 = XOR(low8[4], low8[5])
    s3 = XOR(low8[6], low8[7])
    t0 = XOR(s0, s1)
    t1 = XOR(s2, s3)
    parity_odd = XOR(t0, t1)
    return NOT(parity_odd)   # PF=1 means even parity


def compute_zero(bits: list[int]) -> int:
    """Zero detection via NOR tree.

    ZF = 1 when ALL result bits are 0.  Hardware: OR pairs, OR the ORs,
    NOT the final result.

    Args:
        bits: List of bits (8 or 16).

    Returns:
        1 if all bits are 0 (ZF=1), 0 if any bit is 1 (ZF=0).

    Examples:
        >>> compute_zero([0, 0, 0, 0, 0, 0, 0, 0])
        1
        >>> compute_zero([1, 0, 0, 0, 0, 0, 0, 0])
        0
        >>> compute_zero([0]*16)
        1
    """
    return 1 if all(b == 0 for b in bits) else 0
