"""Tests for the core device abstractions: DeviceType, Device, and subclasses."""

from device_driver_framework.device import (
    BlockDevice,
    CharacterDevice,
    Device,
    DeviceType,
    NetworkDevice,
)


# =========================================================================
# DeviceType Enum Tests
# =========================================================================


class TestDeviceType:
    """Verify that the DeviceType enum has the expected members and values."""

    def test_character_value(self) -> None:
        """CHARACTER should be 0."""
        assert DeviceType.CHARACTER == 0

    def test_block_value(self) -> None:
        """BLOCK should be 1."""
        assert DeviceType.BLOCK == 1

    def test_network_value(self) -> None:
        """NETWORK should be 2."""
        assert DeviceType.NETWORK == 2

    def test_all_types_distinct(self) -> None:
        """All three types must have distinct values."""
        values = [DeviceType.CHARACTER, DeviceType.BLOCK, DeviceType.NETWORK]
        assert len(set(values)) == 3

    def test_enum_members_count(self) -> None:
        """There should be exactly three device types."""
        assert len(DeviceType) == 3


# =========================================================================
# Device Base Class Tests
# =========================================================================


class TestDevice:
    """Verify that the Device base class stores fields correctly."""

    def test_fields_stored(self) -> None:
        """All constructor arguments should be accessible as attributes."""
        dev = Device("test0", DeviceType.CHARACTER, major=1, minor=0, interrupt_number=33)
        assert dev.name == "test0"
        assert dev.device_type == DeviceType.CHARACTER
        assert dev.major == 1
        assert dev.minor == 0
        assert dev.interrupt_number == 33
        assert dev.initialized is False

    def test_default_interrupt_number(self) -> None:
        """Default interrupt_number should be -1 (no interrupt)."""
        dev = Device("test0", DeviceType.BLOCK, major=3, minor=0)
        assert dev.interrupt_number == -1

    def test_init_sets_initialized(self) -> None:
        """Calling init() should set the initialized flag to True."""
        dev = Device("test0", DeviceType.CHARACTER, major=1, minor=0)
        assert dev.initialized is False
        dev.init()
        assert dev.initialized is True

    def test_repr(self) -> None:
        """repr should include name, type, major, minor, and irq."""
        dev = Device("disk0", DeviceType.BLOCK, major=3, minor=0, interrupt_number=34)
        r = repr(dev)
        assert "disk0" in r
        assert "BLOCK" in r
        assert "major=3" in r
        assert "minor=0" in r
        assert "irq=34" in r


# =========================================================================
# CharacterDevice Tests
# =========================================================================


class TestCharacterDevice:
    """Verify CharacterDevice sets device_type and raises on abstract methods."""

    def test_device_type_is_character(self) -> None:
        """CharacterDevice should always have device_type = CHARACTER."""
        dev = CharacterDevice("kb0", major=2, minor=0)
        assert dev.device_type == DeviceType.CHARACTER

    def test_read_not_implemented(self) -> None:
        """Base CharacterDevice.read() should raise NotImplementedError."""
        dev = CharacterDevice("kb0", major=2, minor=0)
        try:
            dev.read(10)
            assert False, "Should have raised NotImplementedError"
        except NotImplementedError:
            pass

    def test_write_not_implemented(self) -> None:
        """Base CharacterDevice.write() should raise NotImplementedError."""
        dev = CharacterDevice("kb0", major=2, minor=0)
        try:
            dev.write(b"hello")
            assert False, "Should have raised NotImplementedError"
        except NotImplementedError:
            pass


# =========================================================================
# BlockDevice Tests
# =========================================================================


class TestBlockDevice:
    """Verify BlockDevice sets device_type and has block_size/total_blocks."""

    def test_device_type_is_block(self) -> None:
        """BlockDevice should always have device_type = BLOCK."""
        dev = BlockDevice("disk0", major=3, minor=0)
        assert dev.device_type == DeviceType.BLOCK

    def test_default_block_size(self) -> None:
        """Default block_size should be 512."""
        dev = BlockDevice("disk0", major=3, minor=0)
        assert dev.block_size == 512

    def test_custom_block_size(self) -> None:
        """block_size should be configurable."""
        dev = BlockDevice("disk0", major=3, minor=0, block_size=4096)
        assert dev.block_size == 4096

    def test_total_blocks(self) -> None:
        """total_blocks should be configurable."""
        dev = BlockDevice("disk0", major=3, minor=0, total_blocks=2048)
        assert dev.total_blocks == 2048

    def test_read_block_not_implemented(self) -> None:
        """Base BlockDevice.read_block() should raise NotImplementedError."""
        dev = BlockDevice("disk0", major=3, minor=0)
        try:
            dev.read_block(0)
            assert False, "Should have raised NotImplementedError"
        except NotImplementedError:
            pass

    def test_write_block_not_implemented(self) -> None:
        """Base BlockDevice.write_block() should raise NotImplementedError."""
        dev = BlockDevice("disk0", major=3, minor=0)
        try:
            dev.write_block(0, b"\x00" * 512)
            assert False, "Should have raised NotImplementedError"
        except NotImplementedError:
            pass


# =========================================================================
# NetworkDevice Tests
# =========================================================================


class TestNetworkDevice:
    """Verify NetworkDevice sets device_type and has mac_address."""

    def test_device_type_is_network(self) -> None:
        """NetworkDevice should always have device_type = NETWORK."""
        mac = b"\xDE\xAD\xBE\xEF\x00\x01"
        dev = NetworkDevice("nic0", major=4, minor=0, mac_address=mac)
        assert dev.device_type == DeviceType.NETWORK

    def test_mac_address_stored(self) -> None:
        """mac_address should be stored and retrievable."""
        mac = b"\xDE\xAD\xBE\xEF\x00\x01"
        dev = NetworkDevice("nic0", major=4, minor=0, mac_address=mac)
        assert dev.mac_address == mac
        assert len(dev.mac_address) == 6

    def test_send_packet_not_implemented(self) -> None:
        """Base NetworkDevice.send_packet() should raise NotImplementedError."""
        mac = b"\xDE\xAD\xBE\xEF\x00\x01"
        dev = NetworkDevice("nic0", major=4, minor=0, mac_address=mac)
        try:
            dev.send_packet(b"hello")
            assert False, "Should have raised NotImplementedError"
        except NotImplementedError:
            pass

    def test_receive_packet_not_implemented(self) -> None:
        """Base NetworkDevice.receive_packet() should raise NotImplementedError."""
        mac = b"\xDE\xAD\xBE\xEF\x00\x01"
        dev = NetworkDevice("nic0", major=4, minor=0, mac_address=mac)
        try:
            dev.receive_packet()
            assert False, "Should have raised NotImplementedError"
        except NotImplementedError:
            pass

    def test_has_packet_not_implemented(self) -> None:
        """Base NetworkDevice.has_packet() should raise NotImplementedError."""
        mac = b"\xDE\xAD\xBE\xEF\x00\x01"
        dev = NetworkDevice("nic0", major=4, minor=0, mac_address=mac)
        try:
            dev.has_packet()
            assert False, "Should have raised NotImplementedError"
        except NotImplementedError:
            pass
