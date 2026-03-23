"""S06 System Board -- the complete simulated computer.

Composes ROM/BIOS, Bootloader, Interrupt Handler, OS Kernel, Display,
and a RISC-V CPU into a complete system that boots to Hello World.

    PowerOn() -> BIOS -> Bootloader -> Kernel -> Hello World -> Idle
"""

from system_board.board import SystemBoard
from system_board.boot_trace import BootEvent, BootPhase, BootTrace
from system_board.config import (
    BOOT_PROTOCOL_ADDR,
    BOOTLOADER_BASE,
    DISK_MAPPED_BASE,
    FRAMEBUFFER_BASE,
    IDLE_PROCESS_BASE,
    KERNEL_BASE,
    KERNEL_STACK_TOP,
    KEYBOARD_PORT,
    ROM_BASE,
    ROM_SIZE,
    USER_PROCESS_BASE,
    DefaultSystemConfig,
    SystemConfig,
)

__all__ = [
    "SystemBoard",
    "BootEvent",
    "BootPhase",
    "BootTrace",
    "SystemConfig",
    "DefaultSystemConfig",
    "BOOT_PROTOCOL_ADDR",
    "BOOTLOADER_BASE",
    "DISK_MAPPED_BASE",
    "FRAMEBUFFER_BASE",
    "IDLE_PROCESS_BASE",
    "KERNEL_BASE",
    "KERNEL_STACK_TOP",
    "KEYBOARD_PORT",
    "ROM_BASE",
    "ROM_SIZE",
    "USER_PROCESS_BASE",
]
