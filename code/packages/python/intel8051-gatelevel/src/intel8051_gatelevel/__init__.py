"""intel8051_gatelevel — Intel 8051 gate-level simulator — Layer 07p2.

Every arithmetic and logical operation in this simulator routes through
gate primitives from the logic_gates and arithmetic packages:

    ADD/ADDC/SUBB → ripple_carry_adder (full adder chains)
    ANL/ORL/XRL   → AND/OR/XOR gate arrays
    Rotates       → bit array rearrangement with gate-checked carry
    MUL/DIV       → repeated gate-level add8 / subb8 loops
    PC increment  → gate-level 16-bit adder

This is identical in behavior to the intel8051_simulator (behavioral) package.
The difference is the execution path — every bit of computation is made
explicit through gate function calls.

Public API:
    Intel8051GateLevelSimulator — the simulator class
"""

from __future__ import annotations

from .simulator import Intel8051GateLevelSimulator

__all__ = ["Intel8051GateLevelSimulator"]
__version__ = "1.0.0"
