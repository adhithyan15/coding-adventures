"""DeviceRegistry -- the kernel's phonebook for devices.

==========================================================================
Why a Registry?
==========================================================================

When the system boots, drivers create device objects and register them here.
Later, when the kernel needs to perform I/O (because a program called
sys_read or sys_write), it looks up the target device in the registry.

Without a registry, the kernel would need hardcoded references to every
device. Adding a new device type would require modifying kernel code.
With a registry, adding a new device type is just:
  1. Implement the driver (a subclass of CharacterDevice/BlockDevice/etc.)
  2. Create an instance
  3. Call registry.register(instance)

No kernel changes needed. This is the Open/Closed Principle in action:
the kernel is OPEN for extension (new device types) but CLOSED for
modification (no kernel code changes required).

==========================================================================
Two Lookup Strategies
==========================================================================

The registry supports two ways to find a device:

  1. By name: "disk0" -> SimulatedDisk instance
     Human-friendly, used by programs and shell commands.

  2. By (major, minor): (3, 0) -> SimulatedDisk instance
     Machine-friendly, used by the kernel's file descriptor table.
     The kernel stores (major, minor) pairs in inodes, not string names.

Both are O(1) lookups using dictionaries (hash maps).
"""

from __future__ import annotations

from device_driver_framework.device import Device, DeviceType


class DeviceRegistry:
    """Central registry for all devices in the system.

    The registry provides fast lookup by name and by (major, minor) pair.
    It also supports listing all devices or filtering by device type.

    Usage:
        registry = DeviceRegistry()
        disk = SimulatedDisk(total_blocks=2048)
        disk.init()
        registry.register(disk)

        # Later, when handling sys_read:
        device = registry.lookup("disk0")
        if device and device.device_type == DeviceType.BLOCK:
            data = device.read_block(block_num)
    """

    def __init__(self) -> None:
        # _by_name: fast lookup by human-readable name
        # Why a dict? Because looking up "disk0" needs to be O(1).
        # The kernel does this on every syscall, so it must be fast.
        self._by_name: dict[str, Device] = {}

        # _by_major_minor: fast lookup by (major, minor) number pair
        # The kernel's inode table stores major/minor numbers, not names.
        # When a program opens /dev/sda, the kernel reads the inode to get
        # (major=3, minor=0) and uses this map to find the driver.
        self._by_major_minor: dict[tuple[int, int], Device] = {}

    def register(self, device: Device) -> None:
        """Register a device in the registry.

        The device must have a unique name and a unique (major, minor) pair.
        Attempting to register a device with a duplicate name or duplicate
        (major, minor) raises a ValueError.

        Args:
            device: The device to register.

        Raises:
            ValueError: If a device with the same name or same (major, minor)
                        is already registered.
        """
        if device.name in self._by_name:
            raise ValueError(
                f"Device with name {device.name!r} is already registered"
            )

        key = (device.major, device.minor)
        if key in self._by_major_minor:
            existing = self._by_major_minor[key]
            raise ValueError(
                f"Device with major={device.major}, minor={device.minor} "
                f"is already registered as {existing.name!r}"
            )

        self._by_name[device.name] = device
        self._by_major_minor[key] = device

    def unregister(self, name: str) -> Device | None:
        """Remove a device from the registry by name.

        Returns the removed device, or None if no device with that name
        was registered.

        This is used when hot-unplugging a device (e.g., removing a USB
        drive) or during shutdown when cleaning up.
        """
        device = self._by_name.pop(name, None)
        if device is not None:
            self._by_major_minor.pop((device.major, device.minor), None)
        return device

    def lookup(self, name: str) -> Device | None:
        """Look up a device by its human-readable name.

        Returns the device, or None if not found.
        This is the most common lookup -- used when the kernel resolves
        a filename like "/dev/disk0" to a device object.
        """
        return self._by_name.get(name)

    def lookup_by_major_minor(self, major: int, minor: int) -> Device | None:
        """Look up a device by its (major, minor) number pair.

        Returns the device, or None if not found.
        This is used internally by the kernel when it has an inode
        (which stores major/minor) but not a name.
        """
        return self._by_major_minor.get((major, minor))

    def list_devices(self) -> list[Device]:
        """Return all registered devices.

        The order is not guaranteed (dict insertion order in CPython 3.7+,
        but we don't rely on it for correctness).
        """
        return list(self._by_name.values())

    def list_by_type(self, device_type: DeviceType) -> list[Device]:
        """Return all devices of a specific type.

        For example, list_by_type(DeviceType.BLOCK) returns all disk-like
        devices. This is useful for commands like "list all disks" or
        "find all network interfaces."
        """
        return [d for d in self._by_name.values() if d.device_type == device_type]
