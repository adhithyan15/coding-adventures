"""JVM simulator package built on disassembled JVM bytecode."""

from __future__ import annotations

from jvm_bytecode_disassembler import (
    JVMInstruction,
    JVMMethodBody,
    JVMOpcode,
    JVMVersion,
    assemble_jvm,
    disassemble_method_body,
    encode_iconst,
    encode_iload,
    encode_istore,
)

from jvm_simulator.simulator import JVMSimulator, JVMTrace
from jvm_simulator.state import JVMState

__all__ = [
    "JVMOpcode",
    "JVMInstruction",
    "JVMMethodBody",
    "JVMVersion",
    "JVMSimulator",
    "JVMState",
    "JVMTrace",
    "assemble_jvm",
    "disassemble_method_body",
    "encode_iconst",
    "encode_iload",
    "encode_istore",
]
