"""Tests for MemoryManager, Buffer, MappedMemory."""

import pytest

from compute_runtime import (
    RuntimeInstance,
    MemoryType,
    BufferUsage,
    Buffer,
    MappedMemory,
)


def make_device(vendor: str = "nvidia"):
    instance = RuntimeInstance()
    physical = next(
        d for d in instance.enumerate_physical_devices()
        if d.vendor == vendor
    )
    return instance.create_logical_device(physical)


class TestAllocate:
    def test_basic_allocation(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(256, MemoryType.DEVICE_LOCAL)
        assert buf.size == 256
        assert buf.device_address >= 0
        assert not buf.freed
        assert not buf.mapped

    def test_allocation_with_usage(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            128,
            MemoryType.DEVICE_LOCAL,
            usage=BufferUsage.STORAGE | BufferUsage.TRANSFER_DST,
        )
        assert BufferUsage.STORAGE in buf.usage
        assert BufferUsage.TRANSFER_DST in buf.usage

    def test_host_visible_allocation(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64,
            MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
        )
        assert MemoryType.HOST_VISIBLE in buf.memory_type

    def test_unique_buffer_ids(self) -> None:
        device = make_device()
        mm = device.memory_manager
        b1 = mm.allocate(64, MemoryType.DEVICE_LOCAL)
        b2 = mm.allocate(64, MemoryType.DEVICE_LOCAL)
        assert b1.buffer_id != b2.buffer_id

    def test_stats_tracked(self) -> None:
        device = make_device()
        mm = device.memory_manager
        mm.allocate(256, MemoryType.DEVICE_LOCAL)
        mm.allocate(128, MemoryType.DEVICE_LOCAL)
        assert device.stats.total_allocations == 2
        assert device.stats.total_allocated_bytes == 384
        assert device.stats.peak_allocated_bytes == 384

    def test_invalid_size(self) -> None:
        device = make_device()
        mm = device.memory_manager
        with pytest.raises(ValueError, match="positive"):
            mm.allocate(0, MemoryType.DEVICE_LOCAL)

    def test_negative_size(self) -> None:
        device = make_device()
        mm = device.memory_manager
        with pytest.raises(ValueError, match="positive"):
            mm.allocate(-100, MemoryType.DEVICE_LOCAL)


class TestFree:
    def test_basic_free(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(256, MemoryType.DEVICE_LOCAL)
        mm.free(buf)
        assert buf.freed
        assert device.stats.total_frees == 1

    def test_double_free(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(256, MemoryType.DEVICE_LOCAL)
        mm.free(buf)
        with pytest.raises(ValueError, match="already freed"):
            mm.free(buf)

    def test_free_while_mapped(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mm.map(buf)
        with pytest.raises(ValueError, match="still mapped"):
            mm.free(buf)

    def test_current_bytes_after_free(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(256, MemoryType.DEVICE_LOCAL)
        assert mm.current_allocated_bytes == 256
        mm.free(buf)
        assert mm.current_allocated_bytes == 0

    def test_peak_bytes_after_free(self) -> None:
        device = make_device()
        mm = device.memory_manager
        b1 = mm.allocate(256, MemoryType.DEVICE_LOCAL)
        b2 = mm.allocate(128, MemoryType.DEVICE_LOCAL)
        mm.free(b1)
        assert device.stats.peak_allocated_bytes == 384
        assert mm.current_allocated_bytes == 128


class TestMap:
    def test_map_host_visible(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mapped = mm.map(buf)
        assert isinstance(mapped, MappedMemory)
        assert buf.mapped
        assert device.stats.total_maps == 1

    def test_map_device_local_fails(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(64, MemoryType.DEVICE_LOCAL)
        with pytest.raises(ValueError, match="HOST_VISIBLE"):
            mm.map(buf)

    def test_map_freed_fails(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mm.free(buf)
        with pytest.raises(ValueError, match="freed"):
            mm.map(buf)

    def test_double_map_fails(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mm.map(buf)
        with pytest.raises(ValueError, match="already mapped"):
            mm.map(buf)

    def test_unmap(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mm.map(buf)
        mm.unmap(buf)
        assert not buf.mapped

    def test_unmap_not_mapped_fails(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        with pytest.raises(ValueError, match="not mapped"):
            mm.unmap(buf)

    def test_unified_memory_map(self) -> None:
        """Apple unified: DEVICE_LOCAL + HOST_VISIBLE works."""
        device = make_device("apple")
        mm = device.memory_manager
        buf = mm.allocate(
            64,
            MemoryType.DEVICE_LOCAL | MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
        )
        mapped = mm.map(buf)
        assert mapped is not None
        mm.unmap(buf)


class TestMappedMemory:
    def test_read_write(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mapped = mm.map(buf)
        mapped.write(0, b"\x42" * 16)
        data = mapped.read(0, 16)
        assert data == b"\x42" * 16

    def test_write_at_offset(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mapped = mm.map(buf)
        mapped.write(32, b"\xAA" * 8)
        data = mapped.read(32, 8)
        assert data == b"\xAA" * 8

    def test_read_out_of_bounds(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            16, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mapped = mm.map(buf)
        with pytest.raises(ValueError, match="out of bounds"):
            mapped.read(0, 32)

    def test_write_out_of_bounds(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            16, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mapped = mm.map(buf)
        with pytest.raises(ValueError, match="out of bounds"):
            mapped.write(0, b"\x00" * 32)

    def test_dirty_flag(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mapped = mm.map(buf)
        assert not mapped.dirty
        mapped.write(0, b"\x01")
        assert mapped.dirty

    def test_get_data(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            8, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mapped = mm.map(buf)
        mapped.write(0, b"\x01\x02\x03\x04")
        data = mapped.get_data()
        assert len(data) == 8
        assert data[:4] == b"\x01\x02\x03\x04"

    def test_size_property(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            128, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mapped = mm.map(buf)
        assert mapped.size == 128


class TestFlushInvalidate:
    def test_flush(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mapped = mm.map(buf)
        mapped.write(0, b"\xFF" * 64)
        mm.flush(buf)  # Should not raise
        mm.unmap(buf)

    def test_invalidate(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mm.invalidate(buf)  # Should not raise

    def test_flush_freed_fails(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mm.free(buf)
        with pytest.raises(ValueError, match="freed"):
            mm.flush(buf)

    def test_invalidate_freed_fails(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(
            64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT
        )
        mm.free(buf)
        with pytest.raises(ValueError, match="freed"):
            mm.invalidate(buf)


class TestBufferCount:
    def test_allocated_buffer_count(self) -> None:
        device = make_device()
        mm = device.memory_manager
        assert mm.allocated_buffer_count == 0
        b1 = mm.allocate(64, MemoryType.DEVICE_LOCAL)
        assert mm.allocated_buffer_count == 1
        b2 = mm.allocate(64, MemoryType.DEVICE_LOCAL)
        assert mm.allocated_buffer_count == 2
        mm.free(b1)
        assert mm.allocated_buffer_count == 1

    def test_get_buffer(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(64, MemoryType.DEVICE_LOCAL)
        retrieved = mm.get_buffer(buf.buffer_id)
        assert retrieved is buf

    def test_get_buffer_not_found(self) -> None:
        device = make_device()
        mm = device.memory_manager
        with pytest.raises(ValueError, match="not found"):
            mm.get_buffer(9999)
