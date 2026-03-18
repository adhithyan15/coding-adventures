"""Arithmetic Logic Unit (ALU) — the computational heart of a CPU.

Takes two N-bit inputs and an operation code, produces an N-bit result
plus status flags. Built from adders and logic gates.
"""

from dataclasses import dataclass
from enum import Enum

from logic_gates import AND, NOT, OR, XOR

from arithmetic.adders import ripple_carry_adder


class ALUOp(Enum):
    """ALU operation codes."""

    ADD = "add"
    SUB = "sub"
    AND = "and"
    OR = "or"
    XOR = "xor"
    NOT = "not"


@dataclass
class ALUResult:
    """Result of an ALU operation."""

    value: list[int]  # Result bits (LSB first)
    zero: bool  # Is result all zeros?
    carry: bool  # Did addition overflow?
    negative: bool  # Is MSB 1? (sign bit in two's complement)
    overflow: bool  # Signed overflow occurred?


def _bitwise_op(
    a: list[int], b: list[int], op: type[AND] | type[OR] | type[XOR]
) -> list[int]:
    """Apply a 2-input gate bitwise across two bit lists."""
    return [op(a[i], b[i]) for i in range(len(a))]


def _twos_complement_negate(bits: list[int]) -> tuple[list[int], int]:
    """Negate a number in two's complement: NOT(bits) + 1."""
    inverted = [NOT(b) for b in bits]
    one = [1] + [0] * (len(bits) - 1)
    return ripple_carry_adder(inverted, one)


class ALU:
    """N-bit Arithmetic Logic Unit."""

    def __init__(self, bit_width: int = 8) -> None:
        if bit_width < 1:
            msg = "bit_width must be at least 1"
            raise ValueError(msg)
        self.bit_width = bit_width

    def execute(self, op: ALUOp, a: list[int], b: list[int]) -> ALUResult:
        """Execute an ALU operation on two N-bit inputs.

        Args:
            op: The operation to perform.
            a: First operand as bits (LSB first), length must equal bit_width.
            b: Second operand as bits (LSB first), length must equal bit_width.
               Ignored for NOT operation.

        Returns:
            ALUResult with value, zero, carry, negative, and overflow flags.
        """
        if len(a) != self.bit_width:
            msg = f"a must have {self.bit_width} bits, got {len(a)}"
            raise ValueError(msg)
        if op != ALUOp.NOT and len(b) != self.bit_width:
            msg = f"b must have {self.bit_width} bits, got {len(b)}"
            raise ValueError(msg)

        carry = False

        if op == ALUOp.ADD:
            value, carry_bit = ripple_carry_adder(a, b)
            carry = carry_bit == 1

        elif op == ALUOp.SUB:
            # A - B = A + NOT(B) + 1 (two's complement subtraction)
            neg_b, _ = _twos_complement_negate(b)
            value, carry_bit = ripple_carry_adder(a, neg_b)
            carry = carry_bit == 1

        elif op == ALUOp.AND:
            value = _bitwise_op(a, b, AND)

        elif op == ALUOp.OR:
            value = _bitwise_op(a, b, OR)

        elif op == ALUOp.XOR:
            value = _bitwise_op(a, b, XOR)

        elif op == ALUOp.NOT:
            value = [NOT(bit) for bit in a]

        else:
            msg = f"Unknown operation: {op}"
            raise ValueError(msg)

        zero = all(bit == 0 for bit in value)
        negative = value[-1] == 1 if value else False

        # Signed overflow: occurs when adding two positive numbers gives
        # negative, or two negative numbers gives positive.
        overflow = False
        if op in (ALUOp.ADD, ALUOp.SUB):
            a_sign = a[-1]
            b_sign = b[-1] if op == ALUOp.ADD else NOT(b[-1])
            result_sign = value[-1]
            overflow = (a_sign == b_sign) and (result_sign != a_sign)

        return ALUResult(
            value=value,
            zero=zero,
            carry=carry,
            negative=negative,
            overflow=overflow,
        )
