"""Protocols — the pluggable interfaces that make this core vendor-agnostic.

=== Why Protocols? ===

Every GPU vendor (NVIDIA, AMD, Intel, ARM) and every accelerator type (GPU,
TPU, NPU) has a processing element at its heart. They all do the same basic
thing: compute floating-point operations. But the details differ:

    NVIDIA CUDA Core:     FP32 ALU + 255 registers + PTX instructions
    AMD Stream Processor: FP32 ALU + 256 VGPRs + GCN instructions
    Intel Vector Engine:  SIMD8 ALU + GRF + Xe instructions
    ARM Mali Exec Engine: FP32 ALU + register bank + Mali instructions
    TPU Processing Element: MAC unit + weight register + accumulator
    NPU MAC Unit:         MAC + activation function + buffer

Instead of building separate simulators for each, we define two protocols:

1. ProcessingElement — the generic "any compute unit" interface
2. InstructionSet — the pluggable "how to decode and execute instructions"

Any vendor-specific implementation just needs to satisfy these protocols.
The core simulation infrastructure (registers, memory, tracing) is reused.

=== What is a Protocol? ===

In Python, a Protocol is like an interface in Java or Go, or a trait in Rust.
It says "any class that has these methods can be used here" without requiring
inheritance. This is called structural subtyping — if it looks like a duck
and quacks like a duck, it's a duck.

    class Flyable(Protocol):
        def fly(self) -> None: ...

    class Bird:
        def fly(self) -> None:
            print("flap flap")

    class Airplane:
        def fly(self) -> None:
            print("zoom")

    # Both Bird and Airplane satisfy Flyable — no inheritance needed!
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Protocol, runtime_checkable

if TYPE_CHECKING:
    from gpu_core.memory import LocalMemory
    from gpu_core.opcodes import Instruction
    from gpu_core.registers import FPRegisterFile


# ---------------------------------------------------------------------------
# ExecuteResult — what an instruction execution produces
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class ExecuteResult:
    """The outcome of executing a single instruction.

    This is what the InstructionSet's execute() method returns. It tells the
    core what changed so the core can build a complete execution trace.

    Fields:
        description:       Human-readable summary, e.g. "R3 = R1 * R2 = 6.0"
        next_pc_offset:    How to advance the program counter.
                           +1 for most instructions (next instruction).
                           Other values for branches/jumps.
        absolute_jump:     If True, next_pc_offset is an absolute address,
                           not a relative offset.
        registers_changed: Map of register name → new float value.
        memory_changed:    Map of memory address → new float value.
        halted:            True if this instruction stops execution.
    """

    description: str
    next_pc_offset: int = 1
    absolute_jump: bool = False
    registers_changed: dict[str, float] | None = None
    memory_changed: dict[int, float] | None = None
    halted: bool = False


# ---------------------------------------------------------------------------
# InstructionSet — pluggable ISA (the key to vendor-agnosticism)
# ---------------------------------------------------------------------------


@runtime_checkable
class InstructionSet(Protocol):
    """A pluggable instruction set that can be swapped to simulate any vendor.

    === How it works ===

    The GPUCore calls isa.execute(instruction, registers, memory) for each
    instruction. The ISA implementation:
    1. Reads the opcode to determine what operation to perform
    2. Reads source registers and/or memory
    3. Performs the computation (using fp_add, fp_mul, fp_fma, etc.)
    4. Writes the result to the destination register and/or memory
    5. Returns an ExecuteResult describing what happened

    === Implementing a new ISA ===

    To add support for a new vendor (e.g., NVIDIA PTX):

        class PTXISA:
            @property
            def name(self) -> str:
                return "PTX"

            def execute(self, instruction, registers, memory) -> ExecuteResult:
                match instruction.opcode:
                    case PTXOp.ADD_F32: ...
                    case PTXOp.FMA_RN_F32: ...

        core = GPUCore(isa=PTXISA())
    """

    @property
    def name(self) -> str:
        """The ISA name, e.g. 'Generic', 'PTX', 'GCN', 'Xe', 'Mali'."""
        ...

    def execute(
        self,
        instruction: Instruction,
        registers: FPRegisterFile,
        memory: LocalMemory,
    ) -> ExecuteResult:
        """Decode and execute a single instruction.

        Args:
            instruction: The instruction to execute.
            registers: The core's floating-point register file.
            memory: The core's local scratchpad memory.

        Returns:
            An ExecuteResult describing what happened.
        """
        ...


# ---------------------------------------------------------------------------
# ProcessingElement — the most generic abstraction
# ---------------------------------------------------------------------------


@runtime_checkable
class ProcessingElement(Protocol):
    """Any compute unit in any accelerator.

    This is the most generic interface — a GPU core, a TPU processing element,
    and an NPU MAC unit all satisfy this protocol. It provides just enough
    structure for a higher-level component (like a warp scheduler or systolic
    array controller) to drive the PE.

    === Why so minimal? ===

    Different accelerators have radically different execution models:
    - GPUs: instruction-stream + register file (step = execute one instruction)
    - TPUs: dataflow, no instructions (step = one MAC + pass data to neighbor)
    - NPUs: scheduled MACs (step = one MAC from the scheduler's queue)

    This protocol captures only what they ALL share: the ability to advance
    one cycle, check if done, and reset.
    """

    def step(self) -> object:
        """Execute one cycle. Returns a trace of what happened."""
        ...

    @property
    def halted(self) -> bool:
        """True if this PE has finished execution."""
        ...

    def reset(self) -> None:
        """Reset to initial state."""
        ...
