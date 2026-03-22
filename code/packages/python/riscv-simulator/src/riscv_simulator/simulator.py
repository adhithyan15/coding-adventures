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

from cpu_simulator.cpu import CPU
from cpu_simulator.pipeline import PipelineTrace

from riscv_simulator.csr import CSRFile
from riscv_simulator.decode import RiscVDecoder
from riscv_simulator.encoding import assemble, encode_addi, encode_add, encode_ecall
from riscv_simulator.execute import RiscVExecutor


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
