# frozen_string_literal: true

require_relative "test_helper"

# Tests for CommandQueue -- submission and execution.
module QueueTestHelper
  include CodingAdventures

  def make_device(vendor = "nvidia")
    instance = ComputeRuntime::RuntimeInstance.new
    physical = instance.enumerate_physical_devices.find { |d| d.vendor == vendor }
    instance.create_logical_device(physical)
  end

  def make_pipeline(device)
    shader = device.create_shader_module(code: [GpuCore.limm(0, 42.0), GpuCore.halt])
    ds_layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([ds_layout])
    device.create_compute_pipeline(shader, pl_layout)
  end
end

class TestSubmit < Minitest::Test
  include QueueTestHelper

  def test_basic_submit
    device = make_device
    queue = device.queues["compute"][0]
    pipeline = make_pipeline(device)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    fence = device.create_fence
    traces = queue.submit([cb], fence: fence)

    assert fence.signaled
    assert_equal :complete, cb.state
    assert traces.length > 0
  end

  def test_submit_not_recorded_fails
    device = make_device
    queue = device.queues["compute"][0]

    cb = device.create_command_buffer
    cb.begin # Still recording, not recorded

    assert_raises(RuntimeError) { queue.submit([cb]) }
  end

  def test_fence_signaled_after_submit
    device = make_device
    queue = device.queues["compute"][0]
    pipeline = make_pipeline(device)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    fence = device.create_fence
    queue.submit([cb], fence: fence)
    assert fence.wait
  end

  def test_submit_without_fence
    device = make_device
    queue = device.queues["compute"][0]
    pipeline = make_pipeline(device)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    traces = queue.submit([cb]) # No fence
    assert traces.length > 0
  end

  def test_multiple_command_buffers
    device = make_device
    queue = device.queues["compute"][0]
    pipeline = make_pipeline(device)

    cb1 = device.create_command_buffer
    cb1.begin
    cb1.cmd_bind_pipeline(pipeline)
    cb1.cmd_dispatch(1, 1, 1)
    cb1.end_recording

    cb2 = device.create_command_buffer
    cb2.begin
    cb2.cmd_bind_pipeline(pipeline)
    cb2.cmd_dispatch(2, 1, 1)
    cb2.end_recording

    fence = device.create_fence
    queue.submit([cb1, cb2], fence: fence)

    assert fence.signaled
    assert_equal :complete, cb1.state
    assert_equal :complete, cb2.state
  end

  def test_stats_updated
    device = make_device
    queue = device.queues["compute"][0]
    pipeline = make_pipeline(device)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    queue.submit([cb])

    assert_equal 1, device.stats.total_submissions
    assert_equal 1, device.stats.total_command_buffers
    assert_equal 1, device.stats.total_dispatches
  end
end

class TestSemaphoresSubmit < Minitest::Test
  include QueueTestHelper

  def test_signal_semaphore
    device = make_device
    queue = device.queues["compute"][0]
    pipeline = make_pipeline(device)
    sem = device.create_semaphore

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    queue.submit([cb], signal_semaphores: [sem])
    assert sem.signaled
  end

  def test_wait_semaphore
    device = make_device
    queue = device.queues["compute"][0]
    pipeline = make_pipeline(device)
    sem = device.create_semaphore

    # First submission signals semaphore
    cb1 = device.create_command_buffer
    cb1.begin
    cb1.cmd_bind_pipeline(pipeline)
    cb1.cmd_dispatch(1, 1, 1)
    cb1.end_recording
    queue.submit([cb1], signal_semaphores: [sem])

    # Second submission waits on semaphore
    cb2 = device.create_command_buffer
    cb2.begin
    cb2.cmd_bind_pipeline(pipeline)
    cb2.cmd_dispatch(1, 1, 1)
    cb2.end_recording
    queue.submit([cb2], wait_semaphores: [sem])

    # Semaphore consumed (reset after wait)
    refute sem.signaled
  end

  def test_wait_unsignaled_fails
    device = make_device
    queue = device.queues["compute"][0]
    pipeline = make_pipeline(device)
    sem = device.create_semaphore

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    assert_raises(RuntimeError) { queue.submit([cb], wait_semaphores: [sem]) }
  end
end

class TestTransferCommandsQueue < Minitest::Test
  include QueueTestHelper

  def test_copy_buffer
    device = make_device
    queue = device.queues["compute"][0]
    mm = device.memory_manager

    src = mm.allocate(64,
      ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT,
      usage: ComputeRuntime::BufferUsage::TRANSFER_SRC)
    dst = mm.allocate(64,
      ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT,
      usage: ComputeRuntime::BufferUsage::TRANSFER_DST)

    # Write data to src
    mapped = mm.map(src)
    mapped.write(0, "\x42".b * 64)
    mm.unmap(src)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_copy_buffer(src, dst, 64)
    cb.end_recording

    queue.submit([cb])
    assert_equal 1, device.stats.total_transfers
  end

  def test_fill_buffer
    device = make_device
    queue = device.queues["compute"][0]
    mm = device.memory_manager

    buf = mm.allocate(64,
      ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT,
      usage: ComputeRuntime::BufferUsage::TRANSFER_DST)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_fill_buffer(buf, 0xFF)
    cb.end_recording

    queue.submit([cb])
    assert_equal 1, device.stats.total_transfers
  end

  def test_update_buffer
    device = make_device
    queue = device.queues["compute"][0]
    mm = device.memory_manager

    buf = mm.allocate(64,
      ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT,
      usage: ComputeRuntime::BufferUsage::TRANSFER_DST)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_update_buffer(buf, 0, "\xAA".b * 16)
    cb.end_recording

    queue.submit([cb])
    assert_equal 1, device.stats.total_transfers
  end
end

class TestBarriersQueue < Minitest::Test
  include QueueTestHelper

  def test_barrier_recorded_in_stats
    device = make_device
    queue = device.queues["compute"][0]

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_pipeline_barrier(ComputeRuntime::PipelineBarrier.new(
      src_stage: :compute,
      dst_stage: :transfer
    ))
    cb.end_recording

    queue.submit([cb])
    assert_equal 1, device.stats.total_barriers
  end

  def test_barrier_produces_trace
    device = make_device
    queue = device.queues["compute"][0]

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_pipeline_barrier(ComputeRuntime::PipelineBarrier.new(
      src_stage: :compute,
      dst_stage: :transfer
    ))
    cb.end_recording

    traces = queue.submit([cb])
    barrier_traces = traces.select { |t| t.event_type == :barrier }
    assert_equal 1, barrier_traces.length
  end
end

class TestQueueProperties < Minitest::Test
  include QueueTestHelper

  def test_queue_type
    device = make_device
    queue = device.queues["compute"][0]
    assert_equal :compute, queue.queue_type
  end

  def test_wait_idle
    device = make_device
    queue = device.queues["compute"][0]
    queue.wait_idle # Should not raise
  end
end

class TestTraces < Minitest::Test
  include QueueTestHelper

  def test_submit_produces_traces
    device = make_device
    queue = device.queues["compute"][0]
    pipeline = make_pipeline(device)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    traces = queue.submit([cb])
    assert traces.length > 0

    event_types = traces.map(&:event_type).to_set
    assert event_types.include?(:submit)
    assert event_types.include?(:begin_execution)
    assert event_types.include?(:end_execution)
  end

  def test_trace_format
    device = make_device
    queue = device.queues["compute"][0]
    pipeline = make_pipeline(device)

    cb = device.create_command_buffer
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end_recording

    traces = queue.submit([cb])
    traces.each do |trace|
      formatted = trace.format
      assert_kind_of String, formatted
      assert formatted.length > 0
    end
  end
end
