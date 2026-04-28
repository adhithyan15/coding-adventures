"""real-fpga-export: HIR -> structural Verilog for the open-tool FPGA flow.

The fast end-to-end win: write Verilog, hand to yosys/nextpnr/icepack,
flash to a real iCE40 board.
"""

from real_fpga_export.toolchain import (
    ToolchainOptions,
    ToolchainResult,
    program_ice40,
    to_ice40,
)
from real_fpga_export.verilog_writer import (
    write_verilog,
    write_verilog_str,
)

__version__ = "0.1.0"

__all__ = [
    "ToolchainOptions",
    "ToolchainResult",
    "__version__",
    "program_ice40",
    "to_ice40",
    "write_verilog",
    "write_verilog_str",
]
