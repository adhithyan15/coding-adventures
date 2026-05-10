"""Motorola 68000 (1979) behavioral simulator — Layer 07n.

Public API
----------
``M68KSimulator``
    The main simulator class.  Implements ``Simulator[M68KState]``
    from ``simulator_protocol`` (SIM00).

``M68KState``
    Frozen dataclass snapshot of the 68000 CPU state at any instant.
    Fields: D0–D7 (data registers), A0–A7 (address registers, A7=SSP),
    PC (program counter), SR (status register), halted, memory (16 MB).
    Properties: .x/.n/.z/.v/.c (CCR bits), .d/.a (register tuples).

Quick start
-----------
>>> from motorola_68000_simulator import M68KSimulator
>>> sim = M68KSimulator()
>>> prog = bytes([
...     0x70, 0x0A,              # MOVEQ #10, D0
...     0x72, 0x14,              # MOVEQ #20, D1
...     0xD0, 0x81,              # ADD.L D1, D0
...     0x4E, 0x72, 0x27, 0x00, # STOP #0x2700
... ])
>>> result = sim.execute(prog)
>>> result.ok
True
>>> result.final_state.d0
30
"""

from motorola_68000_simulator.simulator import M68KSimulator
from motorola_68000_simulator.state import M68KState

__all__ = ["M68KSimulator", "M68KState"]
