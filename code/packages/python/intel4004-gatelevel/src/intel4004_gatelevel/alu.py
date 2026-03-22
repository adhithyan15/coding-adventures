"""4-bit ALU — the arithmetic heart of the Intel 4004.

=== How the real 4004's ALU worked ===

The Intel 4004 had a 4-bit ALU that could add, subtract, and perform
logical operations on 4-bit values. It used a ripple-carry adder built
from full adders, which were themselves built from AND, OR, and XOR gates.

This module wraps the arithmetic package's ALU(bit_width=4) to provide
the exact operations the 4004 needs. Every addition and subtraction
physically routes through the gate chain:

    XOR → AND → OR → full_adder → ripple_carry_adder → ALU

That's real hardware simulation — not behavioral shortcuts.

=== Subtraction via complement-add ===

The 4004 doesn't have a dedicated subtractor. Instead, it uses the
ones' complement method:

    A - B = A + NOT(B) + borrow_in

where borrow_in = 0 if carry_flag else 1 (inverted carry semantics).
The ALU's SUB operation does this internally using NOT gates to
complement B, then feeding through the same adder.
"""

from __future__ import annotations

from arithmetic import ALU, ALUOp

from intel4004_gatelevel.bits import bits_to_int, int_to_bits


class GateALU:
    """4-bit ALU for the Intel 4004 gate-level simulator.

    All operations route through real logic gates via the arithmetic
    package's ALU class. No behavioral shortcuts.

    The ALU provides:
        - add(a, b, carry_in) → (result, carry_out)
        - subtract(a, b, borrow_in) → (result, carry_out)
        - complement(a) → result (4-bit NOT)
        - increment(a) → (result, carry_out)
        - decrement(a) → (result, borrow_out)
    """

    def __init__(self) -> None:
        """Create a 4-bit ALU using real logic gates."""
        self._alu = ALU(bit_width=4)

    def add(
        self, a: int, b: int, carry_in: int = 0
    ) -> tuple[int, bool]:
        """Add two 4-bit values with carry.

        Routes through: XOR → AND → OR → full_adder × 4 → ripple_carry

        Args:
            a: First operand (0–15).
            b: Second operand (0–15).
            carry_in: Carry from previous operation (0 or 1).

        Returns:
            (result, carry_out) where result is 4-bit (0–15).
        """
        a_bits = int_to_bits(a, 4)
        b_bits = int_to_bits(b, 4)

        if carry_in:
            # Add carry_in by first adding a+b, then adding 1
            # This simulates the carry input to the LSB full adder
            result1 = self._alu.execute(ALUOp.ADD, a_bits, b_bits)
            one_bits = int_to_bits(1, 4)
            result2 = self._alu.execute(
                ALUOp.ADD, result1.value, one_bits
            )
            # Carry is set if either addition overflowed
            carry = result1.carry or result2.carry
            return bits_to_int(result2.value), carry
        else:
            result = self._alu.execute(ALUOp.ADD, a_bits, b_bits)
            return bits_to_int(result.value), result.carry

    def subtract(
        self, a: int, b: int, borrow_in: int = 0
    ) -> tuple[int, bool]:
        """Subtract using complement-add: A + NOT(B) + borrow_in.

        The 4004's carry flag semantics for subtraction:
            carry=True  → no borrow (result >= 0)
            carry=False → borrow occurred

        Args:
            a: Minuend (0–15).
            b: Subtrahend (0–15).
            borrow_in: 1 if no previous borrow, 0 if borrow.

        Returns:
            (result, carry_out) where carry_out=True means no borrow.
        """
        # Complement b using NOT gates
        b_bits = int_to_bits(b, 4)
        b_comp = self._alu.execute(ALUOp.NOT, b_bits, b_bits)
        # A + NOT(B) + borrow_in
        return self.add(a, bits_to_int(b_comp.value), borrow_in)

    def complement(self, a: int) -> int:
        """4-bit NOT: invert all bits using NOT gates.

        Args:
            a: Value to complement (0–15).

        Returns:
            Complemented value (0–15).
        """
        a_bits = int_to_bits(a, 4)
        result = self._alu.execute(ALUOp.NOT, a_bits, a_bits)
        return bits_to_int(result.value)

    def increment(self, a: int) -> tuple[int, bool]:
        """Increment by 1 using the adder. Returns (result, carry)."""
        return self.add(a, 1, 0)

    def decrement(self, a: int) -> tuple[int, bool]:
        """Decrement by 1 using complement-add.

        A - 1 = A + NOT(1) + 1 = A + 14 + 1 = A + 15.
        carry=True if A > 0 (no borrow), False if A == 0.
        """
        return self.subtract(a, 1, 1)

    def bitwise_and(self, a: int, b: int) -> int:
        """4-bit AND using AND gates."""
        a_bits = int_to_bits(a, 4)
        b_bits = int_to_bits(b, 4)
        result = self._alu.execute(ALUOp.AND, a_bits, b_bits)
        return bits_to_int(result.value)

    def bitwise_or(self, a: int, b: int) -> int:
        """4-bit OR using OR gates."""
        a_bits = int_to_bits(a, 4)
        b_bits = int_to_bits(b, 4)
        result = self._alu.execute(ALUOp.OR, a_bits, b_bits)
        return bits_to_int(result.value)

    @property
    def gate_count(self) -> int:
        """Estimated gate count for a 4-bit ALU.

        Each full adder: 5 gates (2 XOR + 2 AND + 1 OR).
        4-bit ripple carry: 4 × 5 = 20 gates.
        SUB complement: 4 NOT gates.
        Control muxing: ~8 gates.
        Total: ~32 gates.
        """
        return 32
