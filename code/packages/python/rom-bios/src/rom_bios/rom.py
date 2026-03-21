"""ROM (Read-Only Memory) implementation.

ROM is a memory region where writes are silently ignored. Real computers
have a ROM chip soldered to the motherboard containing firmware that
executes on power-on. The CPU's program counter starts at the ROM's base
address (0xFFFF0000 for our simulated machine).

Analogy: ROM is like a recipe card laminated in plastic. You can read it
any number of times, but you cannot write on it.

Memory map showing ROM's position::

    0xFFFF_FFFF +------------------+
                |    ROM (64 KB)   |  <- CPU starts here
    0xFFFF_0000 +------------------+
                |   Framebuffer    |
    0xFFFB_0000 +------------------+
                |       ...        |
    0x0001_0000 +------------------+
                |   Bootloader     |  <- BIOS jumps here
    0x0000_1000 +------------------+
                |   HardwareInfo   |  <- BIOS writes config here
    0x0000_0000 +------------------+
                |       IDT        |  <- Interrupt Descriptor Table
                +------------------+
"""

from __future__ import annotations

import struct
from dataclasses import dataclass


# Default base address: top of 32-bit address space minus 64 KB.
DEFAULT_ROM_BASE: int = 0xFFFF0000

# Default ROM size: 64 KB (65536 bytes).
DEFAULT_ROM_SIZE: int = 65536


@dataclass
class ROMConfig:
    """Configuration for the ROM memory region.

    Attributes:
        base_address: Start address (default: 0xFFFF0000).
        size: Size in bytes (default: 65536 = 64KB).
    """

    base_address: int = DEFAULT_ROM_BASE
    size: int = DEFAULT_ROM_SIZE


def DefaultROMConfig() -> ROMConfig:  # noqa: ANN201
    """Return the default ROM configuration.

    Base address: 0xFFFF0000, Size: 65536 (64KB).
    """
    return ROMConfig()


class ROM:
    """Read-only memory region.

    Once created with a firmware image, contents cannot be changed.
    Write operations are silently ignored, modeling real ROM chips
    that are programmed at the factory.

    Example::

        from rom_bios import BIOSFirmware, DefaultBIOSConfig, ROM, DefaultROMConfig
        bios = BIOSFirmware(DefaultBIOSConfig())
        rom = ROM(DefaultROMConfig(), bios.generate())

        # Reading works:
        first_byte = rom.read(0xFFFF0000)
        first_word = rom.read_word(0xFFFF0000)

        # Writing is silently ignored:
        rom.write(0xFFFF0000, 0xFF)
        # rom.read(0xFFFF0000) still returns the original byte
    """

    def __init__(self: ROM, config: ROMConfig, firmware: bytes | None = None) -> None:
        """Create a ROM loaded with the given firmware bytes.

        Args:
            config: ROM configuration (base address and size).
            firmware: Firmware bytes to load. If shorter than config.size,
                remaining bytes are zero-filled. If longer, raises ValueError.
        """
        if firmware is None:
            firmware = b""
        if len(firmware) > config.size:
            raise ValueError("firmware larger than ROM size")

        self._config = config
        # Copy firmware into a fixed-size buffer, zero-filled beyond firmware.
        self._data = bytearray(config.size)
        self._data[: len(firmware)] = firmware

    def read(self: ROM, address: int) -> int:
        """Return a single byte from the given absolute address.

        Out-of-range addresses return 0.
        """
        offset = self._address_to_offset(address)
        if offset < 0:
            return 0
        return self._data[offset]

    def read_word(self: ROM, address: int) -> int:
        """Return a 32-bit little-endian word at the given absolute address.

        This is the primary access pattern since RISC-V instructions
        are 32 bits wide.
        """
        offset = self._address_to_offset(address)
        if offset < 0 or offset + 3 >= len(self._data):
            return 0
        return struct.unpack_from("<I", self._data, offset)[0]

    def write(self: ROM, address: int, value: int) -> None:
        """Attempt to write a byte to ROM (silently ignored).

        ROM is read-only. This method exists so ROM can be used
        wherever a writable memory interface is expected, but
        writes have no effect.
        """

    def size(self: ROM) -> int:
        """Return the total size of the ROM in bytes."""
        return self._config.size

    def base_address(self: ROM) -> int:
        """Return the base address of the ROM."""
        return self._config.base_address

    def contains(self: ROM, address: int) -> bool:
        """Return True if the address falls within the ROM region."""
        return self._address_to_offset(address) >= 0

    def _address_to_offset(self: ROM, address: int) -> int:
        """Convert absolute address to byte offset. Returns -1 if out of range."""
        if address < self._config.base_address:
            return -1
        offset = address - self._config.base_address
        if offset >= self._config.size:
            return -1
        return offset
