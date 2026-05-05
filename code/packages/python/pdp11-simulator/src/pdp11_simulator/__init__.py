"""DEC PDP-11 (1970) behavioral simulator — Layer 07o.

Public API
----------
``PDP11Simulator``
    The main simulator class.  Implements ``Simulator[PDP11State]``
    from ``simulator_protocol`` (SIM00).

``PDP11State``
    Frozen dataclass snapshot of the PDP-11 CPU state at any instant.
    Fields: r (R0–R7 tuple), psw (Processor Status Word), halted, memory.
    Properties: .n/.z/.v/.c (condition code flags).

Quick start
-----------
>>> from pdp11_simulator import PDP11Simulator
>>> sim = PDP11Simulator()
>>> # MOV #5, R0  (immediate to register)
>>> # Encoded as: MOV opcode=0x15C0, immediate word=0x0005, then HALT=0x0000
>>> prog = bytes([
...     0xC0, 0x15,   # MOV #n, R0  (mode 2/R7 src, mode 0/R0 dst)
...     0x05, 0x00,   # immediate: 5
...     0x00, 0x00,   # HALT
... ])
>>> result = sim.execute(prog)
>>> result.ok
True
>>> result.final_state.r[0]
5
"""

from pdp11_simulator.simulator import PDP11Simulator
from pdp11_simulator.state import PDP11State

__all__ = ["PDP11Simulator", "PDP11State"]
