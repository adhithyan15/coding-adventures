"""Manchester Baby (SSEM, 1948) behavioral simulator — Layer 07l.

Public API
----------
``BabySimulator``
    The main simulator class.  Implements the ``Simulator[BabyState]``
    protocol from ``simulator_protocol``.

``BabyState``
    Frozen dataclass snapshot of the machine state at any instant.
    Fields: ``store`` (32 words), ``accumulator``, ``ci``, ``halted``.
    Properties: ``acc_signed``, ``present_instruction``.

Quick start
-----------
>>> from manchester_baby_simulator import BabySimulator
>>> sim = BabySimulator()
>>> # LDN 0 (F=010, S=0) → opcode = (0b010 << 13) | 0 = 0x4000
>>> LDN_0 = (0b010 << 13)
>>> STO_1 = (0b011 << 13) | 1
>>> STP   = (0b111 << 13)
>>> def w(v): return v.to_bytes(4, 'little')
>>> prog = w(42) + w(0) + w(LDN_0) + w(STO_1) + w(STP)
>>> result = sim.execute(prog)
>>> result.ok
True
>>> result.final_state.store[1]  # −42 in unsigned two's complement
4294967254
>>> result.final_state.acc_signed
-42
"""

from manchester_baby_simulator.simulator import BabySimulator
from manchester_baby_simulator.state import BabyState

__all__ = ["BabySimulator", "BabyState"]
