"""FPGA — Field-Programmable Gate Array abstraction.

This package models the architecture of an FPGA, from the atomic LUT
(Look-Up Table) up through slices, CLBs (Configurable Logic Blocks),
routing fabric, and I/O blocks.

The key insight: **a truth table is a program**. A LUT stores a truth
table in SRAM and uses a MUX tree to evaluate it. By connecting LUTs
through a programmable routing fabric, any digital circuit can be
implemented — and reprogrammed — without changing the hardware.

Modules:
    lut:            LUT (K-input look-up table)
    slice:          Slice (2 LUTs + 2 FFs + carry chain)
    clb:            CLB (2 slices)
    switch_matrix:  SwitchMatrix (programmable routing crossbar)
    io_block:       IOBlock (bidirectional I/O pad)
"""

from fpga.clb import CLB, CLBOutput
from fpga.io_block import IOBlock, IOMode
from fpga.lut import LUT
from fpga.slice import Slice, SliceOutput
from fpga.switch_matrix import SwitchMatrix

__all__ = [
    "CLB",
    "CLBOutput",
    "IOBlock",
    "IOMode",
    "LUT",
    "Slice",
    "SliceOutput",
    "SwitchMatrix",
]
