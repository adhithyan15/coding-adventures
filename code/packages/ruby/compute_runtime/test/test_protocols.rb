# frozen_string_literal: true

require_relative "test_helper"

# Tests for protocol types -- enums, flags, data types.
class TestMemoryTypeFlags < Minitest::Test
  include CodingAdventures::ComputeRuntime

  def test_individual_flags
    assert_equal 1, MemoryType::DEVICE_LOCAL
    assert_equal 2, MemoryType::HOST_VISIBLE
    assert_equal 4, MemoryType::HOST_COHERENT
    assert_equal 8, MemoryType::HOST_CACHED
  end

  def test_combine_flags
    combined = MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT
    assert_equal 6, combined
    assert((combined & MemoryType::HOST_VISIBLE) != 0)
    assert((combined & MemoryType::HOST_COHERENT) != 0)
    assert((combined & MemoryType::DEVICE_LOCAL) == 0)
  end

  def test_unified_memory_flags
    unified = MemoryType::DEVICE_LOCAL | MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT
    assert((unified & MemoryType::DEVICE_LOCAL) != 0)
    assert((unified & MemoryType::HOST_VISIBLE) != 0)
    assert((unified & MemoryType::HOST_COHERENT) != 0)
  end
end

class TestBufferUsageFlags < Minitest::Test
  include CodingAdventures::ComputeRuntime

  def test_individual_flags
    assert_equal 1, BufferUsage::STORAGE
    assert_equal 2, BufferUsage::UNIFORM
    assert_equal 4, BufferUsage::TRANSFER_SRC
    assert_equal 8, BufferUsage::TRANSFER_DST
    assert_equal 16, BufferUsage::INDIRECT
  end

  def test_combine_usage
    combined = BufferUsage::STORAGE | BufferUsage::TRANSFER_DST
    assert((combined & BufferUsage::STORAGE) != 0)
    assert((combined & BufferUsage::TRANSFER_DST) != 0)
    assert((combined & BufferUsage::TRANSFER_SRC) == 0)
  end
end

class TestAccessFlags < Minitest::Test
  include CodingAdventures::ComputeRuntime

  def test_none_is_zero
    assert_equal 0, AccessFlags::NONE
  end

  def test_individual_flags
    assert_equal 1, AccessFlags::SHADER_READ
    assert_equal 2, AccessFlags::SHADER_WRITE
    assert_equal 4, AccessFlags::TRANSFER_READ
    assert_equal 8, AccessFlags::TRANSFER_WRITE
  end
end

class TestQueueFamily < Minitest::Test
  include CodingAdventures::ComputeRuntime

  def test_creation
    qf = QueueFamily.new(queue_type: :compute, count: 4)
    assert_equal :compute, qf.queue_type
    assert_equal 4, qf.count
  end

  def test_frozen
    qf = QueueFamily.new(queue_type: :transfer, count: 2)
    assert qf.frozen?
  end
end

class TestDeviceLimits < Minitest::Test
  include CodingAdventures::ComputeRuntime

  def test_defaults
    limits = DeviceLimits.new
    assert_equal [1024, 1024, 64], limits.max_workgroup_size
    assert_equal 2 * 1024 * 1024 * 1024, limits.max_buffer_size
    assert_equal 128, limits.max_push_constant_size
  end

  def test_custom
    limits = DeviceLimits.new(max_buffer_size: 1024)
    assert_equal 1024, limits.max_buffer_size
  end
end

class TestMemoryHeap < Minitest::Test
  include CodingAdventures::ComputeRuntime

  def test_creation
    heap = MemoryHeap.new(size: 4096, flags: MemoryType::DEVICE_LOCAL)
    assert_equal 4096, heap.size
    assert_equal MemoryType::DEVICE_LOCAL, heap.flags
  end
end

class TestMemoryProperties < Minitest::Test
  include CodingAdventures::ComputeRuntime

  def test_default_not_unified
    props = MemoryProperties.new(heaps: [])
    refute props.is_unified
  end

  def test_unified
    props = MemoryProperties.new(heaps: [], is_unified: true)
    assert props.is_unified
  end
end

class TestDescriptorBinding < Minitest::Test
  include CodingAdventures::ComputeRuntime

  def test_defaults
    db = DescriptorBinding.new(binding: 0)
    assert_equal 0, db.binding
    assert_equal "storage", db.type
    assert_equal 1, db.count
  end

  def test_uniform
    db = DescriptorBinding.new(binding: 1, type: "uniform", count: 2)
    assert_equal "uniform", db.type
    assert_equal 2, db.count
  end
end

class TestRecordedCommand < Minitest::Test
  include CodingAdventures::ComputeRuntime

  def test_creation
    cmd = RecordedCommand.new(command: "dispatch", args: {group_x: 4})
    assert_equal "dispatch", cmd.command
    assert_equal({group_x: 4}, cmd.args)
  end

  def test_default_args
    cmd = RecordedCommand.new(command: "bind_pipeline")
    assert_equal({}, cmd.args)
  end
end

class TestPipelineBarrier < Minitest::Test
  include CodingAdventures::ComputeRuntime

  def test_defaults
    barrier = PipelineBarrier.new
    assert_equal :top_of_pipe, barrier.src_stage
    assert_equal :bottom_of_pipe, barrier.dst_stage
    assert_equal [], barrier.memory_barriers
    assert_equal [], barrier.buffer_barriers
  end

  def test_with_memory_barrier
    mb = MemoryBarrier.new(
      src_access: AccessFlags::SHADER_WRITE,
      dst_access: AccessFlags::SHADER_READ
    )
    barrier = PipelineBarrier.new(
      src_stage: :compute,
      dst_stage: :compute,
      memory_barriers: [mb]
    )
    assert_equal :compute, barrier.src_stage
    assert_equal 1, barrier.memory_barriers.length
  end
end

class TestRuntimeTrace < Minitest::Test
  include CodingAdventures::ComputeRuntime

  def test_defaults
    trace = RuntimeTrace.new
    assert_equal 0, trace.timestamp_cycles
    assert_equal :submit, trace.event_type
    assert_equal "", trace.description
  end

  def test_format
    trace = RuntimeTrace.new(
      timestamp_cycles: 150,
      event_type: :submit,
      description: "CB#1 to compute queue"
    )
    formatted = trace.format
    assert_includes formatted, "150"
    assert_includes formatted, "SUBMIT"
    assert_includes formatted, "CB#1"
  end
end

class TestRuntimeStats < Minitest::Test
  include CodingAdventures::ComputeRuntime

  def test_initial_values
    stats = RuntimeStats.new
    assert_equal 0, stats.total_submissions
    assert_equal 0, stats.total_dispatches
    assert_equal 0.0, stats.gpu_utilization
    assert_equal [], stats.traces
  end

  def test_update_utilization
    stats = RuntimeStats.new
    stats.total_device_cycles = 80
    stats.total_idle_cycles = 20
    stats.update_utilization
    assert_in_delta 0.8, stats.gpu_utilization, 0.001
  end

  def test_update_utilization_zero
    stats = RuntimeStats.new
    stats.update_utilization
    assert_equal 0.0, stats.gpu_utilization
  end
end
