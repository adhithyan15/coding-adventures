"""Intel 4004 Gate-Level Simulator — every operation routes through real logic gates.

All computation flows through: NOT/AND/OR/XOR → half_adder → full_adder →
ripple_carry_adder → ALU, and state is stored in D flip-flop registers.
"""

from intel4004_gatelevel.cpu import GateTrace, Intel4004GateLevel

__all__ = ["Intel4004GateLevel", "GateTrace"]
