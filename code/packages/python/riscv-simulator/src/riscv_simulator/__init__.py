"""RISC-V Simulator -- Layer 7a of the computing stack.

Full RV32I instruction decoder and executor with M-mode privileged extensions.
Plugs into the CPU simulator via the decoder/executor protocol.
"""

from riscv_simulator.core_adapter import RiscVISADecoder, new_riscv_core
from riscv_simulator.csr import CSRFile
from riscv_simulator.decode import RiscVDecoder
from riscv_simulator.encoding import assemble
from riscv_simulator.execute import RiscVExecutor
from riscv_simulator.simulator import RiscVSimulator
from riscv_simulator.state import RiscVState

__all__ = [
    "CSRFile",
    "RiscVDecoder",
    "RiscVExecutor",
    "RiscVISADecoder",
    "RiscVSimulator",
    "RiscVState",
    "assemble",
    "new_riscv_core",
]
