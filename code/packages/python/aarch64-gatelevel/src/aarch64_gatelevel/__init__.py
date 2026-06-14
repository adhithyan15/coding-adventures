"""aarch64_gatelevel — AArch64 (ARMv8-A, 2011) gate-level simulator.

Layer 07v2 of the coding-adventures architecture simulator stack.

All integer data-path operations route through logic gate primitives
(AND, OR, XOR, NOT) and ripple_carry_adder from the logic_gates and
arithmetic packages.

Public API
──────────
  AArch64GateLevelSimulator : Simulator[AArch64State]
      The main gate-level simulator.

  RegisterFile              : bit-list register file
  decode()                  : combinational instruction decoder
  ALUResult64               : ALU result dataclass (result + NZCV)

  bits module               : bit-list conversion and gate-level arithmetic
  alu module                : ALU operations (add64, sub64, and64, etc.)
"""

from .alu import ALUResult64
from .decoder import AArch64Instruction, decode
from .register_file import RegisterFile
from .simulator import AArch64GateLevelSimulator

__all__ = [
    "AArch64GateLevelSimulator",
    "RegisterFile",
    "decode",
    "AArch64Instruction",
    "ALUResult64",
]
