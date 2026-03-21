# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_device_simulator"

# Tests for Intel GPU device simulator.
class TestIntelGPUConstruction < Minitest::Test
  include CodingAdventures

  def test_default_construction
    gpu = DeviceSimulator::IntelGPU.new(num_cores: 4)
    assert_includes gpu.name, "Intel"
    assert_equal 4, gpu.compute_units.length
  end

  def test_with_config
    config = DeviceSimulator::IntelGPUConfig.new(
      name: "Test Intel",
      num_compute_units: 4,
      l2_cache_size: 4096,
      l2_cache_associativity: 4,
      l2_cache_line_size: 64,
      global_memory_size: 1024 * 1024,
      num_xe_slices: 2,
      slice_config: DeviceSimulator::XeSliceConfig.new(xe_cores_per_slice: 2)
    )
    gpu = DeviceSimulator::IntelGPU.new(config: config)
    assert_equal "Test Intel", gpu.name
    assert_equal 2, gpu.xe_slices.length
  end

  def test_starts_idle
    gpu = DeviceSimulator::IntelGPU.new(num_cores: 2)
    assert gpu.idle?
  end

  def test_xe_slice_grouping
    config = DeviceSimulator::IntelGPUConfig.new(
      name: "Test Intel",
      num_compute_units: 8,
      l2_cache_size: 4096,
      l2_cache_associativity: 4,
      l2_cache_line_size: 64,
      global_memory_size: 1024 * 1024,
      num_xe_slices: 4,
      slice_config: DeviceSimulator::XeSliceConfig.new(xe_cores_per_slice: 2)
    )
    gpu = DeviceSimulator::IntelGPU.new(config: config)
    assert_equal 4, gpu.xe_slices.length
    gpu.xe_slices.each do |s|
      assert_equal 2, s.xe_cores.length
    end
  end
end

class TestIntelGPUKernelExecution < Minitest::Test
  include CodingAdventures

  def test_launch_and_run
    gpu = DeviceSimulator::IntelGPU.new(num_cores: 2)
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

  def test_multi_block
    gpu = DeviceSimulator::IntelGPU.new(num_cores: 4)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "multi",
      program: [GpuCore.limm(0, 1.0), GpuCore.halt],
      grid_dim: [4, 1, 1], block_dim: [32, 1, 1]
    )
    gpu.launch_kernel(kernel)
    traces = gpu.run(2000)
    assert gpu.idle?
  end
end

class TestIntelGPUMemory < Minitest::Test
  include CodingAdventures

  def test_malloc_and_transfer
    gpu = DeviceSimulator::IntelGPU.new(num_cores: 2)
    addr = gpu.malloc(256)
    cycles = gpu.memcpy_host_to_device(addr, "\x42".b * 256)
    assert cycles > 0
    data, _ = gpu.memcpy_device_to_host(addr, 256)
    assert_equal "\x42".b * 256, data
  end
end

class TestIntelGPUTraces < Minitest::Test
  include CodingAdventures

  def test_trace_format
    gpu = DeviceSimulator::IntelGPU.new(num_cores: 2)
    trace = gpu.step
    formatted = trace.format
    assert_includes formatted, "Intel"
  end

  def test_xe_slice_idle
    config = DeviceSimulator::IntelGPUConfig.new(
      name: "Test Intel",
      num_compute_units: 4,
      l2_cache_size: 4096,
      l2_cache_associativity: 4,
      l2_cache_line_size: 64,
      global_memory_size: 1024 * 1024,
      num_xe_slices: 2,
      slice_config: DeviceSimulator::XeSliceConfig.new(xe_cores_per_slice: 2)
    )
    gpu = DeviceSimulator::IntelGPU.new(config: config)
    gpu.xe_slices.each do |s|
      assert s.idle?
    end
  end
end

class TestIntelGPUReset < Minitest::Test
  include CodingAdventures

  def test_reset
    gpu = DeviceSimulator::IntelGPU.new(num_cores: 2)
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
    gpu = DeviceSimulator::IntelGPU.new(num_cores: 2)
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
