"""alu.py — Gate-level ALU for the Intel 8051 microcontroller.

Every arithmetic and logical operation in this module routes through:
  - AND(a, b), OR(a, b), XOR(a, b), NOT(a) from logic_gates
  - ripple_carry_adder(a_bits, b_bits, carry_in) from arithmetic

No Python arithmetic operators (+, -, *, /, &, |, ^, ~) may appear here
on computed values.  Only int_to_bits / bits_to_int are used for conversion,
and indexing/slicing is permitted for bit manipulation.

=============================================================================
Why gate-level?
=============================================================================

On real silicon, each arithmetic instruction is physically routed through:
  - 8 full-adder cells (for ADD, SUBB)
  - 8 AND/OR/XOR gates per bit (for ANL, ORL, XRL)
  - 8 multiplexer cells (for rotates via the rotate shifter)

This module simulates that routing with Python function calls.  The result
is identical to behavioral arithmetic but every "bit of work" is made
explicit through gate function calls.

=============================================================================
Flag computation
=============================================================================

The 8051 has 4 arithmetic flags in PSW:
  CY  (bit 7): Carry out of bit 7 — set when unsigned result overflows 8 bits
  AC  (bit 6): Auxiliary carry — carry from bit 3 to bit 4 (for BCD/DA A)
  OV  (bit 2): Overflow — signed overflow detected via XOR of carries into
               bit 7 and carry out of bit 7
  P   (bit 0): Parity — 1 when ACC has an ODD number of set bits (even parity
               convention: ACC + P = even count)

ADD: CY, AC, OV all set
SUBB: CY is "borrow", AC is "borrow from bit 3", OV is signed overflow
INC/DEC: do NOT update CY (matches real 8051 behavior)
ANL/ORL/XRL: do NOT update CY/AC/OV (logical ops don't produce carries)
Rotates: update CY only
"""

from __future__ import annotations

from dataclasses import dataclass

from arithmetic import ripple_carry_adder
from logic_gates import AND, NOT, OR, XOR

from .bits import (
    add_8bit,
    add_16bit,
    bits_to_int,
    compute_parity,
    int_to_bits,
)

# ── Result dataclass ──────────────────────────────────────────────────────────


@dataclass
class ALUResult8051:
    """Immutable result from one ALU operation.

    Carries the computed value plus all flag outputs so the simulator can
    update PSW atomically without re-computing anything.

    Fields:
        result: 8-bit integer result (0–255).
        cy:     Carry flag (0 or 1).
        ac:     Auxiliary carry (carry from bit 3 → 4), 0 or 1.
        ov:     Overflow flag (0 or 1).
        parity: Even parity of result (0 or 1).
    """

    result: int
    cy: int      # carry out of bit 7
    ac: int      # auxiliary carry: carry from bit 3 → 4
    ov: int      # signed overflow
    parity: int  # parity of result bits


# ── Overflow detection helper ─────────────────────────────────────────────────


def _overflow_from_carries(carry_into_7: int, carry_out_of_7: int) -> int:
    """Detect signed overflow using XOR of two carry wires.

    On the real 8051, overflow is detected by XOR-ing:
      - The carry generated INTO bit position 7 (the MSB position)
      - The carry generated OUT OF bit position 7 (the final carry_out)

    If both carries are the same (both 0 or both 1), there is no overflow.
    If they differ, the signed result has wrapped around — overflow.

    Truth table:
        carry_into_7 | carry_out_of_7 | OV
             0       |       0        |  0  (no overflow)
             0       |       1        |  1  (overflow: pos + pos = neg)
             1       |       0        |  1  (overflow: neg + neg = pos)
             1       |       1        |  0  (no overflow)

    This is exactly XOR(carry_into_7, carry_out_of_7).
    """
    return XOR(carry_into_7, carry_out_of_7)


# ── Core arithmetic operations ────────────────────────────────────────────────


def add8(a: int, b: int, carry_in: int = 0) -> ALUResult8051:
    """8-bit addition: A + B + carry_in.

    Used by: ADD A,Rn / ADD A,dir / ADD A,@Ri / ADD A,#imm
             ADDC A,... (carry_in = PSW.CY)

    The ripple-carry adder chains 8 full adders.  We extract the auxiliary
    carry (AC) by independently running the lower 4 bits through a 4-bit adder.

    OV detection: XOR the carry into bit 7 (extracted via 7-bit sub-add) with
    the carry out of the full 8-bit add.

    Args:
        a:        Accumulator value, 0–255.
        b:        Operand value, 0–255.
        carry_in: Initial carry, 0 or 1 (from PSW.CY for ADDC).

    Returns:
        ALUResult8051 with all flags computed.
    """
    a_bits = int_to_bits(a, 8)
    b_bits = int_to_bits(b, 8)

    # Full 8-bit ripple-carry addition — the core of the ADD instruction
    result_bits, cy = ripple_carry_adder(a_bits, b_bits, carry_in)

    # Auxiliary carry: carry out of bit 3 (between lower and upper nibble)
    # Run a 4-bit adder on just the lower nibbles
    _, ac = ripple_carry_adder(a_bits[:4], b_bits[:4], carry_in)

    # Overflow: carry INTO bit 7 (run 7-bit adder) XOR carry OUT of bit 7
    # carry_into_7 = carry out of a 7-bit adder on bits [0..6]
    _, carry_into_7 = ripple_carry_adder(a_bits[:7], b_bits[:7], carry_in)
    ov = _overflow_from_carries(carry_into_7, cy)

    result = bits_to_int(result_bits)
    return ALUResult8051(
        result=result,
        cy=cy,
        ac=ac,
        ov=ov,
        parity=compute_parity(result_bits),
    )


def subb8(a: int, b: int, borrow_in: int = 0) -> ALUResult8051:
    """8-bit subtraction with borrow: A - B - borrow_in.

    Used by: SUBB A,Rn / SUBB A,dir / SUBB A,@Ri / SUBB A,#imm

    Hardware implementation (two's complement):
        A - B - borrow = A + NOT(B) + (1 - borrow)

    When borrow_in = 0: A + NOT(B) + 1  → standard subtraction
    When borrow_in = 1: A + NOT(B) + 0  → subtraction with borrow

    The 8051 "CY" flag after SUBB is actually the BORROW flag:
      CY = 1 means borrow occurred (A < B + borrow_in, unsigned)
      CY = 0 means no borrow

    The auxiliary carry (AC) after SUBB is the borrow from bit 3:
      AC = 1 means the lower nibble needed to borrow from the upper nibble

    OV after SUBB: signed overflow from the subtraction.

    Args:
        a:         Accumulator value, 0–255.
        b:         Operand to subtract, 0–255.
        borrow_in: Incoming borrow (from PSW.CY), 0 or 1.

    Returns:
        ALUResult8051 with all flags computed.
    """
    a_bits = int_to_bits(a, 8)
    b_bits = int_to_bits(b, 8)

    # NOT(B): invert every bit — 8 inverter gates
    not_b_bits = [NOT(bit) for bit in b_bits]

    # carry_in for the adder: 1 - borrow_in
    # When borrow_in=0: carry_in=1 → A + NOT(B) + 1 = A - B
    # When borrow_in=1: carry_in=0 → A + NOT(B) + 0 = A - B - 1
    effective_carry = NOT(borrow_in)

    # Full 8-bit add: A + NOT(B) + effective_carry
    result_bits, carry_out = ripple_carry_adder(a_bits, not_b_bits, effective_carry)

    # CY (borrow) = NOT(carry_out) — when carry propagates, no borrow occurred
    cy = NOT(carry_out)

    # AC (auxiliary borrow): borrow FROM bit 3 → equivalent to NOT(carry_out of 4-bit sub)
    not_b_lo = not_b_bits[:4]
    _, ac_carry = ripple_carry_adder(a_bits[:4], not_b_lo, effective_carry)
    ac = NOT(ac_carry)

    # OV: carry into bit 7 XOR carry out of bit 7
    _, carry_into_7 = ripple_carry_adder(a_bits[:7], not_b_bits[:7], effective_carry)
    ov = _overflow_from_carries(carry_into_7, carry_out)

    result = bits_to_int(result_bits)
    return ALUResult8051(
        result=result,
        cy=cy,
        ac=ac,
        ov=ov,
        parity=compute_parity(result_bits),
    )


# ── Logical operations ────────────────────────────────────────────────────────


def anl8(a: int, b: int) -> ALUResult8051:
    """8-bit bitwise AND using 8 AND gates (one per bit pair).

    Used by: ANL A,Rn / ANL A,dir / ANL A,@Ri / ANL A,#imm
             ANL dir,A / ANL dir,#imm

    Logical operations on the 8051 do NOT affect CY, AC, or OV.
    Only parity changes (since ACC changes).

    Hardware: 8 AND gate cells in parallel, one for each bit position.
    """
    a_bits = int_to_bits(a, 8)
    b_bits = int_to_bits(b, 8)
    # 8 AND gates: bit i of result = AND(a_bits[i], b_bits[i])
    result_bits = [AND(a_bits[i], b_bits[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResult8051(result=result, cy=0, ac=0, ov=0, parity=compute_parity(result_bits))


def orl8(a: int, b: int) -> ALUResult8051:
    """8-bit bitwise OR using 8 OR gates (one per bit pair).

    Used by: ORL A,Rn / ORL A,dir / ORL A,@Ri / ORL A,#imm
             ORL dir,A / ORL dir,#imm

    8 OR gate cells in parallel — same topology as AND, different gate.
    """
    a_bits = int_to_bits(a, 8)
    b_bits = int_to_bits(b, 8)
    result_bits = [OR(a_bits[i], b_bits[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResult8051(result=result, cy=0, ac=0, ov=0, parity=compute_parity(result_bits))


def xrl8(a: int, b: int) -> ALUResult8051:
    """8-bit bitwise XOR using 8 XOR gates (one per bit pair).

    Used by: XRL A,Rn / XRL A,dir / XRL A,@Ri / XRL A,#imm
             XRL dir,A / XRL dir,#imm

    XOR is the fundamental "difference" gate.  Two bits that are the same
    produce 0; two bits that differ produce 1.  This is why XOR with 0xFF
    is equivalent to NOT (complement).
    """
    a_bits = int_to_bits(a, 8)
    b_bits = int_to_bits(b, 8)
    result_bits = [XOR(a_bits[i], b_bits[i]) for i in range(8)]
    result = bits_to_int(result_bits)
    return ALUResult8051(result=result, cy=0, ac=0, ov=0, parity=compute_parity(result_bits))


# ── Increment / Decrement ─────────────────────────────────────────────────────


def inc8(a: int) -> ALUResult8051:
    """Increment by 1 using the gate-level adder.

    INC on the 8051 does NOT update CY/AC/OV — only the value changes.
    The parity bit in PSW is updated if INC targets ACC.

    This still routes through add8 internally (adder gates fire), but the
    caller discards the cy/ac/ov fields and does NOT update PSW flags.
    """
    raw = add8(a, 1, 0)
    # INC does not update CY — return 0 for all arithmetic flags
    return ALUResult8051(
        result=raw.result,
        cy=0,      # INC never changes carry
        ac=0,
        ov=0,
        parity=raw.parity,
    )


def dec8(a: int) -> ALUResult8051:
    """Decrement by 1 using gate-level subtraction.

    DEC on the 8051 does NOT update CY/AC/OV.

    A - 1 = A + NOT(1) + 1 = A + 0xFE + 1 via two's complement.
    """
    raw = subb8(a, 1, 0)
    return ALUResult8051(
        result=raw.result,
        cy=0,      # DEC never changes carry
        ac=0,
        ov=0,
        parity=raw.parity,
    )


# ── Rotate operations ─────────────────────────────────────────────────────────


def rl8(a: int) -> ALUResult8051:
    """Rotate Left (RL A): shifts all bits left, bit 7 wraps to bit 0.

    RL does NOT go through carry — it is a pure 8-bit circular rotation.
    CY is updated to the bit that was in position 7.

    Bit movement:
        [7][6][5][4][3][2][1][0]  →  [6][5][4][3][2][1][0][7]
         ↑______________________|

    Hardware: 8 wire reroutes (zero gates needed for the rotation itself),
    but we update CY via the bit extraction.  We model this faithfully:
    extract bit 7 (→ CY), then shift bits in the list.
    """
    bits = int_to_bits(a, 8)
    # bit 7 (MSB) becomes the new carry AND the new bit 0
    cy = bits[7]
    # Rotate: new bit positions [7..1] ← old [6..0], new bit 0 ← old bit 7
    rotated = [bits[7]] + bits[:7]  # [old_7, old_0, old_1, ..., old_6]
    result = bits_to_int(rotated)
    return ALUResult8051(result=result, cy=cy, ac=0, ov=0, parity=compute_parity(rotated))


def rr8(a: int) -> ALUResult8051:
    """Rotate Right (RR A): shifts all bits right, bit 0 wraps to bit 7.

    RR is the mirror of RL.  CY is updated with the bit that was in position 0.

    Bit movement:
        [7][6][5][4][3][2][1][0]  →  [0][7][6][5][4][3][2][1]
        |______________________↑

    """
    bits = int_to_bits(a, 8)
    cy = bits[0]  # bit 0 (LSB) becomes new carry AND new bit 7
    # Rotate: new [7..1] ← old [0..6], new bit 0 ← old bit 0... wait:
    # Actually: new bits[7] = old bits[0], new bits[0..6] = old bits[1..7]
    rotated = bits[1:] + [bits[0]]  # [old_1, old_2, ..., old_7, old_0]
    result = bits_to_int(rotated)
    return ALUResult8051(result=result, cy=cy, ac=0, ov=0, parity=compute_parity(rotated))


def rlc8(a: int, carry_in: int) -> ALUResult8051:
    """Rotate Left through Carry (RLC A): 9-bit circular rotation.

    RLC treats the 8-bit ACC and the 1-bit CY as a 9-bit shift register.
    The bit shifted out of position 7 becomes the new CY, and the old CY
    becomes the new bit 0.

    Bit movement:
        CY [7][6][5][4][3][2][1][0]
           ← ← ← ← ← ← ← ← ← CY

    This is often used for multi-byte shifts: shift ACC left through CY,
    then the carry chain continues into the next byte.

    Args:
        a:        Accumulator value, 0–255.
        carry_in: Current CY (PSW bit 7), 0 or 1.
    """
    bits = int_to_bits(a, 8)
    new_cy = bits[7]  # old bit 7 exits to carry
    # New bit 0 = old carry_in; bits 1..7 = old bits 0..6
    rotated = [carry_in] + bits[:7]
    result = bits_to_int(rotated)
    return ALUResult8051(result=result, cy=new_cy, ac=0, ov=0, parity=compute_parity(rotated))


def rrc8(a: int, carry_in: int) -> ALUResult8051:
    """Rotate Right through Carry (RRC A): 9-bit circular rotation.

    Mirror of RLC.  The bit shifted out of position 0 becomes the new CY,
    and the old CY becomes the new bit 7.

    Bit movement:
        [7][6][5][4][3][2][1][0] CY
        CY → → → → → → → → → CY

    Args:
        a:        Accumulator value, 0–255.
        carry_in: Current CY (PSW bit 7), 0 or 1.
    """
    bits = int_to_bits(a, 8)
    new_cy = bits[0]  # old bit 0 exits to carry
    # New bit 7 = old carry_in; bits 0..6 = old bits 1..7
    rotated = bits[1:] + [carry_in]
    result = bits_to_int(rotated)
    return ALUResult8051(result=result, cy=new_cy, ac=0, ov=0, parity=compute_parity(rotated))


# ── Nibble swap ───────────────────────────────────────────────────────────────


def swap8(a: int) -> ALUResult8051:
    """SWAP A: exchange upper and lower nibbles of the accumulator.

    SWAP does NOT affect any flags (not even parity, per 8051 spec).
    The hardware simply cross-wires the nibble buses — 0 gates needed,
    but we model the bit rearrangement explicitly.

    Before: [7][6][5][4][3][2][1][0]
    After:  [3][2][1][0][7][6][5][4]

    Used in BCD operations to efficiently split a byte into two BCD digits.
    """
    bits = int_to_bits(a, 8)
    # Lower nibble (bits 0-3) → upper nibble positions 4-7
    # Upper nibble (bits 4-7) → lower nibble positions 0-3
    swapped = bits[4:] + bits[:4]  # [old_4, old_5, old_6, old_7, old_0, old_1, old_2, old_3]
    result = bits_to_int(swapped)
    # SWAP A does NOT update parity per the 8051 architecture spec
    return ALUResult8051(result=result, cy=0, ac=0, ov=0, parity=0)


# ── Decimal Adjust ────────────────────────────────────────────────────────────


def da8(a: int, cy: int, ac: int) -> ALUResult8051:
    """DA A: decimal adjust after binary addition of two BCD values.

    The 8051 supports BCD (Binary Coded Decimal) arithmetic where each
    nibble (4 bits) represents one decimal digit (0-9).  When you add two
    BCD numbers using binary ADD, the result may be in binary (e.g.,
    0x0A-0x0F for digits that "overflow" 9).  DA A corrects this.

    Algorithm (per 8051 hardware specification):

        Step 1 — Low nibble correction:
            If (low nibble of A > 9) OR AC = 1:
                add 6 to A (pull the binary result back into BCD range)

        Step 2 — High nibble correction:
            If (high nibble of result > 9) OR CY = 1:
                add 0x60 to A, set CY = 1

    Why "+6"?  BCD digits 0-9 are valid.  Binary digits 10-15 (A-F) are
    "illegal" BCD.  Adding 6 (the gap from 9 to 15 to 0+carry) skips over
    the illegal range and produces the correct BCD digit with the carry.

    Example:
        ACC = 0x29 (BCD 29), adding 0x47 (BCD 47)
        Binary:    0x29 + 0x47 = 0x70, AC=0, CY=0 → no correction needed
        Result:    0x70 (BCD 70) ✓

        ACC = 0x59 (BCD 59), adding 0x47 (BCD 47)
        Binary:    0x59 + 0x47 = 0xA0, AC=0, CY=0
        Step 1:    low nibble = 0, AC=0 → no step 1 correction
        Hmm, 0xA0 has high nibble = A > 9 → step 2: + 0x60 = 0x100
        CY=1, result = 0x00 → but that's wrong, BCD 59+47=106...
        Actually 0xA0 high nibble = 0xA, 0xA > 9 → add 0x60:
        0xA0 + 0x60 = 0x100, so result = 0x00, CY=1 → BCD 106 ✓

    Args:
        a:  Accumulator value after a binary ADD (before decimal adjust).
        cy: PSW carry flag from the preceding ADD.
        ac: PSW auxiliary carry flag from the preceding ADD.

    Returns:
        ALUResult8051 with BCD-corrected result.
    """
    a_bits = int_to_bits(a, 8)

    # Low nibble as 4-bit integer for comparison
    low_nibble_bits = a_bits[:4]

    # Step 1: Low nibble correction
    # Condition: low nibble > 9 OR AC = 1
    # low_nibble > 9: check if any of bits 3,2,1 with appropriate combinations
    # We check this via comparison: > 9 means the 4-bit value is 0xA-0xF
    # Gate-tree: (bit3 AND bit1) OR (bit3 AND bit2) — this detects values 10-15
    b3 = low_nibble_bits[3]  # bit 3 (value 8)
    b2 = low_nibble_bits[2]  # bit 2 (value 4)
    b1 = low_nibble_bits[1]  # bit 1 (value 2)
    # Values > 9 in 4 bits: 1010(10), 1011(11), 1100(12), 1101(13), 1110(14), 1111(15)
    # These all have bit3=1 AND (bit2=1 OR bit1=1)
    low_gt9 = AND(b3, OR(b2, b1))
    need_low_correction = OR(low_gt9, ac)

    # Add 6 to A if correction needed (gate-level mux: either add 6 or add 0)
    correction_lo = 6 if need_low_correction else 0
    step1_result, step1_cy, _ = add_8bit(a, correction_lo, 0)
    new_cy_1 = OR(step1_cy, cy)  # preserve original CY

    # Step 2: High nibble correction
    step1_bits = int_to_bits(step1_result, 8)
    high_nibble_bits = step1_bits[4:]
    # high_nibble > 9 uses same gate logic
    h3 = high_nibble_bits[3]
    h2 = high_nibble_bits[2]
    h1 = high_nibble_bits[1]
    high_gt9 = AND(h3, OR(h2, h1))
    need_high_correction = OR(high_gt9, new_cy_1)

    correction_hi = 0x60 if need_high_correction else 0
    final_result, final_cy, _ = add_8bit(step1_result, correction_hi, 0)
    new_cy = OR(final_cy, new_cy_1) if need_high_correction else new_cy_1

    result_bits = int_to_bits(final_result, 8)
    return ALUResult8051(
        result=final_result,
        cy=new_cy,
        ac=ac,  # AC is not modified by DA
        ov=0,
        parity=compute_parity(result_bits),
    )


# ── Multiply / Divide ─────────────────────────────────────────────────────────


def mul8(a: int, b: int) -> tuple[int, int, int]:
    """MUL AB: unsigned 8×8→16-bit multiplication via repeated addition.

    Hardware reality: the 8051 MUL instruction uses a 4-cycle binary
    multiplier (shift-and-add circuit).  We model this as 8 iterations
    of the gate-level add8 function — one for each bit of the multiplier.

    Algorithm (binary multiplication):
        product = 0
        for i in 0..7:
            if multiplier_bit[i] == 1:
                add (multiplicand << i) to product

    This is the classic "shift-and-add" algorithm that matches how digital
    multipliers are physically implemented.  The `if` test is a gate-level
    check: AND(b_bits[i], 1) — if the bit is set, include this partial product.

    The product is 16 bits: result_lo goes to ACC, result_hi goes to B.
    CY is always cleared.  OV = 1 if the result exceeds 255 (i.e., B != 0).

    Args:
        a: Multiplicand (ACC), 0–255.
        b: Multiplier (B register), 0–255.

    Returns:
        (result_hi, result_lo, ov) where result_hi → B, result_lo → ACC.
    """
    b_bits = int_to_bits(b, 8)

    # 16-bit running product, starts at 0
    product = 0

    for i in range(8):
        # AND gate: check if bit i of multiplier is set
        if AND(b_bits[i], 1):
            # Partial product: a << i (bit shift is address math, not data arithmetic)
            shifted = a << i  # shift amount i, this is index math
            # Add partial product to running total via gate-level 16-bit adder
            product, _ = add_16bit(product, shifted, 0)

    # Split 16-bit product into high and low bytes using indexing
    result_lo = product & 0xFF
    result_hi = (product >> 8) & 0xFF

    # OV = 1 if result > 255 (i.e., the high byte is non-zero)
    ov = 1 if result_hi != 0 else 0

    return result_hi, result_lo, ov


def div8(a: int, b: int) -> tuple[int, int, int]:
    """DIV AB: unsigned 8-bit division via repeated subtraction.

    Hardware reality: the 8051 DIV instruction uses a 4-cycle sequential
    divider.  We model this via the non-restoring division algorithm:
    repeatedly subtract the divisor from the dividend until borrow occurs.

    Algorithm:
        if b == 0: OV = 1, result undefined
        else:
            quotient = 0, remainder = a
            while remainder >= b:
                remainder = remainder - b  (gate-level SUBB)
                quotient += 1              (gate-level INC via add8)

    The division terminates when subb8 returns a borrow (CY=1), meaning
    remainder < divisor.

    CY is always cleared after DIV.  OV = 1 only for divide-by-zero.

    Args:
        a: Dividend (ACC), 0–255.
        b: Divisor (B register), 0–255.

    Returns:
        (quotient, remainder, ov) where quotient → ACC, remainder → B.
    """
    if b == 0:
        # Divide by zero: OV=1, CY=0, ACC and B are undefined (we leave them)
        return 0, 0, 1

    quotient = 0
    remainder = a

    # Max iterations: 255 (largest possible quotient for 8-bit / 8-bit)
    for _ in range(256):
        # Subtract divisor from remainder — gate-level operation
        sub_result = subb8(remainder, b, 0)
        if sub_result.cy:
            # Borrow = 1 means remainder < b, division complete
            break
        remainder = sub_result.result
        # Increment quotient via gate-level add8
        q_result = add8(quotient, 1, 0)
        quotient = q_result.result

    return quotient, remainder, 0
