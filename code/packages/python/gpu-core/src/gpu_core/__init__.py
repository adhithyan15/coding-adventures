"""GPU Core — generic, pluggable accelerator processing element.

This package implements a single GPU processing element (Layer 9 of the
accelerator computing stack) with a pluggable instruction set architecture.
It sits between FP arithmetic (Layer 10) and the warp/SIMT engine (Layer 8).

The core is designed to be vendor-agnostic: swap the InstructionSet to
simulate NVIDIA CUDA cores, AMD stream processors, Intel Arc vector engines,
ARM Mali execution engines, or any other accelerator.

Basic usage:
    >>> from gpu_core import GPUCore, GenericISA, limm, fmul, halt
    >>> core = GPUCore(isa=GenericISA())
    >>> core.load_program([limm(0, 3.0), limm(1, 4.0), fmul(2, 0, 1), halt()])
    >>> core.run()
    >>> core.registers.read_float(2)
    12.0
"""

from gpu_core.core import GPUCore
from gpu_core.generic_isa import GenericISA
from gpu_core.memory import LocalMemory
from gpu_core.opcodes import (
    Instruction,
    Opcode,
    beq,
    blt,
    bne,
    fabs,
    fadd,
    ffma,
    fmul,
    fneg,
    fsub,
    halt,
    jmp,
    limm,
    load,
    mov,
    nop,
    store,
)
from gpu_core.protocols import ExecuteResult, InstructionSet, ProcessingElement
from gpu_core.registers import FPRegisterFile
from gpu_core.trace import GPUCoreTrace

__all__ = [
    # Core
    "GPUCore",
    # ISA
    "GenericISA",
    "InstructionSet",
    # Protocols
    "ProcessingElement",
    "ExecuteResult",
    # Components
    "FPRegisterFile",
    "LocalMemory",
    # Instructions
    "Instruction",
    "Opcode",
    # Trace
    "GPUCoreTrace",
    # Helpers
    "fadd",
    "fsub",
    "fmul",
    "ffma",
    "fneg",
    "fabs",
    "load",
    "store",
    "mov",
    "limm",
    "beq",
    "blt",
    "bne",
    "jmp",
    "nop",
    "halt",
]
