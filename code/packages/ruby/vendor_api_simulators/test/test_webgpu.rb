# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# WebGPU Runtime Simulator Tests
# ---------------------------------------------------------------------------
class TestWebGPU < Minitest::Test
  include CodingAdventures::VendorApiSimulators

  def setup
    @gpu = GPU.new
    @adapter = @gpu.request_adapter
    @device = @adapter.request_device
  end

  # =================================================================
  # GPU and adapter
  # =================================================================

  def test_gpu_creates_successfully
    assert_instance_of GPU, @gpu
  end

  def test_request_adapter
    adapter = @gpu.request_adapter
    assert_instance_of GPUAdapter, adapter
  end

  def test_request_adapter_low_power
    adapter = @gpu.request_adapter(options: GPURequestAdapterOptions.new(power_preference: "low-power"))
    assert_instance_of GPUAdapter, adapter
  end

  def test_request_adapter_high_performance
    adapter = @gpu.request_adapter(options: GPURequestAdapterOptions.new(power_preference: "high-performance"))
    assert_instance_of GPUAdapter, adapter
  end

  def test_adapter_name
    refute_empty @adapter.name
  end

  def test_adapter_features
    assert_includes @adapter.features, "compute"
  end

  def test_adapter_limits
    assert_instance_of GPUAdapterLimits, @adapter.limits
    assert @adapter.limits.max_buffer_size > 0
  end

  # =================================================================
  # Device
  # =================================================================

  def test_request_device
    device = @adapter.request_device
    assert_instance_of GPUDevice, device
  end

  def test_device_queue
    assert_instance_of GPUQueue, @device.queue
  end

  def test_device_features
    assert_includes @device.features, "compute"
  end

  def test_device_limits
    assert_instance_of GPUDeviceLimits, @device.limits
  end

  def test_device_destroy
    @device.destroy
  end

  def test_device_default_creation
    device = GPUDevice.new
    assert_instance_of GPUDevice, device
  end

  # =================================================================
  # Buffer creation
  # =================================================================

  def test_create_buffer
    desc = GPUBufferDescriptor.new(size: 256, usage: GPUBufferUsage::STORAGE)
    buf = @device.create_buffer(desc)
    assert_instance_of GPUBuffer, buf
    assert_equal 256, buf.size
    assert_equal GPUBufferUsage::STORAGE, buf.usage
  end

  def test_create_buffer_mapped_at_creation
    desc = GPUBufferDescriptor.new(size: 64, usage: GPUBufferUsage::MAP_WRITE, mapped_at_creation: true)
    buf = @device.create_buffer(desc)
    assert buf.mapped?
    data = buf.get_mapped_range
    assert_kind_of String, data
    buf.unmap
  end

  def test_buffer_map_async_and_get_mapped_range
    desc = GPUBufferDescriptor.new(size: 16, usage: GPUBufferUsage::MAP_READ | GPUBufferUsage::STORAGE)
    buf = @device.create_buffer(desc)
    buf.map_async(GPUMapMode::READ)
    data = buf.get_mapped_range
    assert_equal 16, data.bytesize
    buf.unmap
  end

  def test_buffer_get_mapped_range_with_offset
    desc = GPUBufferDescriptor.new(size: 32, usage: GPUBufferUsage::MAP_READ | GPUBufferUsage::STORAGE)
    buf = @device.create_buffer(desc)
    buf.map_async(GPUMapMode::READ)
    data = buf.get_mapped_range(offset: 8, range_size: 8)
    assert_equal 8, data.bytesize
    buf.unmap
  end

  def test_buffer_unmap_without_map_raises
    desc = GPUBufferDescriptor.new(size: 16, usage: GPUBufferUsage::STORAGE)
    buf = @device.create_buffer(desc)
    assert_raises(RuntimeError) { buf.unmap }
  end

  def test_buffer_get_mapped_range_without_map_raises
    desc = GPUBufferDescriptor.new(size: 16, usage: GPUBufferUsage::STORAGE)
    buf = @device.create_buffer(desc)
    assert_raises(RuntimeError) { buf.get_mapped_range }
  end

  def test_buffer_destroy
    desc = GPUBufferDescriptor.new(size: 64, usage: GPUBufferUsage::STORAGE)
    buf = @device.create_buffer(desc)
    buf.destroy
    assert buf.destroyed?
  end

  def test_buffer_map_destroyed_raises
    desc = GPUBufferDescriptor.new(size: 64, usage: GPUBufferUsage::STORAGE)
    buf = @device.create_buffer(desc)
    buf.destroy
    assert_raises(RuntimeError) { buf.map_async(GPUMapMode::READ) }
  end

  # =================================================================
  # Shader and pipeline
  # =================================================================

  def test_create_shader_module
    desc = GPUShaderModuleDescriptor.new(code: nil)
    shader = @device.create_shader_module(desc)
    assert_instance_of GPUShaderModule, shader
  end

  def test_create_shader_module_with_code
    desc = GPUShaderModuleDescriptor.new(code: [1, 2, 3])
    shader = @device.create_shader_module(desc)
    assert_instance_of GPUShaderModule, shader
  end

  def test_create_compute_pipeline
    shader_desc = GPUShaderModuleDescriptor.new
    shader = @device.create_shader_module(shader_desc)
    stage = GPUProgrammableStage.new(mod: shader)
    desc = GPUComputePipelineDescriptor.new(compute: stage)
    pipeline = @device.create_compute_pipeline(desc)
    assert_instance_of GPUComputePipeline, pipeline
  end

  def test_pipeline_get_bind_group_layout
    shader = @device.create_shader_module(GPUShaderModuleDescriptor.new)
    stage = GPUProgrammableStage.new(mod: shader)
    pipeline = @device.create_compute_pipeline(GPUComputePipelineDescriptor.new(compute: stage))
    layout = pipeline.get_bind_group_layout(0)
    assert_instance_of GPUBindGroupLayout, layout
  end

  def test_pipeline_get_bind_group_layout_out_of_range
    shader = @device.create_shader_module(GPUShaderModuleDescriptor.new)
    stage = GPUProgrammableStage.new(mod: shader)
    pipeline = @device.create_compute_pipeline(GPUComputePipelineDescriptor.new(compute: stage))
    assert_raises(IndexError) { pipeline.get_bind_group_layout(5) }
  end

  # =================================================================
  # Bind groups
  # =================================================================

  def test_create_bind_group_layout
    entry = GPUBindGroupLayoutEntry.new(binding: 0)
    desc = GPUBindGroupLayoutDescriptor.new(entries: [entry])
    layout = @device.create_bind_group_layout(desc)
    assert_instance_of GPUBindGroupLayout, layout
  end

  def test_create_pipeline_layout
    bg_layout = @device.create_bind_group_layout(GPUBindGroupLayoutDescriptor.new(entries: []))
    desc = GPUPipelineLayoutDescriptor.new(bind_group_layouts: [bg_layout])
    pl_layout = @device.create_pipeline_layout(desc)
    assert_instance_of GPUPipelineLayout, pl_layout
  end

  def test_create_bind_group
    entry = GPUBindGroupLayoutEntry.new(binding: 0)
    layout = @device.create_bind_group_layout(GPUBindGroupLayoutDescriptor.new(entries: [entry]))

    buf = @device.create_buffer(GPUBufferDescriptor.new(size: 64, usage: GPUBufferUsage::STORAGE))
    bg_desc = GPUBindGroupDescriptor.new(
      layout: layout,
      entries: [GPUBindGroupEntry.new(binding: 0, resource: buf)]
    )
    bg = @device.create_bind_group(bg_desc)
    assert_instance_of GPUBindGroup, bg
  end

  # =================================================================
  # Command encoding and submission
  # =================================================================

  def test_create_command_encoder
    encoder = @device.create_command_encoder
    assert_instance_of GPUCommandEncoder, encoder
  end

  def test_compute_pass_dispatch
    shader = @device.create_shader_module(GPUShaderModuleDescriptor.new)
    stage = GPUProgrammableStage.new(mod: shader)
    pipeline = @device.create_compute_pipeline(GPUComputePipelineDescriptor.new(compute: stage))

    encoder = @device.create_command_encoder
    pass = encoder.begin_compute_pass
    pass.set_pipeline(pipeline)
    pass.dispatch_workgroups(4)
    pass.end_pass
    cmd_buf = encoder.finish
    assert_instance_of GPUCommandBuffer, cmd_buf

    @device.queue.submit([cmd_buf])
  end

  def test_dispatch_without_pipeline_raises
    encoder = @device.create_command_encoder
    pass = encoder.begin_compute_pass
    assert_raises(RuntimeError) { pass.dispatch_workgroups(1) }
  end

  def test_compute_pass_with_bind_group
    entry = GPUBindGroupLayoutEntry.new(binding: 0)
    layout = @device.create_bind_group_layout(GPUBindGroupLayoutDescriptor.new(entries: [entry]))
    buf = @device.create_buffer(GPUBufferDescriptor.new(size: 64, usage: GPUBufferUsage::STORAGE))
    bg = @device.create_bind_group(GPUBindGroupDescriptor.new(
      layout: layout,
      entries: [GPUBindGroupEntry.new(binding: 0, resource: buf)]
    ))

    shader = @device.create_shader_module(GPUShaderModuleDescriptor.new)
    pipeline = @device.create_compute_pipeline(GPUComputePipelineDescriptor.new(
      compute: GPUProgrammableStage.new(mod: shader)
    ))

    encoder = @device.create_command_encoder
    pass = encoder.begin_compute_pass
    pass.set_pipeline(pipeline)
    pass.set_bind_group(0, bg)
    pass.dispatch_workgroups(2, 2, 1)
    pass.end_pass
    cmd_buf = encoder.finish
    @device.queue.submit([cmd_buf])
  end

  def test_copy_buffer_to_buffer
    src = @device.create_buffer(GPUBufferDescriptor.new(size: 32, usage: GPUBufferUsage::COPY_SRC))
    dst = @device.create_buffer(GPUBufferDescriptor.new(size: 32, usage: GPUBufferUsage::COPY_DST))

    encoder = @device.create_command_encoder
    encoder.copy_buffer_to_buffer(src, 0, dst, 0, 32)
    cmd_buf = encoder.finish
    @device.queue.submit([cmd_buf])
  end

  def test_queue_write_buffer
    buf = @device.create_buffer(GPUBufferDescriptor.new(size: 8, usage: GPUBufferUsage::STORAGE))
    @device.queue.write_buffer(buf, 0, "\x01\x02\x03\x04\x05\x06\x07\x08".b)
  end

  # =================================================================
  # Descriptor type defaults
  # =================================================================

  def test_buffer_descriptor_defaults
    desc = GPUBufferDescriptor.new
    assert_equal 0, desc.size
    assert_equal GPUBufferUsage::STORAGE, desc.usage
    assert_equal false, desc.mapped_at_creation
  end

  def test_compute_pass_descriptor
    desc = GPUComputePassDescriptor.new(label: "my_pass")
    assert_equal "my_pass", desc.label
  end

  def test_command_encoder_descriptor
    desc = GPUCommandEncoderDescriptor.new(label: "enc")
    assert_equal "enc", desc.label
  end
end
