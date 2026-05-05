"""alpha_axp_simulator — DEC Alpha AXP 21064 (1992) behavioral simulator.

Layer 07s in the historical CPU simulator series.

Public API:
  AlphaSimulator  — implements Simulator[AlphaState] (SIM00 protocol)
  AlphaState      — frozen dataclass: pc, npc, regs, memory, halted
"""

from __future__ import annotations

from .simulator import AlphaSimulator
from .state import AlphaState

__all__ = ["AlphaSimulator", "AlphaState"]
