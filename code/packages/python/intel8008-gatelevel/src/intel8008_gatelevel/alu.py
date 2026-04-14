"""8-bit ALU — the arithmetic heart of the Intel 8008 gate-level simulator.

=== How the real 8008's ALU worked ===

The Intel 8008 had an 8-bit ALU — twice the word width of its predecessor
the 4004. This doubling required exactly twice the transistors in the
arithmetic unit: 8 full adders instead of 4, 8 NOT gates for complement,
8 AND gates for bitwise AND, etc.

This module wraps the arithmetic package's `ALU(bit_width=8)` to provide
all the operations the 8008 needs. Every addition and subtraction routes
through the full gate chain:

    XOR/AND → half_adder → full_adder × 8 → ripple_carry_adder

That's real hardware simulation — not behavioral shortcuts.

=== Gate count for the 8-bit ALU ===

Component               Gates       Notes
─────────────────────   ─────       ─────
8-bit ripple-carry add  8 × 5 = 40  8 full adders, each: 2 XOR + 2 AND + 1 OR
8-bit NOT (for SUB)     8           complement B for two's complement subtraction
8-bit AND               8           bitwise AND
8-bit OR                8           bitwise OR
8-bit XOR               8           bitwise XOR
Parity (XOR tree)       8           7 XOR gates + 1 NOT (via XOR_N)
Zero detection          ~8          NOR tree (NOT + OR chain)
─────────────────────   ─────       ─────
Total ALU               ~88 gates

Compare to the 4004's 4-bit ALU at ~32 gates — the 8008 uses ~2.75× more
gates, reflecting the increased data width and the addition of parity.

=== Subtraction via two's complement ===

The 8008 doesn't have a separate subtractor. Like the 4004, it uses the
two's complement method:

    A - B = A + NOT(B) + 1

The +1 handles the two's complement convention. This is implemented by:
1. Inverting all bits of B via 8 NOT gates
2. Running through the same ripple-carry adder with carry_in = 1

For SBB (subtract with borrow): A - B - CY
    = A + NOT(B) + (1 - CY)
    = A + NOT(B) + NOT(CY)

So borrow-in is the COMPLEMENT of the carry flag. CY=0 means borrow
occurred, which is confusing but matches the 8008 hardware convention.
"""

from __future__ import annotations

from dataclasses import dataclass

from arithmetic import ALU, ALUOp
from logic_gates import NOT, OR_N

from intel8008_gatelevel.bits import bits_to_int, compute_parity, int_to_bits


@dataclass
class Intel8008FlagBits:
    """Flag bits computed by the ALU from an 8-bit result.

    These are the raw gate outputs before being stored in the flag register.

    zero:   1 if all 8 result bits are 0 (NOR of all bits)
    sign:   1 if bit 7 of result is 1 (direct wire from MSB)
    parity: 1 if even number of 1-bits (XOR tree + invert)
    carry:  1 if addition overflowed / subtraction borrowed
    """

    zero: int    # 0 or 1
    sign: int    # 0 or 1
    parity: int  # 0 or 1
    carry: int   # 0 or 1

    def to_bools(self) -> tuple[bool, bool, bool, bool]:
        """Convert to (carry, zero, sign, parity) booleans."""
        return bool(self.carry), bool(self.zero), bool(self.sign), bool(self.parity)


class GateALU8:
    """8-bit ALU for the Intel 8008 gate-level simulator.

    All operations route through real logic gates via the arithmetic
    package's ALU class. No behavioral shortcuts.

    The ALU provides the complete set of 8008 arithmetic/logical operations:
        add, subtract (with/without carry/borrow)
        bitwise and, or, xor
        compare (subtract without storing result)
        increment, decrement
        rotate left/right (circular and through carry)

    Usage:
        >>> alu = GateALU8()
        >>> result, carry = alu.add(3, 4)
        >>> result
        7
        >>> carry
        False
    """

    def __init__(self) -> None:
        """Create an 8-bit ALU backed by real logic gates."""
        self._alu = ALU(bit_width=8)

    def compute_flags(self, result: int, carry: bool) -> Intel8008FlagBits:
        """Compute all 4 flags from an 8-bit result using gate functions.

        This models the flag computation circuit in the real 8008:
            zero   = NOT(OR(b0, OR(b1, OR(b2, ...)))) — NOR all bits
            sign   = b7 (direct wire to MSB)
            parity = NOT(XOR(b0, XOR(b1, XOR(..., b7)))) — XOR tree + invert
            carry  = carry_out from adder

        Args:
            result: 8-bit result value (0–255).
            carry:  Carry/borrow bit from the operation.

        Returns:
            Intel8008FlagBits with all flags computed via gates.
        """
        r8 = result & 0xFF
        result_bits = int_to_bits(r8, 8)

        # Zero flag: NOR reduction of all 8 result bits.
        # In hardware: a tree of NOR gates reduces 8 bits to 1.
        # Here: OR_N(all bits) then NOT.
        zero = NOT(OR_N(*result_bits)) if any(result_bits) else 1

        # Sign flag: bit 7 directly — no computation needed.
        # This is a wire, not a gate. The MSB IS the sign bit.
        sign = result_bits[7]

        # Parity flag: XOR reduction of all 8 bits, then inverted.
        # P=1 means even parity; P=0 means odd parity.
        parity = compute_parity(result_bits)

        # Carry flag: from the adder's carry_out.
        carry_bit = 1 if carry else 0

        return Intel8008FlagBits(
            zero=zero,
            sign=sign,
            parity=parity,
            carry=carry_bit,
        )

    def add(self, a: int, b: int, carry_in: bool = False) -> tuple[int, bool]:
        """Add two 8-bit values with carry.

        Routes through: XOR → AND → OR → full_adder × 8 → ripple_carry

        For carry_in: we simulate the initial carry-in to the LSB adder
        by performing two passes (a+b, then +1 if carry_in). This models
        the ADC instruction's behavior with two ripple-carry adder executions.

        Args:
            a:        First operand (0–255).
            b:        Second operand (0–255).
            carry_in: True to add an extra 1 (for ADC instruction).

        Returns:
            (result, carry_out) where result is 0–255.
        """
        a_bits = int_to_bits(a, 8)
        b_bits = int_to_bits(b, 8)

        if carry_in:
            # Two-pass carry_in simulation (matches 4004 gate-level pattern)
            r1 = self._alu.execute(ALUOp.ADD, a_bits, b_bits)
            one_bits = int_to_bits(1, 8)
            r2 = self._alu.execute(ALUOp.ADD, r1.value, one_bits)
            carry = r1.carry or r2.carry
            return bits_to_int(r2.value), carry
        else:
            result = self._alu.execute(ALUOp.ADD, a_bits, b_bits)
            return bits_to_int(result.value), result.carry

    def subtract(self, a: int, b: int, borrow_in: bool = False) -> tuple[int, bool]:
        """Subtract b from a using two's complement: A + NOT(B) + 1.

        The 8008's carry flag semantics for subtraction:
            carry/borrow=True  → borrow occurred (unsigned a < b)
            carry/borrow=False → no borrow (unsigned a >= b)

        This is the INVERSE of the carry convention for addition! On the 8008,
        SUB always sets CY=1 if a borrow was needed. Hardware reason: the
        complement-add produces carry_out=0 when a borrow occurs (because NOT(b)
        makes the subtraction look like an addition that fails to overflow).

        For SBB (subtract with borrow): borrow_in=True means the previous
        operation produced a borrow, so we subtract one extra.

        Args:
            a:          Minuend (0–255).
            b:          Subtrahend (0–255).
            borrow_in:  True if previous operation borrowed (SBB).

        Returns:
            (result, borrow_occurred) where borrow_occurred=True means CY=1.
        """
        b_bits = int_to_bits(b, 8)
        # Complement B via 8 NOT gates — this is the two's complement step
        b_comp = self._alu.execute(ALUOp.NOT, b_bits, b_bits)
        b_comp_int = bits_to_int(b_comp.value)

        # A + NOT(B) + 1 (the +1 is always needed for two's complement)
        # For SBB: A + NOT(B) + NOT(CY) = A + NOT(B) + 0 when CY=1
        # Standard SUB: carry_in=True (adds the +1 for twos complement)
        # SBB with borrow: carry_in=False (borrow eats the +1)
        carry_in_for_add = not borrow_in  # Standard SUB: True; SBB with borrow: False

        result, carry_from_add = self.add(a, b_comp_int, carry_in_for_add)

        # Borrow convention: borrow_occurred = NOT(carry_from_add)
        # When carry_from_add=1 (no borrow), borrow_occurred=False
        # When carry_from_add=0 (borrow occurred), borrow_occurred=True
        borrow_occurred = not carry_from_add
        return result, borrow_occurred

    def bitwise_and(self, a: int, b: int) -> int:
        """8-bit AND via 8 AND gates. Clears carry (ANA convention).

        Args:
            a: First operand (0–255).
            b: Second operand (0–255).

        Returns:
            a & b (0–255).
        """
        a_bits = int_to_bits(a, 8)
        b_bits = int_to_bits(b, 8)
        result = self._alu.execute(ALUOp.AND, a_bits, b_bits)
        return bits_to_int(result.value)

    def bitwise_or(self, a: int, b: int) -> int:
        """8-bit OR via 8 OR gates. Clears carry (ORA convention).

        Args:
            a: First operand (0–255).
            b: Second operand (0–255).

        Returns:
            a | b (0–255).
        """
        a_bits = int_to_bits(a, 8)
        b_bits = int_to_bits(b, 8)
        result = self._alu.execute(ALUOp.OR, a_bits, b_bits)
        return bits_to_int(result.value)

    def bitwise_xor(self, a: int, b: int) -> int:
        """8-bit XOR via 8 XOR gates. Clears carry (XRA convention).

        Args:
            a: First operand (0–255).
            b: Second operand (0–255).

        Returns:
            a ^ b (0–255).
        """
        a_bits = int_to_bits(a, 8)
        b_bits = int_to_bits(b, 8)
        result = self._alu.execute(ALUOp.XOR, a_bits, b_bits)
        return bits_to_int(result.value)

    def compare(self, a: int, b: int) -> Intel8008FlagBits:
        """Compare a and b (A − B), return flags. Result is discarded.

        Identical to subtract() internally, but only the flag bits matter.
        The accumulator is NOT updated.

        Args:
            a: Minuend (0–255), from accumulator.
            b: Subtrahend (0–255), from register or immediate.

        Returns:
            Intel8008FlagBits describing the comparison result.
        """
        result, borrow = self.subtract(a, b, False)
        return self.compute_flags(result, borrow)

    def increment(self, a: int) -> tuple[int, bool]:
        """Increment by 1. Returns (result, carry_out).

        Used by INR. Note: INR does NOT update the carry flag in the 8008 —
        the carry_out returned here is used only for flag computation
        (Z, S, P are updated; CY is preserved). The CPU handles this
        by passing update_carry=False when applying the flags.

        Args:
            a: Value to increment (0–255).

        Returns:
            (result, carry) where carry is True if A was 0xFF (wrapped to 0).
        """
        return self.add(a, 1, False)

    def decrement(self, a: int) -> tuple[int, bool]:
        """Decrement by 1. Returns (result, borrow_occurred).

        Uses subtract(a, 1). Like INR, DCR does NOT update CY in the 8008.

        Args:
            a: Value to decrement (0–255).

        Returns:
            (result, borrow) where borrow=True if A was 0x00 (wrapped to 0xFF).
        """
        return self.subtract(a, 1, False)

    def rotate_left_circular(self, a: int) -> tuple[int, bool]:
        """Rotate A left circular (RLC).

        CY ← A[7]; A ← (A << 1) | A[7]

        The most significant bit wraps around to become the new LSB,
        and also sets the carry flag. Implemented via bit rewiring:
        no arithmetic gates needed — just shift the bit list.

        Args:
            a: Accumulator value (0–255).

        Returns:
            (new_a, new_carry) where new_carry = old A[7].
        """
        a_bits = int_to_bits(a, 8)
        # Rewire: new_bits[0] = old_bits[7], new_bits[i+1] = old_bits[i]
        new_bits = [a_bits[7]] + a_bits[0:7]
        carry = bool(a_bits[7])
        return bits_to_int(new_bits), carry

    def rotate_right_circular(self, a: int) -> tuple[int, bool]:
        """Rotate A right circular (RRC).

        CY ← A[0]; A ← (A >> 1) | (A[0] << 7)

        Args:
            a: Accumulator value (0–255).

        Returns:
            (new_a, new_carry) where new_carry = old A[0].
        """
        a_bits = int_to_bits(a, 8)
        # Rewire: new_bits[7] = old_bits[0], new_bits[i] = old_bits[i+1]
        new_bits = a_bits[1:8] + [a_bits[0]]
        carry = bool(a_bits[0])
        return bits_to_int(new_bits), carry

    def rotate_left_carry(self, a: int, carry_in: bool) -> tuple[int, bool]:
        """Rotate A left through carry (RAL) — 9-bit rotation.

        new_CY ← A[7]; A ← (A << 1) | old_CY

        The 9 bits (CY + 8 data bits) rotate together as a ring. The carry
        flag participates as the 9th bit. This allows double-precision
        shifting: use RAL on low byte then high byte to shift a 16-bit value.

        Args:
            a:        Accumulator value (0–255).
            carry_in: Current carry flag (becomes new A[0]).

        Returns:
            (new_a, new_carry) where new_carry = old A[7].
        """
        a_bits = int_to_bits(a, 8)
        new_bit0 = 1 if carry_in else 0
        new_bits = [new_bit0] + a_bits[0:7]
        carry = bool(a_bits[7])
        return bits_to_int(new_bits), carry

    def rotate_right_carry(self, a: int, carry_in: bool) -> tuple[int, bool]:
        """Rotate A right through carry (RAR) — 9-bit rotation.

        new_CY ← A[0]; A ← (old_CY << 7) | (A >> 1)

        Args:
            a:        Accumulator value (0–255).
            carry_in: Current carry flag (becomes new A[7]).

        Returns:
            (new_a, new_carry) where new_carry = old A[0].
        """
        a_bits = int_to_bits(a, 8)
        new_bit7 = 1 if carry_in else 0
        new_bits = a_bits[1:8] + [new_bit7]
        carry = bool(a_bits[0])
        return bits_to_int(new_bits), carry

    @property
    def gate_count(self) -> int:
        """Estimated gate count for the 8-bit ALU.

        Ripple-carry adder: 8 × 5 = 40 gates
        NOT for complement:  8 gates
        AND:                 8 gates
        OR:                  8 gates
        XOR:                 8 gates
        Parity tree:        ~8 gates
        Zero NOR tree:      ~8 gates
        Control muxing:    ~10 gates
        Total:             ~98 gates
        """
        return 98
