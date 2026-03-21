"""CPU Simulator — Layer 8 of the computing stack.

Simulates the core of a processor: registers, memory, program counter,
and the fetch-decode-execute cycle that drives all computation.

This is a generic CPU model — not tied to any specific architecture.
The ISA simulators (RISC-V, ARM, WASM, Intel 4004) build on top of this
by providing their own instruction decoders.
"""

from cpu_simulator.cpu import CPU, CPUState
from cpu_simulator.memory import Memory
from cpu_simulator.pipeline import PipelineStage, PipelineTrace
from cpu_simulator.registers import RegisterFile
from cpu_simulator.sparse_memory import MemoryRegion, SparseMemory

__all__ = [
    "CPU",
    "CPUState",
    "Memory",
    "MemoryRegion",
    "RegisterFile",
    "PipelineStage",
    "PipelineTrace",
    "SparseMemory",
]
