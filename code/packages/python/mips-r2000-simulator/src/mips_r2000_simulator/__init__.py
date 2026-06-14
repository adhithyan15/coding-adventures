"""MIPS R2000 (1985) behavioral simulator — Layer 07q.

Public exports:
    MIPSSimulator  — the simulator class (implements Simulator[MIPSState])
    MIPSState      — frozen CPU state snapshot dataclass
"""

from .simulator import MIPSSimulator
from .state import MIPSState

__all__ = ["MIPSSimulator", "MIPSState"]
