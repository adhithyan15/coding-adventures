# frozen_string_literal: true

require_relative "test_helper"

# Tests for Pipeline, ShaderModule, DescriptorSet.
module PipelineTestHelper
  include CodingAdventures

  def make_device
    instance = ComputeRuntime::RuntimeInstance.new
    physical = instance.enumerate_physical_devices[0]
    instance.create_logical_device(physical)
  end
end

class TestShaderModule < Minitest::Test
  include PipelineTestHelper

  def test_gpu_style
    shader = ComputeRuntime::ShaderModule.new(code: [GpuCore.limm(0, 1.0), GpuCore.halt])
    assert shader.gpu_style?
    refute shader.dataflow_style?
    refute_nil shader.code
    assert_equal 2, shader.code.length
  end

  def test_dataflow_style
    shader = ComputeRuntime::ShaderModule.new(operation: "matmul")
    assert shader.dataflow_style?
    refute shader.gpu_style?
    assert_equal "matmul", shader.operation
  end

  def test_local_size
    shader = ComputeRuntime::ShaderModule.new(code: [GpuCore.halt], local_size: [256, 1, 1])
    assert_equal [256, 1, 1], shader.local_size
  end

  def test_entry_point
    shader = ComputeRuntime::ShaderModule.new(code: [GpuCore.halt], entry_point: "compute_main")
    assert_equal "compute_main", shader.entry_point
  end

  def test_unique_ids
    s1 = ComputeRuntime::ShaderModule.new(code: [GpuCore.halt])
    s2 = ComputeRuntime::ShaderModule.new(code: [GpuCore.halt])
    refute_equal s1.module_id, s2.module_id
  end

  def test_default_entry_point
    shader = ComputeRuntime::ShaderModule.new(code: [GpuCore.halt])
    assert_equal "main", shader.entry_point
  end

  def test_default_local_size
    shader = ComputeRuntime::ShaderModule.new(code: [GpuCore.halt])
    assert_equal [32, 1, 1], shader.local_size
  end
end

class TestDescriptorSetLayout < Minitest::Test
  include PipelineTestHelper

  def test_basic_layout
    layout = ComputeRuntime::DescriptorSetLayout.new([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage"),
      ComputeRuntime::DescriptorBinding.new(binding: 1, type: "storage")
    ])
    assert_equal 2, layout.bindings.length
    assert_equal 0, layout.bindings[0].binding
    assert_equal 1, layout.bindings[1].binding
  end

  def test_empty_layout
    layout = ComputeRuntime::DescriptorSetLayout.new([])
    assert_equal 0, layout.bindings.length
  end

  def test_uniform_binding
    layout = ComputeRuntime::DescriptorSetLayout.new([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "uniform")
    ])
    assert_equal "uniform", layout.bindings[0].type
  end

  def test_unique_ids
    l1 = ComputeRuntime::DescriptorSetLayout.new([])
    l2 = ComputeRuntime::DescriptorSetLayout.new([])
    refute_equal l1.layout_id, l2.layout_id
  end
end

class TestPipelineLayout < Minitest::Test
  include PipelineTestHelper

  def test_basic
    ds_layout = ComputeRuntime::DescriptorSetLayout.new([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    layout = ComputeRuntime::PipelineLayout.new([ds_layout], push_constant_size: 16)
    assert_equal 1, layout.set_layouts.length
    assert_equal 16, layout.push_constant_size
  end

  def test_no_push_constants
    layout = ComputeRuntime::PipelineLayout.new([])
    assert_equal 0, layout.push_constant_size
  end

  def test_unique_ids
    l1 = ComputeRuntime::PipelineLayout.new([])
    l2 = ComputeRuntime::PipelineLayout.new([])
    refute_equal l1.layout_id, l2.layout_id
  end
end

class TestPipeline < Minitest::Test
  include PipelineTestHelper

  def test_creation
    shader = ComputeRuntime::ShaderModule.new(code: [GpuCore.limm(0, 1.0), GpuCore.halt])
    ds_layout = ComputeRuntime::DescriptorSetLayout.new([])
    pl_layout = ComputeRuntime::PipelineLayout.new([ds_layout])
    pipeline = ComputeRuntime::Pipeline.new(shader, pl_layout)
    assert_same shader, pipeline.shader
    assert_same pl_layout, pipeline.layout
  end

  def test_workgroup_size
    shader = ComputeRuntime::ShaderModule.new(code: [GpuCore.halt], local_size: [128, 2, 1])
    pl_layout = ComputeRuntime::PipelineLayout.new([])
    pipeline = ComputeRuntime::Pipeline.new(shader, pl_layout)
    assert_equal [128, 2, 1], pipeline.workgroup_size
  end

  def test_unique_ids
    shader = ComputeRuntime::ShaderModule.new(code: [GpuCore.halt])
    pl_layout = ComputeRuntime::PipelineLayout.new([])
    p1 = ComputeRuntime::Pipeline.new(shader, pl_layout)
    p2 = ComputeRuntime::Pipeline.new(shader, pl_layout)
    refute_equal p1.pipeline_id, p2.pipeline_id
  end
end

class TestDescriptorSet < Minitest::Test
  include PipelineTestHelper

  def test_write_and_read
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::STORAGE)

    layout = ComputeRuntime::DescriptorSetLayout.new([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    desc_set = ComputeRuntime::DescriptorSet.new(layout)
    desc_set.write(0, buf)

    assert_same buf, desc_set.get_buffer(0)
  end

  def test_multiple_bindings
    device = make_device
    mm = device.memory_manager
    buf_x = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::STORAGE)
    buf_y = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::STORAGE)

    layout = ComputeRuntime::DescriptorSetLayout.new([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage"),
      ComputeRuntime::DescriptorBinding.new(binding: 1, type: "storage")
    ])
    desc_set = ComputeRuntime::DescriptorSet.new(layout)
    desc_set.write(0, buf_x)
    desc_set.write(1, buf_y)

    assert_same buf_x, desc_set.get_buffer(0)
    assert_same buf_y, desc_set.get_buffer(1)
  end

  def test_invalid_binding
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::STORAGE)

    layout = ComputeRuntime::DescriptorSetLayout.new([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    desc_set = ComputeRuntime::DescriptorSet.new(layout)
    assert_raises(ArgumentError) { desc_set.write(99, buf) }
  end

  def test_freed_buffer
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::STORAGE)
    mm.free(buf)

    layout = ComputeRuntime::DescriptorSetLayout.new([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    desc_set = ComputeRuntime::DescriptorSet.new(layout)
    assert_raises(ArgumentError) { desc_set.write(0, buf) }
  end

  def test_unbound_returns_nil
    layout = ComputeRuntime::DescriptorSetLayout.new([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    desc_set = ComputeRuntime::DescriptorSet.new(layout)
    assert_nil desc_set.get_buffer(0)
  end

  def test_bindings_dict
    device = make_device
    mm = device.memory_manager
    buf = mm.allocate(64, ComputeRuntime::MemoryType::DEVICE_LOCAL,
      usage: ComputeRuntime::BufferUsage::STORAGE)

    layout = ComputeRuntime::DescriptorSetLayout.new([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    desc_set = ComputeRuntime::DescriptorSet.new(layout)
    desc_set.write(0, buf)
    bindings = desc_set.bindings
    assert bindings.key?(0)
    assert_same buf, bindings[0]
  end

  def test_unique_ids
    layout = ComputeRuntime::DescriptorSetLayout.new([])
    d1 = ComputeRuntime::DescriptorSet.new(layout)
    d2 = ComputeRuntime::DescriptorSet.new(layout)
    refute_equal d1.set_id, d2.set_id
  end
end

class TestDeviceFactory < Minitest::Test
  include PipelineTestHelper

  def test_create_shader_module
    device = make_device
    shader = device.create_shader_module(
      code: [GpuCore.limm(0, 1.0), GpuCore.halt],
      local_size: [64, 1, 1]
    )
    assert shader.gpu_style?
    assert_equal [64, 1, 1], shader.local_size
  end

  def test_create_dataflow_shader
    device = make_device
    shader = device.create_shader_module(operation: "matmul")
    assert shader.dataflow_style?
  end

  def test_create_full_pipeline
    device = make_device
    shader = device.create_shader_module(code: [GpuCore.limm(0, 1.0), GpuCore.halt])
    ds_layout = device.create_descriptor_set_layout([
      ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
    ])
    pl_layout = device.create_pipeline_layout([ds_layout], push_constant_size: 4)
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    assert_same shader, pipeline.shader
    assert_same pl_layout, pipeline.layout
    assert_equal 4, pipeline.layout.push_constant_size
  end
end
