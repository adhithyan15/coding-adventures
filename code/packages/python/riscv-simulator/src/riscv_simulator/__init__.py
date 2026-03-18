"""RISC-V Simulator — Layer 7a of the computing stack.

Minimal RV32I instruction decoder and executor.
Plugs into the CPU simulator via the decoder/executor protocol.
"""

from riscv_simulator.simulator import RiscVSimulator

__all__ = ["RiscVSimulator"]
