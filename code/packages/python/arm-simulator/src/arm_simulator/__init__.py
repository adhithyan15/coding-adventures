"""ARM Simulator — Layer 4 of the computing stack.

ARMv7 instruction decoder and executor.
Plugs into the CPU simulator via the decoder/executor protocol.
"""

from arm_simulator.simulator import ARMSimulator
from arm_simulator.state import ARMState

__all__ = ["ARMSimulator", "ARMState"]
