"""DiskImage -- simulated persistent storage.

Real computers have hard drives, SSDs, or floppy disks. Our simulated
computer has a DiskImage -- a bytearray that acts as persistent storage.
The disk image is pre-loaded with the kernel binary and (optionally) user
program binaries before the system powers on.

Disk Layout:
    Offset 0x00000000: Boot sector (512 bytes, unused in our system)
    Offset 0x00080000: Kernel binary (default location)
    Offset 0x00100000: User program area
"""

from __future__ import annotations

DISK_BOOT_SECTOR_OFFSET: int = 0x00000000
DISK_BOOT_SECTOR_SIZE: int = 512
DISK_KERNEL_OFFSET: int = 0x00080000
DISK_USER_PROGRAM_BASE: int = 0x00100000
DEFAULT_DISK_SIZE: int = 2 * 1024 * 1024


class DiskImage:
    """Simulates persistent storage (hard drive / SSD).

    Pre-loaded with kernel and user program binaries before boot.
    """

    def __init__(self: DiskImage, size_bytes: int = DEFAULT_DISK_SIZE) -> None:
        """Create an empty disk image of the given size."""
        self._data = bytearray(size_bytes)

    def load_kernel(self: DiskImage, kernel_binary: bytes) -> None:
        """Write a kernel binary to the conventional disk offset (0x00080000)."""
        self.load_at(DISK_KERNEL_OFFSET, kernel_binary)

    def load_user_program(self: DiskImage, program_binary: bytes, offset: int) -> None:
        """Write a user program binary at a specified disk offset."""
        self.load_at(offset, program_binary)

    def load_at(self: DiskImage, offset: int, data: bytes) -> None:
        """Write raw bytes at a specific offset within the disk image."""
        if offset + len(data) > len(self._data):
            msg = "DiskImage: data exceeds disk size"
            raise ValueError(msg)
        self._data[offset : offset + len(data)] = data

    def read_word(self: DiskImage, offset: int) -> int:
        """Read a 32-bit little-endian word at the given disk offset."""
        if offset < 0 or offset + 4 > len(self._data):
            return 0
        return (
            self._data[offset]
            | (self._data[offset + 1] << 8)
            | (self._data[offset + 2] << 16)
            | (self._data[offset + 3] << 24)
        )

    def read_byte_at(self: DiskImage, offset: int) -> int:
        """Read a single byte at the given disk offset."""
        if offset < 0 or offset >= len(self._data):
            return 0
        return self._data[offset]

    def data(self: DiskImage) -> bytearray:
        """Return the raw byte array for memory-mapping."""
        return self._data

    def size(self: DiskImage) -> int:
        """Return the total size of the disk image in bytes."""
        return len(self._data)
