# frozen_string_literal: true

require_relative "test_helper"

# Tests for ValidationLayer -- error detection.
module ValidationTestHelper
  include CodingAdventures

  def make_device
    instance = ComputeRuntime::RuntimeInstance.new
    physical = instance.enumerate_physical_devices[0]
    instance.create_logical_device(physical)
  end
end

class TestCommandBufferValidation < Minitest::Test
  include ValidationTestHelper

  def test_validate_begin_initial
    vl = ComputeRuntime::ValidationLayer.new
    cb = ComputeRuntime::CommandBuffer.new
    vl.validate_begin(cb) # Should not raise
  end

  def test_validate_begin_recording_fails
    vl = ComputeRuntime::ValidationLayer.new
    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    assert_raises(ComputeRuntime::ValidationError) { vl.validate_begin(cb) }
  end

  def test_validate_end_recording
    vl = ComputeRuntime::ValidationLayer.new
    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    vl.validate_end(cb) # Should not raise
  end

  def test_validate_end_initial_fails
    vl = ComputeRuntime::ValidationLayer.new
    cb = ComputeRuntime::CommandBuffer.new
    assert_raises(ComputeRuntime::ValidationError) { vl.validate_end(cb) }
  end

  def test_validate_submit_recorded
    vl = ComputeRuntime::ValidationLayer.new
    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.end_recording
    vl.validate_submit(cb) # Should not raise
  end

  def test_validate_submit_initial_fails
    vl = ComputeRuntime::ValidationLayer.new
    cb = ComputeRuntime::CommandBuffer.new
    assert_raises(ComputeRuntime::ValidationError) { vl.validate_submit(cb) }
  end
end

class TestDispatchValidation < Minitest::Test
  include ValidationTestHelper

  def test_dispatch_without_pipeline
    vl = ComputeRuntime::ValidationLayer.new
    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    assert_raises(ComputeRuntime::ValidationError) { vl.validate_dispatch(cb, 1, 1, 1) }
  end

  def test_dispatch_negative_dims
    vl = ComputeRuntime::ValidationLayer.new
    device = make_device
    shader = device.create_shader_module(code: [GpuCore.halt])
    layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_bind_pipeline(pipeline)

    assert_raises(ComputeRuntime::ValidationError) { vl.validate_dispatch(cb, -1, 1, 1) }
  end

  def test_dispatch_zero_dims
    vl = ComputeRuntime::ValidationLayer.new
    device = make_device
    shader = device.create_shader_module(code: [GpuCore.halt])
    layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_bind_pipeline(pipeline)

    assert_raises(ComputeRuntime::ValidationError) { vl.validate_dispatch(cb, 0, 1, 1) }
  end

  def test_dispatch_valid
    vl = ComputeRuntime::ValidationLayer.new
    device = make_device
    shader = device.create_shader_module(code: [GpuCore.halt])
    layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    vl.validate_dispatch(cb, 4, 2, 1) # Should not raise
  end
end

class TestMemoryValidation < Minitest::Test
  include ValidationTestHelper

  def test_map_host_visible
    vl = ComputeRuntime::ValidationLayer.new
    buf = ComputeRuntime::Buffer.new(
      buffer_id: 0, size: 64,
      memory_type: ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT,
      usage: ComputeRuntime::BufferUsage::STORAGE
    )
    vl.validate_map(buf) # Should not raise
  end

  def test_map_device_local_fails
    vl = ComputeRuntime::ValidationLayer.new
    buf = ComputeRuntime::Buffer.new(
      buffer_id: 0, size: 64,
      memory_type: ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::STORAGE
    )
    assert_raises(ComputeRuntime::ValidationError) { vl.validate_map(buf) }
  end

  def test_map_freed_fails
    vl = ComputeRuntime::ValidationLayer.new
    buf = ComputeRuntime::Buffer.new(
      buffer_id: 0, size: 64,
      memory_type: ComputeRuntime::MemoryType::HOST_VISIBLE,
      usage: ComputeRuntime::BufferUsage::STORAGE
    )
    buf.freed = true
    assert_raises(ComputeRuntime::ValidationError) { vl.validate_map(buf) }
  end

  def test_map_already_mapped_fails
    vl = ComputeRuntime::ValidationLayer.new
    buf = ComputeRuntime::Buffer.new(
      buffer_id: 0, size: 64,
      memory_type: ComputeRuntime::MemoryType::HOST_VISIBLE,
      usage: ComputeRuntime::BufferUsage::STORAGE
    )
    buf.mapped = true
    assert_raises(ComputeRuntime::ValidationError) { vl.validate_map(buf) }
  end

  def test_buffer_usage_validation
    vl = ComputeRuntime::ValidationLayer.new
    buf = ComputeRuntime::Buffer.new(
      buffer_id: 0, size: 64,
      memory_type: ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::STORAGE
    )
    vl.validate_buffer_usage(buf, ComputeRuntime::BufferUsage::STORAGE) # OK
  end

  def test_buffer_usage_missing
    vl = ComputeRuntime::ValidationLayer.new
    buf = ComputeRuntime::Buffer.new(
      buffer_id: 0, size: 64,
      memory_type: ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::STORAGE
    )
    assert_raises(ComputeRuntime::ValidationError) do
      vl.validate_buffer_usage(buf, ComputeRuntime::BufferUsage::TRANSFER_SRC)
    end
  end

  def test_buffer_not_freed
    vl = ComputeRuntime::ValidationLayer.new
    buf = ComputeRuntime::Buffer.new(
      buffer_id: 0, size: 64,
      memory_type: ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::STORAGE
    )
    buf.freed = true
    assert_raises(ComputeRuntime::ValidationError) { vl.validate_buffer_not_freed(buf) }
  end
end

class TestBarrierValidation < Minitest::Test
  include ValidationTestHelper

  def test_write_without_barrier_warns
    vl = ComputeRuntime::ValidationLayer.new
    vl.record_write(42)
    vl.validate_read_after_write(42)
    assert_equal 1, vl.warnings.length
    assert_includes vl.warnings[0].downcase, "barrier"
  end

  def test_write_with_barrier_ok
    vl = ComputeRuntime::ValidationLayer.new
    vl.record_write(42)
    vl.record_barrier # Global barrier
    vl.validate_read_after_write(42)
    assert_equal 0, vl.warnings.length
  end

  def test_unwritten_buffer_no_warning
    vl = ComputeRuntime::ValidationLayer.new
    vl.validate_read_after_write(99)
    assert_equal 0, vl.warnings.length
  end

  def test_barrier_specific_buffer
    vl = ComputeRuntime::ValidationLayer.new
    vl.record_write(10)
    vl.record_write(20)
    vl.record_barrier(buffer_ids: Set[10])

    vl.validate_read_after_write(10) # OK, barriered
    assert_equal 0, vl.warnings.length

    vl.validate_read_after_write(20) # Not barriered
    assert_equal 1, vl.warnings.length
  end

  def test_clear
    vl = ComputeRuntime::ValidationLayer.new
    vl.record_write(1)
    vl.validate_read_after_write(1)
    assert_equal 1, vl.warnings.length
    vl.clear
    assert_equal 0, vl.warnings.length
    assert_equal 0, vl.errors.length
  end
end

class TestDescriptorSetValidation < Minitest::Test
  include ValidationTestHelper

  def test_valid_descriptor_set
    vl = ComputeRuntime::ValidationLayer.new
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::STORAGE)

    ds_layout = device.create_descriptor_set_layout([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    desc_set = device.create_descriptor_set(ds_layout)
    desc_set.write(0, buf)

    shader = device.create_shader_module(code: [GpuCore.halt])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    vl.validate_descriptor_set(desc_set, pipeline)
    assert_equal 0, vl.warnings.length
  end

  def test_missing_binding_warns
    vl = ComputeRuntime::ValidationLayer.new
    device = make_device

    ds_layout = device.create_descriptor_set_layout([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    desc_set = device.create_descriptor_set(ds_layout)
    # Don't write binding 0

    shader = device.create_shader_module(code: [GpuCore.halt])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    vl.validate_descriptor_set(desc_set, pipeline)
    assert_equal 1, vl.warnings.length
    assert_includes vl.warnings[0], "not set"
  end

  def test_freed_buffer_in_descriptor
    vl = ComputeRuntime::ValidationLayer.new
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::STORAGE)

    ds_layout = device.create_descriptor_set_layout([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    desc_set = device.create_descriptor_set(ds_layout)
    desc_set.write(0, buf)

    # Free the buffer after binding
    mm.free(buf)

    shader = device.create_shader_module(code: [GpuCore.halt])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    assert_raises(ComputeRuntime::ValidationError) do
      vl.validate_descriptor_set(desc_set, pipeline)
    end
  end
end
