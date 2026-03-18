"""JVM Simulator -- Layer 4e of the computing stack.

Simulates the Java Virtual Machine bytecode instruction set.
"""

from jvm_simulator.simulator import (
    JVMOpcode,
    JVMSimulator,
    JVMTrace,
    assemble_jvm,
    encode_iconst,
    encode_iload,
    encode_istore,
)

__all__ = [
    "JVMOpcode",
    "JVMSimulator",
    "JVMTrace",
    "assemble_jvm",
    "encode_iconst",
    "encode_iload",
    "encode_istore",
]
