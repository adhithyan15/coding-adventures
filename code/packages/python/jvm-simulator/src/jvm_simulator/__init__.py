"""JVM Simulator -- Layer 4e of the computing stack.

Simulates the Java Virtual Machine bytecode instruction set.
"""

from __future__ import annotations

from jvm_simulator.simulator import (
    JVMOpcode,
    JVMSimulator,
    JVMTrace,
    assemble_jvm,
    encode_iconst,
    encode_iload,
    encode_istore,
)
from jvm_simulator.state import JVMState

__all__ = [
    "JVMOpcode",
    "JVMSimulator",
    "JVMState",
    "JVMTrace",
    "assemble_jvm",
    "encode_iconst",
    "encode_iload",
    "encode_istore",
]
