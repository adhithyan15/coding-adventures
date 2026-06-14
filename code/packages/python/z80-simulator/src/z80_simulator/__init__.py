"""Zilog Z80 behavioral simulator — Layer 07k.

Exports the public API: Z80Simulator and Z80State.

Example::

    from z80_simulator import Z80Simulator

    sim = Z80Simulator()
    result = sim.execute(bytes([
        0x3E, 0x0A,   # LD A, 10
        0xC6, 0x05,   # ADD A, 5
        0x76,         # HALT
    ]))
    assert result.final_state.a == 15
"""

from z80_simulator.simulator import Z80Simulator
from z80_simulator.state import Z80State

__all__ = ["Z80Simulator", "Z80State"]
