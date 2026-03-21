# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_device_simulator"

# Tests for AMD GPU device simulator.
class TestAmdGPUConstruction < Minitest::Test
  include CodingAdventures

  def test_default_construction
    gpu = DeviceSimulator::AmdGPU.new(num_cus: 4)
    assert_includes gpu.name, "AMD"
    assert_equal 4, gpu.compute_units.length
  end

  def test_with_amd_config
    config = DeviceSimulator::AmdGPUConfig.new(
      name: "Test AMD",
      num_compute_units: 4,
      l2_cache_size: 4096,
      l2_cache_associativity: 4,
      l2_cache_line_size: 64,
      global_memory_size: 1024 * 1024,
      num_shader_engines: 2,
      se_config: DeviceSimulator::ShaderEngineConfig.new(cus_per_engine: 2)
    )
    gpu = DeviceSimulator::AmdGPU.new(config: config)
    assert_equal "Test AMD", gpu.name
    assert_equal 2, gpu.shader_engines.length
    assert_equal 4, gpu.compute_units.length
  end

  def test_starts_idle
    gpu = DeviceSimulator::AmdGPU.new(num_cus: 2)
    assert gpu.idle?
  end

  def test_shader_engine_grouping
    config = DeviceSimulator::AmdGPUConfig.new(
      name: "Test AMD",
      num_compute_units: 6,
      l2_cache_size: 4096,
      l2_cache_associativity: 4,
      l2_cache_line_size: 64,
      global_memory_size: 1024 * 1024,
      num_shader_engines: 3,
      se_config: DeviceSimulator::ShaderEngineConfig.new(cus_per_engine: 2)
    )
    gpu = DeviceSimulator::AmdGPU.new(config: config)
    assert_equal 3, gpu.shader_engines.length
    gpu.shader_engines.each do |se|
      assert_equal 2, se.cus.length
    end
  end
end

class TestAmdGPUKernelExecution < Minitest::Test
  include CodingAdventures

  def test_launch_and_run
    gpu = DeviceSimulator::AmdGPU.new(num_cus: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      program: [GpuCore.limm(0, 42.0), GpuCore.halt],
      grid_dim: [2, 1, 1], block_dim: [32, 1, 1]
    )
    gpu.launch_kernel(kernel)
    traces = gpu.run(1000)
    assert traces.length > 0
    assert gpu.idle?
  end

  def test_multi_block_kernel
    gpu = DeviceSimulator::AmdGPU.new(num_cus: 4)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "multi_block",
      program: [GpuCore.limm(0, 1.0), GpuCore.halt],
      grid_dim: [4, 1, 1], block_dim: [32, 1, 1]
    )
    gpu.launch_kernel(kernel)
    traces = gpu.run(2000)
    assert gpu.idle?
  end
end

class TestAmdGPUMemory < Minitest::Test
  include CodingAdventures

  def test_malloc_and_transfer
    gpu = DeviceSimulator::AmdGPU.new(num_cus: 2)
    addr = gpu.malloc(256)
    cycles = gpu.memcpy_host_to_device(addr, "\x42".b * 256)
    assert cycles > 0
    data, _ = gpu.memcpy_device_to_host(addr, 256)
    assert_equal "\x42".b * 256, data
  end
end

class TestAmdGPUTraces < Minitest::Test
  include CodingAdventures

  def test_trace_format
    gpu = DeviceSimulator::AmdGPU.new(num_cus: 2)
    trace = gpu.step
    formatted = trace.format
    assert_includes formatted, "AMD"
  end

  def test_shader_engine_idle
    config = DeviceSimulator::AmdGPUConfig.new(
      name: "Test AMD",
      num_compute_units: 4,
      l2_cache_size: 4096,
      l2_cache_associativity: 4,
      l2_cache_line_size: 64,
      global_memory_size: 1024 * 1024,
      num_shader_engines: 2,
      se_config: DeviceSimulator::ShaderEngineConfig.new(cus_per_engine: 2)
    )
    gpu = DeviceSimulator::AmdGPU.new(config: config)
    gpu.shader_engines.each do |se|
      assert se.idle?
    end
  end
end

class TestAmdGPUReset < Minitest::Test
  include CodingAdventures

  def test_reset
    gpu = DeviceSimulator::AmdGPU.new(num_cus: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      program: [GpuCore.limm(0, 42.0), GpuCore.halt],
      grid_dim: [2, 1, 1], block_dim: [32, 1, 1]
    )
    gpu.launch_kernel(kernel)
    gpu.run(500)
    gpu.reset
    assert gpu.idle?
  end

  def test_stats
    gpu = DeviceSimulator::AmdGPU.new(num_cus: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      program: [GpuCore.limm(0, 42.0), GpuCore.halt],
      grid_dim: [2, 1, 1], block_dim: [32, 1, 1]
    )
    gpu.launch_kernel(kernel)
    gpu.run(500)
    stats = gpu.stats
    assert_equal 1, stats.total_kernels_launched
  end
end
