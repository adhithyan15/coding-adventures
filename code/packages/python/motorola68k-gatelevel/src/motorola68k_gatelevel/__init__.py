"""Motorola 68000 gate-level simulator.

Every ALU operation (ADD, SUB, AND, OR, XOR, NOT, shifts, rotates) routes
through logic gate primitives from the ``logic-gates`` and ``arithmetic``
packages.  No Python integer arithmetic is used in the critical ALU path.

Cross-validates against the behavioral ``motorola-68000-simulator`` package.

Quick start::

    from motorola68k_gatelevel.simulator import Motorola68kGateLevelSimulator

    sim = Motorola68kGateLevelSimulator()
    prog = bytes([
        0x70, 0x05,              # MOVEQ #5, D0
        0x72, 0x03,              # MOVEQ #3, D1
        0xD0, 0x81,              # ADD.L D1, D0
        0x4E, 0x72, 0x27, 0x00, # STOP #0x2700
    ])
    result = sim.execute(prog)
    print(result.final_state.d0)  # 8
"""

from motorola68k_gatelevel.simulator import Motorola68kGateLevelSimulator

__all__ = ["Motorola68kGateLevelSimulator"]
__version__ = "1.0.0"
