# frozen_string_literal: true

require_relative "test_helper"

# End-to-end execution tests -- full pipeline from allocation to results.
module ExecutionTestHelper
  include CodingAdventures

  def make_device(vendor = "nvidia")
    instance = ComputeRuntime::RuntimeInstance.new
    physical = instance.enumerate_physical_devices.find { |d| d.vendor == vendor }
    instance.create_logical_device(physical)
  end
end

class TestGPUExecution < Minitest::Test
  include ExecutionTestHelper

  def test_simple_dispatch
    device = make_device("nvidia")
    queue = device.queues["compute"][0]

    shader = device.create_shader_module(
      code: [GpuCore.limm(0, 42.0), GpuCore.halt],
      local_size: [32, 1, 1]
    )
    ds_layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    fence = device.create_fence
    queue.submit([cb], fence: fence)
    assert fence.signaled
    assert fence.wait
  end

  def test_dispatch_with_barrier
    device = make_device("nvidia")
    queue = device.queues["compute"][0]

    shader = device.create_shader_module(
      code: [GpuCore.limm(0, 1.0), GpuCore.halt],
      local_size: [32, 1, 1]
    )
    ds_layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.cmd_pipeline_barrier(ComputeRuntime::PipelineBarrier.new(
      src_stage: :compute,
      dst_stage: :compute,
      memory_barriers: [
        ComputeRuntime::MemoryBarrier.new(
          src_access: ComputeRuntime::AccessFlags::SHADER_WRITE,
          dst_access: ComputeRuntime::AccessFlags::SHADER_READ
        )
      ]
    ))
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    fence = device.create_fence
    queue.submit([cb], fence: fence)
    assert fence.signaled
    assert_equal 2, device.stats.total_dispatches
    assert_equal 1, device.stats.total_barriers
  end

  def test_upload_and_dispatch
    device = make_device("nvidia")
    queue = device.queues["compute"][0]
    mm = device.memory_manager

    # Allocate staging and device buffers
    staging = mm.allocate(
      64,
      ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT,
      usage: ComputeRuntime::BufferUsage::TRANSFER_SRC
    )
    device_buf = mm.allocate(
      64,
      ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT,
      usage: ComputeRuntime::BufferUsage::STORAGE | ComputeRuntime::BufferUsage::TRANSFER_DST
    )

    # Write to staging
    mapped = mm.map(staging)
    mapped.write(0, "\x42".b * 64)
    mm.unmap(staging)

    # Upload + dispatch
    shader = device.create_shader_module(
      code: [GpuCore.limm(0, 1.0), GpuCore.halt],
      local_size: [32, 1, 1]
    )
    ds_layout = device.create_descriptor_set_layout([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    desc_set = device.create_descriptor_set(ds_layout)
    desc_set.write(0, device_buf)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_copy_buffer(staging, device_buf, 64)
    cb.cmd_pipeline_barrier(ComputeRuntime::PipelineBarrier.new(
      src_stage: :transfer,
      dst_stage: :compute,
      memory_barriers: [
        ComputeRuntime::MemoryBarrier.new(
          src_access: ComputeRuntime::AccessFlags::TRANSFER_WRITE,
          dst_access: ComputeRuntime::AccessFlags::SHADER_READ
        )
      ]
    ))
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_bind_descriptor_set(desc_set)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    fence = device.create_fence
    queue.submit([cb], fence: fence)
    assert fence.signaled
  end

  def test_all_gpu_devices
    %w[nvidia amd intel].each do |vendor|
      device = make_device(vendor)
      queue = device.queues["compute"][0]

      shader = device.create_shader_module(
        code: [GpuCore.limm(0, 42.0), GpuCore.halt],
        local_size: [32, 1, 1]
      )
      ds_layout = device.create_descriptor_set_layout([])
      pl_layout = device.create_pipeline_layout([ds_layout])
      pipeline = device.create_compute_pipeline(shader, pl_layout)

      cb = device.create_command_buffer
      cb.begin
      cb.cmd_bind_pipeline(pipeline)
      cb.cmd_dispatch(1, 1, 1)
      cb.end_recording

      fence = device.create_fence
      queue.submit([cb], fence: fence)
      assert fence.signaled, "#{vendor} dispatch should complete"
    end
  end
end

class TestDataflowExecution < Minitest::Test
  include ExecutionTestHelper

  def test_tpu_dispatch
    device = make_device("google")
    queue = device.queues["compute"][0]

    shader = device.create_shader_module(operation: "matmul")
    ds_layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    fence = device.create_fence
    queue.submit([cb], fence: fence)
    assert fence.signaled
  end

  def test_ane_dispatch
    device = make_device("apple")
    queue = device.queues["compute"][0]

    shader = device.create_shader_module(operation: "matmul")
    ds_layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    fence = device.create_fence
    queue.submit([cb], fence: fence)
    assert fence.signaled
  end
end

class TestUnifiedMemory < Minitest::Test
  include ExecutionTestHelper

  def test_zero_copy_pattern
    device = make_device("apple")
    queue = device.queues["compute"][0]
    mm = device.memory_manager

    # On unified memory, DEVICE_LOCAL + HOST_VISIBLE works
    buf = mm.allocate(
      64,
      ComputeRuntime::MemoryType::DEVICE_LOCAL |
        ComputeRuntime::MemoryType::HOST_VISIBLE |
        ComputeRuntime::MemoryType::HOST_COHERENT,
      usage: ComputeRuntime::BufferUsage::STORAGE
    )

    # Write directly -- no staging buffer!
    mapped = mm.map(buf)
    mapped.write(0, "\x42".b * 64)
    mm.unmap(buf)

    # Dispatch
    shader = device.create_shader_module(operation: "matmul")
    ds_layout = device.create_descriptor_set_layout([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)
    desc_set = device.create_descriptor_set(ds_layout)
    desc_set.write(0, buf)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_bind_descriptor_set(desc_set)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    fence = device.create_fence
    queue.submit([cb], fence: fence)
    assert fence.signaled
  end
end

class TestCommandBufferReuse < Minitest::Test
  include ExecutionTestHelper

  def test_reuse_after_completion
    device = make_device
    queue = device.queues["compute"][0]

    shader = device.create_shader_module(
      code: [GpuCore.limm(0, 1.0), GpuCore.halt],
      local_size: [32, 1, 1]
    )
    ds_layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = device.create_command_buffer

    # First use
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording
    fence1 = device.create_fence
    queue.submit([cb], fence: fence1)
    assert fence1.signaled

    # Reset and reuse
    cb.reset
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(2, 1, 1)
    cb.end_recording
    fence2 = device.create_fence
    queue.submit([cb], fence: fence2)
    assert fence2.signaled

    assert_equal 2, device.stats.total_dispatches
  end
end

class TestMultiSubmit < Minitest::Test
  include ExecutionTestHelper

  def test_sequential_command_buffers
    device = make_device
    queue = device.queues["compute"][0]

    shader = device.create_shader_module(
      code: [GpuCore.limm(0, 1.0), GpuCore.halt],
      local_size: [32, 1, 1]
    )
    ds_layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    # Create 3 CBs
    cbs = 3.times.map do
      cb = device.create_command_buffer
      cb.begin
      cb.cmd_bind_pipeline(pipeline)
      cb.cmd_dispatch(1, 1, 1)
      cb.end_recording
      cb
    end

    fence = device.create_fence
    queue.submit(cbs, fence: fence)
    assert fence.signaled
    assert_equal 3, device.stats.total_dispatches
    assert_equal 3, device.stats.total_command_buffers
  end
end

class TestRuntimeStatsE2E < Minitest::Test
  include ExecutionTestHelper

  def test_stats_accumulate
    device = make_device
    queue = device.queues["compute"][0]

    shader = device.create_shader_module(
      code: [GpuCore.limm(0, 1.0), GpuCore.halt],
      local_size: [32, 1, 1]
    )
    ds_layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    5.times do
      cb = device.create_command_buffer
      cb.begin
      cb.cmd_bind_pipeline(pipeline)
      cb.cmd_dispatch(1, 1, 1)
      cb.end_recording
      queue.submit([cb])
    end

    stats = device.stats
    assert_equal 5, stats.total_submissions
    assert_equal 5, stats.total_dispatches
    assert stats.total_device_cycles > 0
  end

  def test_traces_collected
    device = make_device
    queue = device.queues["compute"][0]

    shader = device.create_shader_module(
      code: [GpuCore.limm(0, 1.0), GpuCore.halt],
      local_size: [32, 1, 1]
    )
    ds_layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    queue.submit([cb])

    assert device.stats.traces.length > 0
  end

  def test_utilization_calculated
    device = make_device
    queue = device.queues["compute"][0]

    shader = device.create_shader_module(
      code: [GpuCore.limm(0, 1.0), GpuCore.halt],
      local_size: [32, 1, 1]
    )
    ds_layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    queue.submit([cb])
    # After at least one dispatch, utilization should be calculated
    assert device.stats.total_device_cycles > 0
  end
end
