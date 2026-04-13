"""RISC-V RV32I Simulator with M-mode privileged extensions.

=== What is RISC-V? ===

RISC-V (pronounced "risk-five") is an open-source instruction set architecture
(ISA) designed at UC Berkeley. It was designed from scratch in 2010 with no
historical baggage, making it the cleanest ISA to learn.

=== What this simulator supports ===

The full RV32I base integer instruction set (37 instructions):
  - Arithmetic: add, sub, addi, slt, sltu, slti, sltiu, and, or, xor, andi, ori, xori
  - Shifts: sll, srl, sra, slli, srli, srai
  - Loads: lb, lh, lw, lbu, lhu
  - Stores: sb, sh, sw
  - Branches: beq, bne, blt, bge, bltu, bgeu
  - Jumps: jal, jalr
  - Upper immediates: lui, auipc
  - System: ecall

Plus M-mode privileged extensions:
  - CSR access: csrrw, csrrs, csrrc
  - Trap return: mret
  - CSR registers: mstatus, mtvec, mepc, mcause, mscratch

=== Architecture ===

This simulator bridges the gap between binary encoded bits and the generic
fetch-decode-execute cycle provided by the cpu-simulator package:

  simulator.py  -- top-level simulator struct and factory
  opcodes.py    -- opcode and funct3/funct7 constants
  decode.py     -- instruction decoder (binary -> structured fields)
  execute.py    -- instruction executor (structured fields -> state changes)
  csr.py        -- Control and Status Register file for M-mode
  encoding.py   -- helpers to construct machine code for testing
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from cpu_simulator.cpu import CPU
from cpu_simulator.pipeline import PipelineTrace

from riscv_simulator.csr import (
    CSR_MCAUSE,
    CSR_MEPC,
    CSR_MSCRATCH,
    CSR_MSTATUS,
    CSR_MTVEC,
    CSRFile,
)
from riscv_simulator.decode import RiscVDecoder
from riscv_simulator.encoding import assemble, encode_addi, encode_add, encode_ecall
from riscv_simulator.execute import RiscVExecutor
from riscv_simulator.state import RiscVState

if TYPE_CHECKING:
    from simulator_protocol import ExecutionResult, StepTrace


class RiscVSimulator:
    """Complete RISC-V simulator -- ISA + CPU in one convenient class.

    This wraps the CPU simulator with the RISC-V decoder and executor,
    providing a simple interface for running RISC-V programs.
    """

    def __init__(self, memory_size: int = 65536) -> None:
        self.decoder = RiscVDecoder()
        self.csr = CSRFile()
        self.executor = RiscVExecutor(csr=self.csr)
        self.cpu = CPU(
            decoder=self.decoder,
            executor=self.executor,
            num_registers=32,
            bit_width=32,
            memory_size=memory_size,
        )

    def run(self, program: bytes) -> list[PipelineTrace]:
        """Load and run a RISC-V program, returning the pipeline trace."""
        self.cpu.load_program(program)
        return self.cpu.run()

    def step(self) -> PipelineTrace:
        """Execute one instruction and return its pipeline trace."""
        return self.cpu.step()

    # ── simulator-protocol conformance ────────────────────────────────────

    def get_state(self) -> RiscVState:
        """Return a frozen snapshot of the current simulator state.

        Satisfies the ``Simulator[RiscVState]`` protocol.  Reads all 32
        general-purpose registers, the PC, the five M-mode CSRs, and a
        copy of the full memory.

        Memory is captured as ``bytes`` (immutable) by reading the
        underlying ``bytearray`` via ``bytes()``, so later writes to the
        simulator's RAM do NOT affect the snapshot.

        Returns
        -------
        RiscVState:
            Frozen dataclass with all simulator state at this moment.
        """
        return RiscVState(
            registers=tuple(self.cpu.registers.read(i) for i in range(32)),
            pc=self.cpu.pc,
            csr_mstatus=self.csr.read(CSR_MSTATUS),
            csr_mtvec=self.csr.read(CSR_MTVEC),
            csr_mscratch=self.csr.read(CSR_MSCRATCH),
            csr_mepc=self.csr.read(CSR_MEPC),
            csr_mcause=self.csr.read(CSR_MCAUSE),
            memory=bytes(self.cpu.memory._data),
            halted=self.cpu.halted,
        )

    def load(self, program: bytes) -> None:
        """Load a binary program into memory at address 0.

        Satisfies the ``Simulator[RiscVState]`` protocol ``load()`` method.
        Delegates to ``cpu.load_program()`` so all existing callers are
        unaffected.

        Parameters
        ----------
        program:
            Raw machine-code bytes to write into memory starting at address 0.
        """
        self.cpu.load_program(program, 0)

    def reset(self) -> None:
        """Reset the simulator to power-on state.

        Satisfies the ``Simulator[RiscVState]`` protocol ``reset()`` method.
        Rebuilds the CPU, CSR file, decoder, and executor from scratch so
        every register, flag, and memory cell is zeroed.

        After reset, the simulator is ready to run a fresh program as if it
        had just been constructed.
        """
        memory_size = self.cpu.memory.size
        self.decoder = RiscVDecoder()
        self.csr = CSRFile()
        self.executor = RiscVExecutor(csr=self.csr)
        self.cpu = CPU(
            decoder=self.decoder,
            executor=self.executor,
            num_registers=32,
            bit_width=32,
            memory_size=memory_size,
        )

    def execute(
        self, program: bytes, max_steps: int = 100_000
    ) -> ExecutionResult[RiscVState]:
        """Load *program*, run to HALT or *max_steps*, return a full result.

        Satisfies the ``Simulator[RiscVState]`` protocol.

        This is the primary entry point for end-to-end testing.  It:

        1. Resets the simulator to power-on state.
        2. Loads the program bytes into memory at address 0.
        3. Runs the fetch-decode-execute loop.
        4. Collects a ``StepTrace`` for every instruction executed.
        5. Returns an ``ExecutionResult[RiscVState]`` with the final state,
           trace list, halt status, and error (if any).

        The ``StepTrace`` for each step is built from the ``PipelineTrace``
        returned by ``cpu.step()``:

        - ``pc_before``  <- ``pipeline_trace.fetch.pc``
        - ``pc_after``   <- the ``cpu.pc`` *after* the step
        - ``mnemonic``   <- ``pipeline_trace.decode.mnemonic``
        - ``description`` <- a formatted string with mnemonic and address

        HALT detection: the underlying ``CPU.step()`` sets ``cpu.halted``
        when the executor signals ``halted=True`` (ecall with mtvec == 0).

        Parameters
        ----------
        program:
            Raw machine-code bytes.
        max_steps:
            Maximum instructions to execute.  Default 100 000.

        Returns
        -------
        ExecutionResult[RiscVState]:
            Full result including halted status, step count, final state,
            optional error string, and per-instruction trace list.
        """
        from simulator_protocol import ExecutionResult, StepTrace

        self.reset()
        self.cpu.load_program(program, 0)

        step_traces: list[StepTrace] = []
        error: str | None = None

        for _ in range(max_steps):
            if self.cpu.halted:
                break
            pipeline_trace = self.cpu.step()
            pc_after = self.cpu.pc
            mnemonic = pipeline_trace.decode.mnemonic
            pc_before = pipeline_trace.fetch.pc
            step_traces.append(
                StepTrace(
                    pc_before=pc_before,
                    pc_after=pc_after,
                    mnemonic=mnemonic,
                    description=f"{mnemonic} @ 0x{pc_before:08X}",
                )
            )
        else:
            # Loop completed without a break — max_steps was reached
            if not self.cpu.halted:
                error = f"max_steps ({max_steps}) exceeded"

        return ExecutionResult(
            halted=self.cpu.halted,
            steps=len(step_traces),
            final_state=self.get_state(),
            error=error,
            traces=step_traces,
        )


# Re-export encoding helpers for backward compatibility
__all__ = [
    "RiscVSimulator",
    "RiscVDecoder",
    "RiscVExecutor",
    "CSRFile",
    "assemble",
    "encode_addi",
    "encode_add",
    "encode_ecall",
]
