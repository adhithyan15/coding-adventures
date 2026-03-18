"""CPU — the central processing unit that ties everything together.

=== What is a CPU? ===

The CPU (Central Processing Unit) is the "brain" of a computer. But unlike
a human brain, it's extremely simple — it can only do one thing:

    Read an instruction, figure out what it means, do what it says. Repeat.

That's it. That's all a CPU does. The power of a computer comes not from
the complexity of individual operations (they're trivial — add two numbers,
copy a value, compare two things) but from doing billions of them per second.

=== CPU components ===

A CPU has four main parts:

    ┌──────────────────────────────────────────────────────┐
    │                        CPU                           │
    │                                                      │
    │  ┌──────────┐  ┌────────────────┐  ┌─────────────┐  │
    │  │ Program  │  │  Register File │  │    ALU      │  │
    │  │ Counter  │  │ R0  R1  R2 ... │  │  (add, sub, │  │
    │  │  (PC)    │  │ [0] [0] [0]    │  │   and, or)  │  │
    │  └──────────┘  └────────────────┘  └─────────────┘  │
    │                                                      │
    │  ┌──────────────────────────────────────────┐        │
    │  │  Control Unit (fetch-decode-execute)      │        │
    │  └──────────────────────────────────────────┘        │
    │                                                      │
    └───────────────────────┬──────────────────────────────┘
                            │ (memory bus)
                            ▼
    ┌──────────────────────────────────────────────────────┐
    │                      Memory                          │
    │  [instruction 0] [instruction 1] [data] [data] ...   │
    └──────────────────────────────────────────────────────┘

  - Program Counter (PC): A special register that holds the address of
    the next instruction to execute. It's like a bookmark in a book.

  - Register File: A small set of fast storage slots (see registers.py).

  - ALU: The arithmetic/logic unit that does actual computation
    (see the arithmetic package).

  - Control Unit: The logic that orchestrates the fetch-decode-execute
    cycle — reading instructions, decoding them, and dispatching
    operations to the ALU and registers.

  - Memory: External storage connected via a "bus" (see memory.py).

=== How this module works ===

This module provides the CPU shell — registers, memory, PC, and the
pipeline framework. It does NOT know how to decode specific instructions
(that's ISA-specific). Instead, it accepts a `decode_fn` and `execute_fn`
that are provided by the ISA simulator (RISC-V, ARM, etc.).

This separation means the same CPU can run RISC-V, ARM, WASM, or 4004
instructions — you just plug in a different decoder.
"""

from dataclasses import dataclass, field
from typing import Protocol

from cpu_simulator.memory import Memory
from cpu_simulator.pipeline import (
    DecodeResult,
    ExecuteResult,
    FetchResult,
    PipelineTrace,
)
from cpu_simulator.registers import RegisterFile


# ---------------------------------------------------------------------------
# Decoder and Executor protocols
# ---------------------------------------------------------------------------
# These define the interface that ISA simulators must implement.
# The CPU calls decode() and execute() — the ISA provides the implementation.


class InstructionDecoder(Protocol):
    """Interface for ISA-specific instruction decoding.

    The CPU fetches raw bits from memory and passes them to the decoder.
    The decoder figures out what those bits mean in the context of a
    specific instruction set (RISC-V, ARM, etc.).
    """

    def decode(self, raw_instruction: int, pc: int) -> DecodeResult: ...


class InstructionExecutor(Protocol):
    """Interface for ISA-specific instruction execution.

    The CPU passes the decoded instruction to the executor, along with
    the register file and memory. The executor performs the operation
    and returns what changed.
    """

    def execute(
        self,
        decoded: DecodeResult,
        registers: RegisterFile,
        memory: Memory,
        pc: int,
    ) -> ExecuteResult: ...


# ---------------------------------------------------------------------------
# CPU State
# ---------------------------------------------------------------------------


@dataclass
class CPUState:
    """A snapshot of the entire CPU state at a point in time.

    This is useful for debugging and visualization — you can capture
    the state before and after each instruction to see what changed.

    Example:
        CPUState(
            pc=8,
            registers={'R0': 0, 'R1': 1, 'R2': 2, 'R3': 3},
            halted=False,
            cycle=2,
        )
    """

    pc: int
    registers: dict[str, int]
    halted: bool
    cycle: int


# ---------------------------------------------------------------------------
# CPU
# ---------------------------------------------------------------------------


class CPU:
    """A generic CPU that executes instructions through a visible pipeline.

    The CPU doesn't know what instruction set it's running — that's
    determined by the decoder and executor you provide. This makes it
    reusable across RISC-V, ARM, WASM, and Intel 4004.

    Usage:
        1. Create a CPU with a decoder and executor
        2. Load a program into memory
        3. Call step() to execute one instruction (visible pipeline)
        4. Or call run() to execute until halt

    Example:
        >>> cpu = CPU(
        ...     decoder=my_riscv_decoder,
        ...     executor=my_riscv_executor,
        ...     num_registers=32,
        ... )
        >>> cpu.load_program(machine_code_bytes)
        >>> trace = cpu.step()
        >>> print(trace.format_pipeline())
        --- Cycle 0 ---
          FETCH              | DECODE             | EXECUTE
          PC: 0x0000         | addi x1, x0, 1     | x1 = 1
          -> 0x00100093      | rd=1 rs1=0 imm=1   | PC -> 4
    """

    def __init__(
        self,
        decoder: InstructionDecoder,
        executor: InstructionExecutor,
        num_registers: int = 16,
        bit_width: int = 32,
        memory_size: int = 65536,
    ) -> None:
        self.registers = RegisterFile(
            num_registers=num_registers, bit_width=bit_width
        )
        self.memory = Memory(size=memory_size)
        self.pc: int = 0
        self.halted: bool = False
        self.cycle: int = 0
        self._decoder = decoder
        self._executor = executor

    @property
    def state(self) -> CPUState:
        """Capture the current CPU state as a snapshot."""
        return CPUState(
            pc=self.pc,
            registers=self.registers.dump(),
            halted=self.halted,
            cycle=self.cycle,
        )

    def load_program(self, program: bytes, start_address: int = 0) -> None:
        """Load machine code bytes into memory.

        This is how programs get into the computer — the bytes are copied
        into memory starting at `start_address`. The PC is set to point
        at the first instruction.

        Example:
            >>> cpu.load_program(b'\\x93\\x00\\x10\\x00')  # addi x1, x0, 1
        """
        self.memory.load_bytes(start_address, program)
        self.pc = start_address

    def step(self) -> PipelineTrace:
        """Execute ONE instruction through the full pipeline.

        This is the core of the CPU — the fetch-decode-execute cycle
        made visible. Each call to step() processes one instruction
        and returns a PipelineTrace showing what happened at each stage.

        The three stages:

        ┌───────────┐    ┌───────────┐    ┌───────────┐
        │   FETCH   │───→│  DECODE   │───→│  EXECUTE  │
        │           │    │           │    │           │
        │ Read 4    │    │ What does │    │ Do the    │
        │ bytes at  │    │ this      │    │ operation,│
        │ PC from   │    │ binary    │    │ update    │
        │ memory    │    │ mean?     │    │ registers │
        └───────────┘    └───────────┘    └───────────┘

        Returns:
            PipelineTrace with fetch, decode, and execute results.

        Raises:
            RuntimeError: If the CPU has halted (no more instructions).
        """
        if self.halted:
            msg = "CPU has halted — no more instructions to execute"
            raise RuntimeError(msg)

        # === STAGE 1: FETCH ===
        # Read 4 bytes from memory at the current PC.
        # These 4 bytes form one 32-bit instruction.
        raw_instruction = self.memory.read_word(self.pc)
        fetch_result = FetchResult(pc=self.pc, raw_instruction=raw_instruction)

        # === STAGE 2: DECODE ===
        # Pass the raw bits to the ISA-specific decoder.
        # The decoder extracts the opcode, register numbers, and immediate values.
        decode_result = self._decoder.decode(raw_instruction, self.pc)

        # === STAGE 3: EXECUTE ===
        # Pass the decoded instruction to the ISA-specific executor.
        # The executor reads registers, uses the ALU, writes results back.
        execute_result = self._executor.execute(
            decode_result, self.registers, self.memory, self.pc
        )

        # === UPDATE CPU STATE ===
        # After execution, update the PC and check if we should halt.
        self.pc = execute_result.next_pc
        self.halted = execute_result.halted

        # Build the complete pipeline trace for this instruction
        trace = PipelineTrace(
            cycle=self.cycle,
            fetch=fetch_result,
            decode=decode_result,
            execute=execute_result,
            register_snapshot=self.registers.dump(),
        )

        self.cycle += 1
        return trace

    def run(self, max_steps: int = 10000) -> list[PipelineTrace]:
        """Run the CPU until it halts or hits the step limit.

        Returns a list of PipelineTrace objects — one for each instruction
        executed. This gives you the complete execution history.

        Example:
            >>> traces = cpu.run()
            >>> for trace in traces:
            ...     print(trace.format_pipeline())

        Args:
            max_steps: Safety limit to prevent infinite loops.

        Returns:
            List of PipelineTrace objects, one per instruction.
        """
        traces: list[PipelineTrace] = []
        for _ in range(max_steps):
            if self.halted:
                break
            traces.append(self.step())
        return traces
