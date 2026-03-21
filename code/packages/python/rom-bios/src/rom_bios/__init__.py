"""ROM & BIOS firmware for simulated computer power-on initialization.

This package implements the very first code that runs when the simulated
computer powers on. ROM (Read-Only Memory) is a memory region at address
0xFFFF0000 that cannot be modified. It contains the BIOS firmware -- a
RISC-V program that initializes hardware and hands off to the bootloader.
"""

from rom_bios.bios import (
    AnnotatedInstruction,
    BIOSConfig,
    BIOSFirmware,
    DefaultBIOSConfig,
)
from rom_bios.hardware_info import (
    HARDWARE_INFO_ADDRESS,
    HARDWARE_INFO_SIZE,
    HardwareInfo,
)
from rom_bios.rom import (
    DEFAULT_ROM_BASE,
    DEFAULT_ROM_SIZE,
    ROM,
    ROMConfig,
    DefaultROMConfig,
)

__all__ = [
    "ROM",
    "ROMConfig",
    "DefaultROMConfig",
    "DEFAULT_ROM_BASE",
    "DEFAULT_ROM_SIZE",
    "HardwareInfo",
    "HARDWARE_INFO_ADDRESS",
    "HARDWARE_INFO_SIZE",
    "BIOSFirmware",
    "BIOSConfig",
    "DefaultBIOSConfig",
    "AnnotatedInstruction",
]
