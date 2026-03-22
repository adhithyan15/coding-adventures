"""Block RAM — hardware-level read/write memory arrays.

This package provides SRAM cells, arrays, and RAM modules that model
the actual memory hardware found in FPGAs and ASICs. Built from the
ground up: individual 6T SRAM cells → 2D arrays → synchronous RAM
modules with read modes and dual-port access → configurable Block RAM
with reconfigurable aspect ratios.

Modules:
    sram: SRAMCell and SRAMArray (raw storage)
    ram:  SinglePortRAM and DualPortRAM (synchronous, with read modes)
    bram: ConfigurableBRAM (FPGA-style reconfigurable memory)
"""

from block_ram.bram import ConfigurableBRAM
from block_ram.ram import (
    DualPortRAM,
    ReadMode,
    SinglePortRAM,
    WriteCollisionError,
)
from block_ram.sram import SRAMArray, SRAMCell

__all__ = [
    "ConfigurableBRAM",
    "DualPortRAM",
    "ReadMode",
    "SinglePortRAM",
    "SRAMArray",
    "SRAMCell",
    "WriteCollisionError",
]
