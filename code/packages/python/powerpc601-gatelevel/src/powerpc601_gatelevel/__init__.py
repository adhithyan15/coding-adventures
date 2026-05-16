"""PowerPC 601 (1992) gate-level simulator — Layer 07u2.

Every 32-bit ALU operation routes through logic gate primitives (AND, OR,
XOR, NOT) and ripple_carry_adder.  No Python arithmetic operators appear
in the data-path execution paths.

Main entry point:
    PowerPC601GateLevelSimulator — implements Simulator[PowerPC601State]

Supporting modules:
    bits.py          — 32-bit int ↔ bit-list conversion helpers
    alu.py           — gate-level 32-bit ALU operations
    register_file.py — GPR, LR, CTR, XER, CR, CIA as bit lists
    decoder.py       — combinational instruction decode
    simulator.py     — top-level simulation loop
"""

from __future__ import annotations

from .simulator import PowerPC601GateLevelSimulator

__all__ = ["PowerPC601GateLevelSimulator"]
