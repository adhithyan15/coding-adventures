"""System configuration and address space constants."""

from __future__ import annotations

from dataclasses import dataclass, field

from bootloader import BootloaderConfig, DefaultBootloaderConfig
from display import DisplayConfig, DefaultDisplayConfig
from os_kernel import DefaultKernelConfig, KernelConfig
from rom_bios import BIOSConfig, DefaultBIOSConfig

# Address space constants
ROM_BASE: int = 0xFFFF0000
ROM_SIZE: int = 0x00010000
BOOT_PROTOCOL_ADDR: int = 0x00001000
BOOTLOADER_BASE: int = 0x00010000
KERNEL_BASE: int = 0x00020000
IDLE_PROCESS_BASE: int = 0x00030000
USER_PROCESS_BASE: int = 0x00040000
KERNEL_STACK_TOP: int = 0x0006FFF0
DISK_MAPPED_BASE: int = 0x10000000
FRAMEBUFFER_BASE: int = 0xFFFB0000
KEYBOARD_PORT: int = 0xFFFC0000


@dataclass
class SystemConfig:
    """All configuration for the complete simulated computer."""

    memory_size: int = 1024 * 1024
    display_config: DisplayConfig = field(default_factory=DisplayConfig)
    bios_config: BIOSConfig = field(default_factory=DefaultBIOSConfig)
    bootloader_config: BootloaderConfig = field(default_factory=DefaultBootloaderConfig)
    kernel_config: KernelConfig = field(default_factory=DefaultKernelConfig)
    user_program: bytes | None = None


def DefaultSystemConfig() -> SystemConfig:  # noqa: ANN201, N802
    """Return sensible defaults for the hello-world demo."""
    bios_config = DefaultBIOSConfig()
    bios_config.memory_size = 1024 * 1024

    return SystemConfig(
        memory_size=1024 * 1024,
        display_config=DefaultDisplayConfig(),
        bios_config=bios_config,
        bootloader_config=DefaultBootloaderConfig(),
        kernel_config=DefaultKernelConfig(),
        user_program=None,
    )
