"""SPARC V8 (1987) behavioral simulator — Layer 07r.

Public exports:
    SPARCSimulator  — the simulator class (implements Simulator[SPARCState])
    SPARCState      — frozen CPU state snapshot dataclass
"""

from .simulator import SPARCSimulator
from .state import SPARCState

__all__ = ["SPARCSimulator", "SPARCState"]
