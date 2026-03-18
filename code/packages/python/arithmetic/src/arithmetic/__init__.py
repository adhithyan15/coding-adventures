"""Arithmetic — Layer 2 of the computing stack.

Half adder, full adder, ripple carry adder, and ALU.
Built entirely from logic gates (Layer 1).
"""

from arithmetic.adders import full_adder, half_adder, ripple_carry_adder
from arithmetic.alu import ALU, ALUOp, ALUResult

__all__ = [
    "half_adder",
    "full_adder",
    "ripple_carry_adder",
    "ALU",
    "ALUOp",
    "ALUResult",
]
