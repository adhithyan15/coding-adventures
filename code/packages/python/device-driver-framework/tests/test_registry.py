"""Tests for the DeviceRegistry."""

import pytest

from device_driver_framework.device import Device, DeviceType
from device_driver_framework.registry import DeviceRegistry


# =========================================================================
# Helper: create simple test devices
# =========================================================================


def make_device(
    name: str,
    dtype: DeviceType = DeviceType.CHARACTER,
    major: int = 1,
    minor: int = 0,
) -> Device:
    """Create a simple Device for testing."""
    return Device(name, dtype, major, minor)


# =========================================================================
# Registration and Lookup Tests
# =========================================================================


class TestDeviceRegistry:
    """Verify register, lookup, unregister, and listing operations."""

    def test_register_and_lookup_by_name(self) -> None:
        """register() + lookup() should round-trip."""
        reg = DeviceRegistry()
        dev = make_device("display0")
        reg.register(dev)
        assert reg.lookup("display0") is dev

    def test_register_and_lookup_by_major_minor(self) -> None:
        """register() + lookup_by_major_minor() should round-trip."""
        reg = DeviceRegistry()
        dev = make_device("display0", major=1, minor=0)
        reg.register(dev)
        assert reg.lookup_by_major_minor(1, 0) is dev

    def test_lookup_nonexistent_name(self) -> None:
        """Looking up a name that does not exist should return None."""
        reg = DeviceRegistry()
        assert reg.lookup("nonexistent") is None

    def test_lookup_nonexistent_major_minor(self) -> None:
        """Looking up an unregistered (major, minor) should return None."""
        reg = DeviceRegistry()
        assert reg.lookup_by_major_minor(99, 99) is None

    def test_duplicate_name_raises(self) -> None:
        """Registering two devices with the same name should raise ValueError."""
        reg = DeviceRegistry()
        reg.register(make_device("dup", major=1, minor=0))
        with pytest.raises(ValueError, match="already registered"):
            reg.register(make_device("dup", major=2, minor=0))

    def test_duplicate_major_minor_raises(self) -> None:
        """Registering two devices with the same (major, minor) should raise."""
        reg = DeviceRegistry()
        reg.register(make_device("dev_a", major=3, minor=0))
        with pytest.raises(ValueError, match="already registered"):
            reg.register(make_device("dev_b", major=3, minor=0))

    def test_unregister_existing(self) -> None:
        """unregister() should remove the device and return it."""
        reg = DeviceRegistry()
        dev = make_device("disk0", major=3, minor=0)
        reg.register(dev)
        removed = reg.unregister("disk0")
        assert removed is dev
        assert reg.lookup("disk0") is None
        assert reg.lookup_by_major_minor(3, 0) is None

    def test_unregister_nonexistent(self) -> None:
        """unregister() on a missing name should return None."""
        reg = DeviceRegistry()
        assert reg.unregister("ghost") is None

    def test_list_devices(self) -> None:
        """list_devices() should return all registered devices."""
        reg = DeviceRegistry()
        d1 = make_device("a", major=1, minor=0)
        d2 = make_device("b", major=2, minor=0)
        d3 = make_device("c", major=3, minor=0)
        reg.register(d1)
        reg.register(d2)
        reg.register(d3)
        devices = reg.list_devices()
        assert len(devices) == 3
        assert set(d.name for d in devices) == {"a", "b", "c"}

    def test_list_devices_empty(self) -> None:
        """list_devices() on empty registry should return []."""
        reg = DeviceRegistry()
        assert reg.list_devices() == []

    def test_list_by_type(self) -> None:
        """list_by_type() should filter by device type."""
        reg = DeviceRegistry()
        char_dev = make_device("kb0", DeviceType.CHARACTER, major=2, minor=0)
        block_dev = make_device("disk0", DeviceType.BLOCK, major=3, minor=0)
        net_dev = make_device("nic0", DeviceType.NETWORK, major=4, minor=0)
        reg.register(char_dev)
        reg.register(block_dev)
        reg.register(net_dev)

        char_list = reg.list_by_type(DeviceType.CHARACTER)
        assert len(char_list) == 1
        assert char_list[0] is char_dev

        block_list = reg.list_by_type(DeviceType.BLOCK)
        assert len(block_list) == 1
        assert block_list[0] is block_dev

        net_list = reg.list_by_type(DeviceType.NETWORK)
        assert len(net_list) == 1
        assert net_list[0] is net_dev

    def test_list_by_type_empty(self) -> None:
        """list_by_type() should return [] if no devices of that type exist."""
        reg = DeviceRegistry()
        reg.register(make_device("kb0", DeviceType.CHARACTER, major=2, minor=0))
        assert reg.list_by_type(DeviceType.BLOCK) == []

    def test_register_after_unregister(self) -> None:
        """After unregistering, the same name and (major,minor) can be reused."""
        reg = DeviceRegistry()
        dev1 = make_device("disk0", major=3, minor=0)
        reg.register(dev1)
        reg.unregister("disk0")
        dev2 = make_device("disk0", major=3, minor=0)
        reg.register(dev2)
        assert reg.lookup("disk0") is dev2

    def test_multiple_devices_same_type(self) -> None:
        """Multiple devices of the same type should all appear in list_by_type."""
        reg = DeviceRegistry()
        d1 = make_device("disk0", DeviceType.BLOCK, major=3, minor=0)
        d2 = make_device("disk1", DeviceType.BLOCK, major=3, minor=1)
        reg.register(d1)
        reg.register(d2)
        blocks = reg.list_by_type(DeviceType.BLOCK)
        assert len(blocks) == 2
