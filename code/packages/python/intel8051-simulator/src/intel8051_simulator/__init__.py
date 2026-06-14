"""Intel 8051 (MCS-51, 1980) behavioral simulator — Layer 07p.

Public exports:
    I8051Simulator  — the simulator class (implements Simulator[I8051State])
    I8051State      — frozen CPU state snapshot dataclass
"""

from .simulator import I8051Simulator
from .state import I8051State

__all__ = ["I8051Simulator", "I8051State"]
