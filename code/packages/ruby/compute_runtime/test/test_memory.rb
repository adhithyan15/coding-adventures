# frozen_string_literal: true

require_relative "test_helper"

# Helper to create a logical device for testing.
module MemoryTestHelper
  include CodingAdventures

  def make_device(vendor = "nvidia")
    instance = ComputeRuntime::RuntimeInstance.new
    physical = instance.enumerate_physical_devices.find { |d| d.vendor == vendor }
    instance.create_logical_device(physical)
  end
end

class TestAllocate < Minitest::Test
  include MemoryTestHelper

  def test_basic_allocation
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(256, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    assert_equal 256, buf.size
    assert buf.device_address >= 0
    refute buf.freed
    refute buf.mapped
  end

  def test_allocation_with_usage
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      128,
      ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::STORAGE | ComputeRuntime::BufferUsage::TRANSFER_DST
    )
    assert (buf.usage & ComputeRuntime::BufferUsage::STORAGE) != 0
    assert (buf.usage & ComputeRuntime::BufferUsage::TRANSFER_DST) != 0
  end

  def test_host_visible_allocation
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64,
      ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    assert (buf.memory_type & ComputeRuntime::MemoryType::HOST_VISIBLE) != 0
  end

  def test_unique_buffer_ids
    device = make_device
    mm = device.memory_manager
    b1 = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    b2 = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    refute_equal b1.buffer_id, b2.buffer_id
  end

  def test_stats_tracked
    device = make_device
    mm = device.memory_manager
    mm.allocate(256, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    mm.allocate(128, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    assert_equal 2, device.stats.total_allocations
    assert_equal 384, device.stats.total_allocated_bytes
    assert_equal 384, device.stats.peak_allocated_bytes
  end

  def test_invalid_size
    device = make_device
    mm = device.memory_manager
    assert_raises(ArgumentError) do
      mm.allocate(0, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    end
  end

  def test_negative_size
    device = make_device
    mm = device.memory_manager
    assert_raises(ArgumentError) do
      mm.allocate(-100, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    end
  end
end

class TestFreeMemory < Minitest::Test
  include MemoryTestHelper

  def test_basic_free
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(256, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    mm.free(buf)
    assert buf.freed
    assert_equal 1, device.stats.total_frees
  end

  def test_double_free
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(256, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    mm.free(buf)
    assert_raises(ArgumentError) { mm.free(buf) }
  end

  def test_free_while_mapped
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mm.map(buf)
    assert_raises(ArgumentError) { mm.free(buf) }
  end

  def test_current_bytes_after_free
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(256, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    assert_equal 256, mm.current_allocated_bytes
    mm.free(buf)
    assert_equal 0, mm.current_allocated_bytes
  end

  def test_peak_bytes_after_free
    device = make_device
    mm = device.memory_manager
    b1 = mm.allocate(256, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    b2 = mm.allocate(128, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    mm.free(b1)
    assert_equal 384, device.stats.peak_allocated_bytes
    assert_equal 128, mm.current_allocated_bytes
  end
end

class TestMap < Minitest::Test
  include MemoryTestHelper

  def test_map_host_visible
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mapped = mm.map(buf)
    assert_kind_of ComputeRuntime::MappedMemory, mapped
    assert buf.mapped
    assert_equal 1, device.stats.total_maps
  end

  def test_map_device_local_fails
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    assert_raises(ArgumentError) { mm.map(buf) }
  end

  def test_map_freed_fails
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mm.free(buf)
    assert_raises(ArgumentError) { mm.map(buf) }
  end

  def test_double_map_fails
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mm.map(buf)
    assert_raises(ArgumentError) { mm.map(buf) }
  end

  def test_unmap
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mm.map(buf)
    mm.unmap(buf)
    refute buf.mapped
  end

  def test_unmap_not_mapped_fails
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    assert_raises(ArgumentError) { mm.unmap(buf) }
  end

  def test_unified_memory_map
    device = make_device("apple")
    mm = device.memory_manager
    buf = mm.allocate(
      64,
      ComputeRuntime::MemoryType::DEVICE_LOCAL |
        ComputeRuntime::MemoryType::HOST_VISIBLE |
        ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mapped = mm.map(buf)
    refute_nil mapped
    mm.unmap(buf)
  end
end

class TestMappedMemory < Minitest::Test
  include MemoryTestHelper

  def test_read_write
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mapped = mm.map(buf)
    mapped.write(0, "\x42".b * 16)
    data = mapped.read(0, 16)
    assert_equal "\x42".b * 16, data
  end

  def test_write_at_offset
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mapped = mm.map(buf)
    mapped.write(32, "\xAA".b * 8)
    data = mapped.read(32, 8)
    assert_equal "\xAA".b * 8, data
  end

  def test_read_out_of_bounds
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      16, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mapped = mm.map(buf)
    assert_raises(ArgumentError) { mapped.read(0, 32) }
  end

  def test_write_out_of_bounds
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      16, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mapped = mm.map(buf)
    assert_raises(ArgumentError) { mapped.write(0, "\x00".b * 32) }
  end

  def test_dirty_flag
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mapped = mm.map(buf)
    refute mapped.dirty
    mapped.write(0, "\x01".b)
    assert mapped.dirty
  end

  def test_get_data
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      8, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mapped = mm.map(buf)
    mapped.write(0, "\x01\x02\x03\x04".b)
    data = mapped.get_data
    assert_equal 8, data.bytesize
    assert_equal "\x01\x02\x03\x04".b, data.byteslice(0, 4)
  end

  def test_size_property
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      128, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mapped = mm.map(buf)
    assert_equal 128, mapped.size
  end
end

class TestFlushInvalidate < Minitest::Test
  include MemoryTestHelper

  def test_flush
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mapped = mm.map(buf)
    mapped.write(0, "\xFF".b * 64)
    mm.flush(buf) # Should not raise
    mm.unmap(buf)
  end

  def test_invalidate
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mm.invalidate(buf) # Should not raise
  end

  def test_flush_freed_fails
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mm.free(buf)
    assert_raises(ArgumentError) { mm.flush(buf) }
  end

  def test_invalidate_freed_fails
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64, ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT
    )
    mm.free(buf)
    assert_raises(ArgumentError) { mm.invalidate(buf) }
  end
end

class TestBufferCount < Minitest::Test
  include MemoryTestHelper

  def test_allocated_buffer_count
    device = make_device
    mm = device.memory_manager
    assert_equal 0, mm.allocated_buffer_count
    b1 = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    assert_equal 1, mm.allocated_buffer_count
    b2 = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    assert_equal 2, mm.allocated_buffer_count
    mm.free(b1)
    assert_equal 1, mm.allocated_buffer_count
  end

  def test_get_buffer
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL)
    retrieved = mm.get_buffer(buf.buffer_id)
    assert_same buf, retrieved
  end

  def test_get_buffer_not_found
    device = make_device
    mm = device.memory_manager
    assert_raises(ArgumentError) { mm.get_buffer(9999) }
  end
end
