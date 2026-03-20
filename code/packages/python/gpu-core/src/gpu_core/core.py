"""GPUCore — the generic, pluggable accelerator processing element.

=== What is a GPU Core? ===

A GPU core is the smallest independently programmable compute unit on a GPU.
It's like a tiny, simplified CPU that does one thing well: floating-point math.

    CPU Core (complex):                    GPU Core (simple):
    ┌────────────────────────┐             ┌──────────────────────┐
    │ Branch predictor       │             │                      │
    │ Out-of-order engine    │             │ In-order execution   │
    │ Large cache hierarchy  │             │ Small register file  │
    │ Integer + FP ALUs      │             │ FP ALU only          │
    │ Complex decoder        │             │ Simple fetch-execute  │
    │ Speculative execution  │             │ No speculation       │
    └────────────────────────┘             └──────────────────────┘

A single GPU core is MUCH simpler than a CPU core. GPUs achieve performance
not through per-core complexity, but through massive parallelism: thousands
of these simple cores running in parallel.

=== How This Core is Pluggable ===

The GPUCore takes an InstructionSet as a constructor parameter. This ISA
object handles all the vendor-specific decode and execute logic:

    # Generic educational ISA (this package)
    core = GPUCore(isa=GenericISA())

    # NVIDIA PTX (future package)
    core = GPUCore(isa=PTXISA(), num_registers=255)

    # AMD GCN (future package)
    core = GPUCore(isa=GCNISA(), num_registers=256)

The core itself (fetch loop, registers, memory, tracing) stays the same.
Only the ISA changes.

=== Execution Model ===

The GPU core uses a simple fetch-execute loop (no separate decode stage):

    ┌─────────────────────────────────────────┐
    │              GPU Core                    │
    │                                         │
    │  ┌─────────┐    ┌──────────────────┐   │
    │  │ Program  │───→│   Fetch          │   │
    │  │ Memory   │    │   instruction    │   │
    │  └─────────┘    │   at PC          │   │
    │                  └───────┬──────────┘   │
    │                          │              │
    │                  ┌───────▼──────────┐   │
    │  ┌───────────┐  │   ISA.execute()  │   │
    │  │ Register  │◄─│   (pluggable!)   │──→│ Trace
    │  │ File      │──│                  │   │
    │  └───────────┘  └───────┬──────────┘   │
    │                          │              │
    │  ┌───────────┐  ┌───────▼──────────┐   │
    │  │  Local    │◄─│  Update PC       │   │
    │  │  Memory   │  └──────────────────┘   │
    │  └───────────┘                         │
    └─────────────────────────────────────────┘

Each step():
1. Fetch: read instruction at program[PC]
2. Execute: call isa.execute(instruction, registers, memory)
3. Update PC: advance based on ExecuteResult (branch or +1)
4. Return trace: GPUCoreTrace with full execution details
"""

from __future__ import annotations

from fp_arithmetic import FP32, FloatFormat

from gpu_core.generic_isa import GenericISA
from gpu_core.memory import LocalMemory
from gpu_core.opcodes import Instruction
from gpu_core.protocols import InstructionSet
from gpu_core.registers import FPRegisterFile
from gpu_core.trace import GPUCoreTrace


class GPUCore:
    """A generic GPU processing element with a pluggable instruction set.

    This is the central class of the package. It simulates a single
    processing element — one CUDA core, one AMD stream processor, one
    Intel vector engine, or one ARM Mali execution engine — depending
    on which InstructionSet you plug in.

    Args:
        isa:           The instruction set to use (default: GenericISA).
        fmt:           Floating-point format for registers (default: FP32).
        num_registers: Number of FP registers (default: 32, max: 256).
        memory_size:   Local memory size in bytes (default: 4096).

    Example:
        >>> from gpu_core import GPUCore, GenericISA, limm, fmul, halt
        >>> core = GPUCore(isa=GenericISA())
        >>> core.load_program([
        ...     limm(0, 3.0),
        ...     limm(1, 4.0),
        ...     fmul(2, 0, 1),
        ...     halt(),
        ... ])
        >>> traces = core.run()
        >>> core.registers.read_float(2)
        12.0
    """

    def __init__(
        self,
        isa: InstructionSet | None = None,
        fmt: FloatFormat = FP32,
        num_registers: int = 32,
        memory_size: int = 4096,
    ) -> None:
        self.isa: InstructionSet = isa if isa is not None else GenericISA()
        self.fmt = fmt
        self.registers = FPRegisterFile(num_registers=num_registers, fmt=fmt)
        self.memory = LocalMemory(size=memory_size)
        self.pc: int = 0
        self.cycle: int = 0
        self._halted: bool = False
        self._program: list[Instruction] = []

    @property
    def halted(self) -> bool:
        """True if the core has executed a HALT instruction."""
        return self._halted

    def load_program(self, program: list[Instruction]) -> None:
        """Load a program (list of instructions) into the core.

        This replaces any previously loaded program and resets the PC to 0,
        but does NOT reset registers or memory. Call reset() for a full reset.

        Args:
            program: A list of Instruction objects to execute.
        """
        self._program = list(program)
        self.pc = 0
        self._halted = False
        self.cycle = 0

    def step(self) -> GPUCoreTrace:
        """Execute one instruction and return a trace of what happened.

        This is the core fetch-execute loop:
        1. Check if halted or PC out of range
        2. Fetch instruction at PC
        3. Call ISA.execute() to perform the operation
        4. Update PC based on the result
        5. Build and return a trace record

        Returns:
            A GPUCoreTrace describing what this instruction did.

        Raises:
            RuntimeError: If the core is halted or PC is out of range.
        """
        if self._halted:
            msg = "Cannot step: core is halted"
            raise RuntimeError(msg)

        if self.pc < 0 or self.pc >= len(self._program):
            msg = f"PC={self.pc} out of program range [0, {len(self._program)})"
            raise RuntimeError(msg)

        # Fetch
        instruction = self._program[self.pc]
        current_pc = self.pc
        self.cycle += 1

        # Execute (delegated to the pluggable ISA)
        result = self.isa.execute(instruction, self.registers, self.memory)

        # Update PC
        if result.halted:
            self._halted = True
            next_pc = current_pc  # PC doesn't advance on halt
        elif result.absolute_jump:
            next_pc = result.next_pc_offset
        else:
            next_pc = current_pc + result.next_pc_offset
        self.pc = next_pc

        # Build trace
        return GPUCoreTrace(
            cycle=self.cycle,
            pc=current_pc,
            instruction=instruction,
            description=result.description,
            next_pc=next_pc,
            halted=result.halted,
            registers_changed=result.registers_changed or {},
            memory_changed=result.memory_changed or {},
        )

    def run(self, max_steps: int = 10000) -> list[GPUCoreTrace]:
        """Execute the program until HALT or max_steps reached.

        This repeatedly calls step() until the core halts or the step
        limit is reached (preventing infinite loops from hanging).

        Args:
            max_steps: Maximum number of instructions to execute.

        Returns:
            A list of GPUCoreTrace records, one per instruction executed.

        Raises:
            RuntimeError: If max_steps is exceeded (likely an infinite loop).
        """
        traces: list[GPUCoreTrace] = []
        steps = 0

        while not self._halted and steps < max_steps:
            traces.append(self.step())
            steps += 1

        if not self._halted and steps >= max_steps:
            msg = (
                f"Execution limit reached ({max_steps} steps). "
                f"Possible infinite loop. Last PC={self.pc}"
            )
            raise RuntimeError(msg)

        return traces

    def reset(self) -> None:
        """Reset the core to its initial state.

        Clears registers, memory, PC, and cycle count. The loaded program
        is preserved — call load_program() to change it.
        """
        self.registers = FPRegisterFile(
            num_registers=self.registers.num_registers, fmt=self.fmt
        )
        self.memory = LocalMemory(size=self.memory.size)
        self.pc = 0
        self.cycle = 0
        self._halted = False

    def __repr__(self) -> str:
        status = "halted" if self._halted else f"running at PC={self.pc}"
        return (
            f"GPUCore(isa={self.isa.name}, "
            f"regs={self.registers.num_registers}, "
            f"fmt={self.fmt.name}, "
            f"{status})"
        )
