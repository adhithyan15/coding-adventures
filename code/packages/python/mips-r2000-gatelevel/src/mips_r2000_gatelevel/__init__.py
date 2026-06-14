"""mips_r2000_gatelevel — MIPS R2000 (1985) gate-level simulator — Layer 07q2.

Every ALU operation (ADD, SUB, AND, OR, XOR, NOT, SLT, shifts, MULT, DIV)
routes through logic gate primitives from the ``logic_gates`` package and the
``ripple_carry_adder`` from the ``arithmetic`` package.

This is behaviorally equivalent to ``mips_r2000_simulator`` (the behavioral
simulator) but models the gate-level data path explicitly.

Public exports
──────────────
    MIPSR2000GateLevelSimulator  — the main simulator class
    ALUResult32                  — result type for ALU operations
    RegisterFile32               — the register file (bit-array storage)
    decode_instruction           — instruction decoder
"""

from .alu import ALUResult32
from .decoder import decode_instruction
from .register_file import RegisterFile32
from .simulator import MIPSR2000GateLevelSimulator

__all__ = [
    "MIPSR2000GateLevelSimulator",
    "ALUResult32",
    "RegisterFile32",
    "decode_instruction",
]
