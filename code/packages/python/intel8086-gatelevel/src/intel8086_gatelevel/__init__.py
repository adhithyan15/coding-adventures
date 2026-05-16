"""Intel 8086 gate-level simulator.

Every data-path operation routes through AND, OR, XOR, NOT, and
ripple_carry_adder primitives — no Python integer arithmetic on the
execution path.

Public API::

    from intel8086_gatelevel import Intel8086GateLevelSimulator
    from intel_8086_simulator.state import X86State

    sim = Intel8086GateLevelSimulator()
    result = sim.execute(bytes([
        0xB8, 0x0A, 0x00,   # MOV AX, 10
        0xF4,               # HLT
    ]))
    assert result.final_state.ax == 10
"""

from intel8086_gatelevel.simulator import Intel8086GateLevelSimulator

__all__ = ["Intel8086GateLevelSimulator"]
__version__ = "1.0.0"
