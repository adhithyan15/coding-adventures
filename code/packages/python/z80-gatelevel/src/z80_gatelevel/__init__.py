"""z80_gatelevel — Zilog Z80 gate-level simulator.

Every ALU operation routes through real logic gate functions (AND, OR, XOR,
NOT, ripple_carry_adder) from the `coding-adventures-logic-gates` and
`coding-adventures-arithmetic` packages.

Registers are stored as arrays of D flip-flops (simulated). Memory uses a
plain Python bytearray (simulating 65,536 flip-flop cells per byte would be
impractical, but the data path operations are all gate-level).

This package is part of the Layer 07 CPU simulator series:
  07a: Manchester Baby (1948)  — behavioral
  07b: IBM 704 (1954)          — behavioral
  07c: Intel 8080 (1974)       — behavioral
  07d: Z80 (1976)              — behavioral
  ...
  07k1: Intel 8080 gate-level
  07k2: Z80 gate-level  ← this package

The output type is Z80State from `coding-adventures-z80-simulator`, making
gate-level and behavioral simulators drop-in compatible.

Usage::

    from z80_gatelevel import Z80GateLevelSimulator

    sim = Z80GateLevelSimulator()
    result = sim.execute(bytes([
        0x3E, 0x05,  # LD A, 5
        0x06, 0x03,  # LD B, 3
        0x80,        # ADD A, B   ← routes through 8 full-adder stages
        0x76,        # HALT
    ]))
    assert result.final_state.a == 8
    assert result.halted is True
"""

from z80_gatelevel.simulator import Z80GateLevelSimulator

__all__ = ["Z80GateLevelSimulator"]
