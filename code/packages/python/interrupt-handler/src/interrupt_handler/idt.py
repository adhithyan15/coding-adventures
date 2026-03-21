"""Interrupt Descriptor Table (IDT) — maps interrupt numbers to ISR addresses.

The IDT is an array of 256 entries stored at address 0x00000000 in memory.
Each entry maps an interrupt number to the address of its handler (ISR).
The BIOS (S01) populates the IDT during boot; the kernel (S04) may modify
entries later.

IDT Layout in Memory (at 0x00000000):
    Entry 0:   Division by zero handler         8 bytes
    Entry 1:   Debug exception handler          8 bytes
    ...
    Entry 32:  Timer interrupt handler          8 bytes
    Entry 33:  Keyboard interrupt handler       8 bytes
    ...
    Entry 128: System call handler              8 bytes
    ...
    Entry 255: (last entry)                     8 bytes
    Total: 256 entries x 8 bytes = 2,048 bytes (2 KB)

Each IDT Entry (8 bytes):
    Bytes 0-3: ISR address (little-endian uint32)
    Byte 4:    Present (0x00 or 0x01)
    Byte 5:    Privilege level (0x00 = kernel)
    Bytes 6-7: Reserved (0x00, 0x00)
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field

# ----- Constants -----

IDT_ENTRY_SIZE = 8
"""Each IDT entry occupies 8 bytes in memory."""

IDT_SIZE = 256 * IDT_ENTRY_SIZE  # 2048 bytes
"""Total IDT size: 256 entries * 8 bytes."""

IDT_BASE_ADDRESS = 0x00000000
"""Default memory location of the IDT."""


# ----- IDT Entry -----


@dataclass
class IDTEntry:
    """One row in the Interrupt Descriptor Table.

    Attributes:
        isr_address: Where the CPU jumps when this interrupt fires.
        present: True if this entry is valid. False triggers double fault.
        privilege_level: 0 = kernel only.
    """

    isr_address: int = 0
    present: bool = False
    privilege_level: int = 0


# ----- Interrupt Descriptor Table -----


@dataclass
class InterruptDescriptorTable:
    """256-entry table mapping interrupt numbers to ISR addresses.

    Why 256 entries? Matches x86 convention:
        0-31:   CPU exceptions (division by zero, invalid opcode, page fault)
        32-47:  Hardware device interrupts (timer, keyboard)
        48-127: Available
        128:    System call (ecall)
        129-255: Available
    """

    entries: list[IDTEntry] = field(
        default_factory=lambda: [IDTEntry() for _ in range(256)]
    )

    def set_entry(self: InterruptDescriptorTable, number: int, entry: IDTEntry) -> None:
        """Install a handler at the given interrupt number (0-255)."""
        if number < 0 or number > 255:
            raise ValueError("IDT entry number must be 0-255")
        self.entries[number] = entry

    def get_entry(self: InterruptDescriptorTable, number: int) -> IDTEntry:
        """Return the entry for the given interrupt number (0-255)."""
        if number < 0 or number > 255:
            raise ValueError("IDT entry number must be 0-255")
        return self.entries[number]

    def write_to_memory(
        self: InterruptDescriptorTable, memory: bytearray, base_address: int
    ) -> None:
        """Serialize the IDT into a bytearray at the given base address.

        Each entry occupies 8 bytes in little-endian format:
            Offset 0-3: ISR address (uint32, little-endian)
            Offset 4:   Present bit (0x00 or 0x01)
            Offset 5:   Privilege level (uint8)
            Offset 6-7: Reserved (zeroed)
        """
        for i in range(256):
            offset = base_address + i * IDT_ENTRY_SIZE
            entry = self.entries[i]

            # Bytes 0-3: ISR address (little-endian)
            struct.pack_into("<I", memory, offset, entry.isr_address & 0xFFFFFFFF)

            # Byte 4: Present bit
            memory[offset + 4] = 0x01 if entry.present else 0x00

            # Byte 5: Privilege level
            memory[offset + 5] = entry.privilege_level & 0xFF

            # Bytes 6-7: Reserved
            memory[offset + 6] = 0x00
            memory[offset + 7] = 0x00

    def load_from_memory(
        self: InterruptDescriptorTable, memory: bytes | bytearray, base_address: int
    ) -> None:
        """Deserialize the IDT from a byte sequence at the given base address."""
        for i in range(256):
            offset = base_address + i * IDT_ENTRY_SIZE

            # Bytes 0-3: ISR address (little-endian)
            (isr_address,) = struct.unpack_from("<I", memory, offset)

            # Byte 4: Present bit
            present = memory[offset + 4] != 0x00

            # Byte 5: Privilege level
            privilege_level = memory[offset + 5]

            self.entries[i] = IDTEntry(
                isr_address=isr_address,
                present=present,
                privilege_level=privilege_level,
            )
