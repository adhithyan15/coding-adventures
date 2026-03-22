"""D12 Device Driver Framework -- unified abstraction for character, block, and
network devices.

Every operating system faces the same fundamental challenge: the kernel needs to
communicate with dozens of different hardware devices (keyboards, disks, network
cards, displays), but each device speaks a different protocol with different
timing, different register layouts, and different data formats.

Device drivers solve this by inserting a translation layer between the kernel
and the hardware. The kernel speaks a small set of well-defined protocols
(read bytes, write blocks, send packets), and each driver translates those
generic operations into the specific commands its hardware understands.

This package provides:
  - Three device family base classes (CharacterDevice, BlockDevice, NetworkDevice)
  - A DeviceRegistry for the kernel to discover and look up devices
  - Simulated implementations of common devices for our educational computer

Analogy: Device drivers are like a universal remote control. You press
"Volume Up" and it works on any TV brand. Each TV speaks a different infrared
protocol, but the remote translates your single button press into the right
signal for each brand. The kernel is the person pressing the button; the driver
is the remote; the hardware is the TV.
"""

from device_driver_framework.device import (
    BlockDevice,
    CharacterDevice,
    Device,
    DeviceType,
    NetworkDevice,
)
from device_driver_framework.registry import DeviceRegistry
from device_driver_framework.shared_wire import SharedWire
from device_driver_framework.simulated_disk import SimulatedDisk
from device_driver_framework.simulated_display import SimulatedDisplay
from device_driver_framework.simulated_keyboard import SimulatedKeyboard
from device_driver_framework.simulated_nic import SimulatedNIC

# Well-known interrupt numbers for devices (matching the spec)
INT_TIMER = 32
INT_KEYBOARD = 33
INT_DISK = 34
INT_NIC = 35

# Well-known major numbers for our simulated devices
MAJOR_DISPLAY = 1
MAJOR_KEYBOARD = 2
MAJOR_DISK = 3
MAJOR_NIC = 4

__all__ = [
    # Core types
    "DeviceType",
    "Device",
    "CharacterDevice",
    "BlockDevice",
    "NetworkDevice",
    # Registry
    "DeviceRegistry",
    # Simulated devices
    "SimulatedDisk",
    "SimulatedKeyboard",
    "SimulatedDisplay",
    "SimulatedNIC",
    "SharedWire",
    # Constants
    "INT_TIMER",
    "INT_KEYBOARD",
    "INT_DISK",
    "INT_NIC",
    "MAJOR_DISPLAY",
    "MAJOR_KEYBOARD",
    "MAJOR_DISK",
    "MAJOR_NIC",
]
