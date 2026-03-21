"""HardwareInfo -- the boot protocol structure written by BIOS.

When the BIOS finishes initializing hardware, it leaves a status report
at a well-known memory address (0x00001000). This struct tells the
bootloader and kernel everything about the hardware configuration.

Memory layout at 0x00001000 (28 bytes total, all little-endian uint32)::

    Offset  Field              Default
    ------  -----------------  ----------
    0x00    memory_size        (probed)
    0x04    display_columns    80
    0x08    display_rows       25
    0x0C    framebuffer_base   0xFFFB0000
    0x10    idt_base           0x00000000
    0x14    idt_entries        256
    0x18    bootloader_entry   0x00010000
"""

from __future__ import annotations

import struct
from dataclasses import dataclass

# Fixed memory address where BIOS writes the HardwareInfo struct.
HARDWARE_INFO_ADDRESS: int = 0x00001000

# Size of the HardwareInfo struct: 7 fields * 4 bytes = 28 bytes.
HARDWARE_INFO_SIZE: int = 28


@dataclass
class HardwareInfo:
    """Hardware configuration discovered and set by the BIOS.

    Attributes:
        memory_size: Total RAM in bytes (discovered by memory probe).
        display_columns: Text display width (default: 80).
        display_rows: Text display height (default: 25).
        framebuffer_base: Framebuffer start address (default: 0xFFFB0000).
        idt_base: IDT start address (default: 0x00000000).
        idt_entries: Number of IDT entries (default: 256).
        bootloader_entry: Where to jump after BIOS (default: 0x00010000).
    """

    memory_size: int = 0
    display_columns: int = 80
    display_rows: int = 25
    framebuffer_base: int = 0xFFFB0000
    idt_base: int = 0x00000000
    idt_entries: int = 256
    bootloader_entry: int = 0x00010000

    def to_bytes(self: HardwareInfo) -> bytes:
        """Serialize to 28-byte little-endian buffer."""
        return struct.pack(
            "<7I",
            self.memory_size,
            self.display_columns,
            self.display_rows,
            self.framebuffer_base,
            self.idt_base,
            self.idt_entries,
            self.bootloader_entry,
        )

    @classmethod
    def from_bytes(cls: type[HardwareInfo], data: bytes) -> HardwareInfo:
        """Deserialize from a 28-byte little-endian buffer."""
        if len(data) < HARDWARE_INFO_SIZE:
            raise ValueError("data too short for HardwareInfo")
        fields = struct.unpack_from("<7I", data)
        return cls(
            memory_size=fields[0],
            display_columns=fields[1],
            display_rows=fields[2],
            framebuffer_base=fields[3],
            idt_base=fields[4],
            idt_entries=fields[5],
            bootloader_entry=fields[6],
        )
