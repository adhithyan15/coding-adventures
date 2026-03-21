# frozen_string_literal: true

require_relative "test_helper"

# Tests for CommandBuffer -- recording and state transitions.
module CommandBufferTestHelper
  include CodingAdventures

  def make_device
    instance = ComputeRuntime::RuntimeInstance.new
    physical = instance.enumerate_physical_devices[0]
    instance.create_logical_device(physical)
  end
end

class TestCommandBufferLifecycle < Minitest::Test
  include CommandBufferTestHelper

  def test_initial_state
    cb = ComputeRuntime::CommandBuffer.new
    assert_equal :initial, cb.state
  end

  def test_begin
    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    assert_equal :recording, cb.state
  end

  def test_end
    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.end_recording
    assert_equal :recorded, cb.state
  end

  def test_reset
    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.end_recording
    cb.reset
    assert_equal :initial, cb.state
  end

  def test_begin_from_wrong_state
    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    assert_raises(RuntimeError) { cb.begin }
  end

  def test_end_from_wrong_state
    cb = ComputeRuntime::CommandBuffer.new
    assert_raises(RuntimeError) { cb.end_recording }
  end

  def test_record_without_begin
    device = make_device
    shader = device.create_shader_module(code: [GpuCore.limm(0, 1.0), GpuCore.halt])
    layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = ComputeRuntime::CommandBuffer.new
    assert_raises(RuntimeError) { cb.cmd_bind_pipeline(pipeline) }
  end

  def test_unique_ids
    cb1 = ComputeRuntime::CommandBuffer.new
    cb2 = ComputeRuntime::CommandBuffer.new
    refute_equal cb1.command_buffer_id, cb2.command_buffer_id
  end

  def test_reuse_after_reset
    device = make_device
    shader = device.create_shader_module(code: [GpuCore.limm(0, 1.0), GpuCore.halt])
    layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.end_recording
    assert_equal 1, cb.commands.length

    cb.reset
    assert_equal 0, cb.commands.length
    assert_equal :initial, cb.state

    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_bind_pipeline(pipeline)
    cb.end_recording
    assert_equal 2, cb.commands.length
  end
end

class TestComputeCommands < Minitest::Test
  include CommandBufferTestHelper

  def test_bind_pipeline
    device = make_device
    shader = device.create_shader_module(code: [GpuCore.limm(0, 1.0), GpuCore.halt])
    layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.end_recording

    assert_equal 1, cb.commands.length
    assert_equal "bind_pipeline", cb.commands[0].command
  end

  def test_bind_descriptor_set
    device = make_device
    layout = device.create_descriptor_set_layout([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    desc_set = device.create_descriptor_set(layout)

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_bind_descriptor_set(desc_set)
    cb.end_recording

    assert_equal 1, cb.commands.length
    assert_equal "bind_descriptor_set", cb.commands[0].command
  end

  def test_dispatch
    device = make_device
    shader = device.create_shader_module(code: [GpuCore.limm(0, 1.0), GpuCore.halt])
    layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(4, 1, 1)
    cb.end_recording

    assert_equal 2, cb.commands.length
    dispatch_cmd = cb.commands[1]
    assert_equal "dispatch", dispatch_cmd.command
    assert_equal 4, dispatch_cmd.args[:group_x]
  end

  def test_dispatch_without_pipeline
    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    assert_raises(RuntimeError) { cb.cmd_dispatch(1, 1, 1) }
  end

  def test_push_constants
    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_push_constants(0, "\x00\x00\x80\x3f".b) # 1.0f
    cb.end_recording

    assert_equal "push_constants", cb.commands[0].command
    assert_equal 4, cb.commands[0].args[:size]
  end

  def test_dispatch_indirect
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      12, ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::INDIRECT
    )
    shader = device.create_shader_module(code: [GpuCore.limm(0, 1.0), GpuCore.halt])
    layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch_indirect(buf)
    cb.end_recording

    assert_equal "dispatch_indirect", cb.commands[1].command
  end
end

class TestTransferCommands < Minitest::Test
  include CommandBufferTestHelper

  def test_copy_buffer
    device = make_device
    mm = device.memory_manager
    src = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::TRANSFER_SRC)
    dst = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::TRANSFER_DST)

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_copy_buffer(src, dst, 64)
    cb.end_recording

    assert_equal "copy_buffer", cb.commands[0].command
    assert_equal 64, cb.commands[0].args[:size]
  end

  def test_fill_buffer
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::TRANSFER_DST)

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_fill_buffer(buf, 0)
    cb.end_recording

    assert_equal "fill_buffer", cb.commands[0].command
    assert_equal 0, cb.commands[0].args[:value]
  end

  def test_update_buffer
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::TRANSFER_DST)

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_update_buffer(buf, 0, "\x42".b * 16)
    cb.end_recording

    assert_equal "update_buffer", cb.commands[0].command
  end
end

class TestSyncCommands < Minitest::Test
  include CommandBufferTestHelper

  def test_pipeline_barrier
    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_pipeline_barrier(ComputeRuntime::PipelineBarrier.new(
      src_stage: :compute,
      dst_stage: :transfer,
      memory_barriers: [
        ComputeRuntime::MemoryBarrier.new(
          src_access: ComputeRuntime::AccessFlags::SHADER_WRITE,
          dst_access: ComputeRuntime::AccessFlags::TRANSFER_READ
        )
      ]
    ))
    cb.end_recording

    assert_equal "pipeline_barrier", cb.commands[0].command
    assert_equal 1, cb.commands[0].args[:memory_barrier_count]
  end

  def test_set_event
    device = make_device
    event = device.create_event

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_set_event(event, :compute)
    cb.end_recording

    assert_equal "set_event", cb.commands[0].command
  end

  def test_wait_event
    device = make_device
    event = device.create_event

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_wait_event(event, :compute, :compute)
    cb.end_recording

    assert_equal "wait_event", cb.commands[0].command
  end

  def test_reset_event
    device = make_device
    event = device.create_event

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_reset_event(event, :compute)
    cb.end_recording

    assert_equal "reset_event", cb.commands[0].command
  end
end

class TestCommandList < Minitest::Test
  include CommandBufferTestHelper

  def test_multiple_commands
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(
      64,
      ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT,
      usage: ComputeRuntime::BufferUsage::STORAGE | ComputeRuntime::BufferUsage::TRANSFER_DST
    )
    shader = device.create_shader_module(code: [GpuCore.limm(0, 1.0), GpuCore.halt])
    layout = device.create_descriptor_set_layout([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    pl_layout = device.create_pipeline_layout([layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)
    desc_set = device.create_descriptor_set(layout)
    desc_set.write(0, buf)

    cb = ComputeRuntime::CommandBuffer.new
    cb.begin
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_bind_descriptor_set(desc_set)
    cb.cmd_dispatch(1, 1, 1)
    cb.cmd_pipeline_barrier(ComputeRuntime::PipelineBarrier.new(
      src_stage: :compute,
      dst_stage: :transfer
    ))
    cb.cmd_fill_buffer(buf, 0)
    cb.end_recording

    commands = cb.commands
    assert_equal 5, commands.length
    assert_equal "bind_pipeline", commands[0].command
    assert_equal "bind_descriptor_set", commands[1].command
    assert_equal "dispatch", commands[2].command
    assert_equal "pipeline_barrier", commands[3].command
    assert_equal "fill_buffer", commands[4].command
  end
end
