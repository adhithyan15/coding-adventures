"""Tests for SimulatedDisk."""

import pytest

from device_driver_framework.device import DeviceType
from device_driver_framework.simulated_disk import SimulatedDisk


class TestSimulatedDisk:
    """Verify SimulatedDisk read/write operations and edge cases."""

    def test_default_configuration(self) -> None:
        """Default disk should be 1 MB (2048 blocks of 512 bytes)."""
        disk = SimulatedDisk()
        assert disk.name == "disk0"
        assert disk.device_type == DeviceType.BLOCK
        assert disk.major == 3
        assert disk.minor == 0
        assert disk.block_size == 512
        assert disk.total_blocks == 2048
        assert disk.interrupt_number == 34

    def test_custom_configuration(self) -> None:
        """Disk parameters should be configurable."""
        disk = SimulatedDisk(
            name="disk1", minor=1, block_size=1024, total_blocks=100
        )
        assert disk.name == "disk1"
        assert disk.minor == 1
        assert disk.block_size == 1024
        assert disk.total_blocks == 100

    def test_init_zeros_storage(self) -> None:
        """After init(), all blocks should be zeroed."""
        disk = SimulatedDisk(total_blocks=4)
        # Write some data first
        disk.write_block(0, b"\xFF" * 512)
        # Init should zero everything
        disk.init()
        assert disk.initialized is True
        data = disk.read_block(0)
        assert data == b"\x00" * 512

    def test_read_block_fresh_disk(self) -> None:
        """Reading from a fresh disk should return all zeros."""
        disk = SimulatedDisk(total_blocks=4)
        data = disk.read_block(0)
        assert data == b"\x00" * 512
        assert len(data) == 512

    def test_write_then_read(self) -> None:
        """Writing data and reading it back should return the same data."""
        disk = SimulatedDisk(total_blocks=4)
        payload = bytes(range(256)) + bytes(range(256))  # 512 bytes
        disk.write_block(2, payload)
        assert disk.read_block(2) == payload

    def test_write_does_not_affect_other_blocks(self) -> None:
        """Writing to one block should not affect adjacent blocks."""
        disk = SimulatedDisk(total_blocks=4)
        disk.write_block(1, b"\xAA" * 512)
        assert disk.read_block(0) == b"\x00" * 512
        assert disk.read_block(2) == b"\x00" * 512

    def test_read_block_out_of_range(self) -> None:
        """Reading past the last block should raise ValueError."""
        disk = SimulatedDisk(total_blocks=4)
        with pytest.raises(ValueError, match="out of range"):
            disk.read_block(4)

    def test_read_block_negative(self) -> None:
        """Reading a negative block number should raise ValueError."""
        disk = SimulatedDisk(total_blocks=4)
        with pytest.raises(ValueError, match="out of range"):
            disk.read_block(-1)

    def test_write_block_out_of_range(self) -> None:
        """Writing past the last block should raise ValueError."""
        disk = SimulatedDisk(total_blocks=4)
        with pytest.raises(ValueError, match="out of range"):
            disk.write_block(4, b"\x00" * 512)

    def test_write_block_wrong_size(self) -> None:
        """Writing data that is not exactly block_size should raise ValueError."""
        disk = SimulatedDisk(total_blocks=4)
        with pytest.raises(ValueError, match="exactly 512 bytes"):
            disk.write_block(0, b"\x00" * 100)

    def test_write_block_negative(self) -> None:
        """Writing to a negative block number should raise ValueError."""
        disk = SimulatedDisk(total_blocks=4)
        with pytest.raises(ValueError, match="out of range"):
            disk.write_block(-1, b"\x00" * 512)

    def test_storage_property(self) -> None:
        """The storage property should expose the backing bytearray."""
        disk = SimulatedDisk(total_blocks=4)
        assert isinstance(disk.storage, bytearray)
        assert len(disk.storage) == 4 * 512

    def test_last_valid_block(self) -> None:
        """Should be able to read/write the very last block."""
        disk = SimulatedDisk(total_blocks=4)
        disk.write_block(3, b"\xFF" * 512)
        assert disk.read_block(3) == b"\xFF" * 512

    def test_overwrite_block(self) -> None:
        """Writing the same block twice should use the latest data."""
        disk = SimulatedDisk(total_blocks=4)
        disk.write_block(0, b"\xAA" * 512)
        disk.write_block(0, b"\xBB" * 512)
        assert disk.read_block(0) == b"\xBB" * 512
