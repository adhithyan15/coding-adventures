"""S02 Bootloader -- generates RISC-V machine code for the second stage of
the boot sequence: loading the OS kernel from disk into RAM.

=== What is a bootloader? ===

A bootloader is a small program that runs after the BIOS (S01) finishes
hardware initialization. Its job is deceptively simple but critical:

  1. Read the boot protocol left by the BIOS at 0x00001000
  2. Validate the magic number (0xB007CAFE) to ensure BIOS ran correctly
  3. Copy the kernel binary from the "disk" region into kernel RAM
  4. Set the stack pointer for the kernel
  5. Jump to the kernel entry point (0x00020000)

=== Memory Map ===

    0x00001000: Boot protocol (written by BIOS, read by bootloader)
    0x00010000: Bootloader code (this package generates these bytes)
    0x00020000: Kernel code (bootloader copies kernel here)
    0x0006FFF0: Kernel stack pointer (bootloader sets SP here)
    0x10000000: Disk image base (memory-mapped disk region)
    0x10080000: Kernel on disk (disk base + kernel offset)
"""

from bootloader.bootloader import (
    AnnotatedInstruction,
    Bootloader,
    BootloaderConfig,
    DefaultBootloaderConfig,
)
from bootloader.disk_image import DiskImage

# Well-known addresses and constants
DEFAULT_ENTRY_ADDRESS: int = 0x00010000
DEFAULT_KERNEL_DISK_OFFSET: int = 0x00080000
DEFAULT_KERNEL_LOAD_ADDRESS: int = 0x00020000
DEFAULT_STACK_BASE: int = 0x0006FFF0
DISK_MEMORY_MAP_BASE: int = 0x10000000
BOOT_PROTOCOL_ADDRESS: int = 0x00001000
BOOT_PROTOCOL_MAGIC: int = 0xB007CAFE

# Disk image constants
DISK_BOOT_SECTOR_OFFSET: int = 0x00000000
DISK_BOOT_SECTOR_SIZE: int = 512
DISK_KERNEL_OFFSET: int = 0x00080000
DISK_USER_PROGRAM_BASE: int = 0x00100000
DEFAULT_DISK_SIZE: int = 2 * 1024 * 1024

__all__ = [
    "AnnotatedInstruction",
    "Bootloader",
    "BootloaderConfig",
    "DefaultBootloaderConfig",
    "DiskImage",
    "BOOT_PROTOCOL_ADDRESS",
    "BOOT_PROTOCOL_MAGIC",
    "DEFAULT_DISK_SIZE",
    "DEFAULT_ENTRY_ADDRESS",
    "DEFAULT_KERNEL_DISK_OFFSET",
    "DEFAULT_KERNEL_LOAD_ADDRESS",
    "DEFAULT_STACK_BASE",
    "DISK_BOOT_SECTOR_OFFSET",
    "DISK_BOOT_SECTOR_SIZE",
    "DISK_KERNEL_OFFSET",
    "DISK_MEMORY_MAP_BASE",
    "DISK_USER_PROGRAM_BASE",
]
