"""Intel 8080 Simulator — Layer 4i of the computing stack.

The Altair 8800's CPU, the ancestor of Z80 and x86, simulated in Python.
"""

from intel8080_simulator.flags import (
    compute_ac_add,
    compute_ac_sub,
    compute_cy_add,
    compute_cy_sub,
    compute_p,
    compute_s,
    compute_z,
    flags_from_byte,
    szp_flags,
)
from intel8080_simulator.simulator import Intel8080Simulator
from intel8080_simulator.state import Intel8080State

__all__ = [
    # Simulator
    "Intel8080Simulator",
    # State
    "Intel8080State",
    # Flag helpers
    "compute_s",
    "compute_z",
    "compute_p",
    "compute_cy_add",
    "compute_cy_sub",
    "compute_ac_add",
    "compute_ac_sub",
    "flags_from_byte",
    "szp_flags",
]
