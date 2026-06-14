"""MOS 6502 gate-level simulator.

Every data-path operation routes through AND, OR, XOR, NOT, and
ripple_carry_adder primitives — no Python integer arithmetic on the
execution path.

Public API::

    from mos6502_gatelevel import MOS6502GateLevelSimulator
    from mos6502_simulator import MOS6502State

    sim = MOS6502GateLevelSimulator()
    result = sim.execute(bytes([
        0xA9, 0x0A,   # LDA #10
        0x69, 0x05,   # ADC #5
        0x00,          # BRK
    ]))
    assert result.final_state.a == 15
"""

from mos6502_gatelevel.simulator import MOS6502GateLevelSimulator

__all__ = ["MOS6502GateLevelSimulator"]
__version__ = "1.0.0"
