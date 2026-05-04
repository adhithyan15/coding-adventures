"""MOS 6502 behavioral simulator — Layer 07j.

Provides a complete, accurate simulator for the MOS Technology 6502
(NMOS) microprocessor. All 151 official opcodes, all 13 addressing modes,
BCD decimal-mode arithmetic, and the famous indirect JMP page-wrap bug.

Implements the SIM00 Simulator[MOS6502State] protocol from
``simulator-protocol``.

Quick start::

    from mos6502_simulator import MOS6502Simulator

    sim = MOS6502Simulator()
    result = sim.execute(bytes([
        0xA9, 0x0A,   # LDA #10
        0x69, 0x05,   # ADC #5
        0x00,         # BRK (halt)
    ]))
    assert result.final_state.a == 15
"""

from __future__ import annotations

from mos6502_simulator.flags import (
    bcd_add,
    bcd_sub,
    compute_nz,
    compute_overflow_add,
    compute_overflow_sub,
    pack_p,
    unpack_p,
)
from mos6502_simulator.simulator import MOS6502Simulator
from mos6502_simulator.state import MOS6502State

__all__ = [
    "MOS6502Simulator",
    "MOS6502State",
    # Flag helpers (useful for test assertions)
    "compute_nz",
    "compute_overflow_add",
    "compute_overflow_sub",
    "pack_p",
    "unpack_p",
    "bcd_add",
    "bcd_sub",
]
