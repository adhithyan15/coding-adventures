"""Version-aware JVM bytecode disassembler package."""

from __future__ import annotations

from jvm_bytecode_disassembler.disassembler import (
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

__all__ = [
    "JVMInstruction",
    "JVMMethodBody",
    "JVMOpcode",
    "JVMVersion",
    "assemble_jvm",
    "disassemble_method_body",
    "encode_iconst",
    "encode_iload",
    "encode_istore",
]
