# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_device_simulator"

# Tests for NVIDIA GPU device simulator.
class TestNvidiaGPUConstruction < Minitest::Test
  include CodingAdventures

  def test_default_construction
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
    assert_includes gpu.name, "NVIDIA"
    assert_equal 2, gpu.compute_units.length
  end

  def test_with_config
    config = DeviceSimulator::DeviceConfig.new(
      name: "Test GPU",
      num_compute_units: 3,
      l2_cache_size: 4096,
      l2_cache_associativity: 4,
      l2_cache_line_size: 64,
      global_memory_size: 1024 * 1024
    )
    gpu = DeviceSimulator::NvidiaGPU.new(config: config)
    assert_equal "Test GPU", gpu.name
    assert_equal 3, gpu.compute_units.length
  end

  def test_starts_idle
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
    assert gpu.idle?
  end
end

class TestNvidiaGPUMemoryManagement < Minitest::Test
  include CodingAdventures

  def test_malloc_and_free
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
    addr = gpu.malloc(256)
    assert addr >= 0
    gpu.free(addr)
  end

  def test_sequential_mallocs
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
    a1 = gpu.malloc(256)
    a2 = gpu.malloc(256)
    assert a2 > a1
  end

  def test_memcpy_host_to_device
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
    addr = gpu.malloc(128)
    cycles = gpu.memcpy_host_to_device(addr, "\x42".b * 128)
    assert cycles > 0
  end

  def test_memcpy_device_to_host
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
    addr = gpu.malloc(64)
    gpu.memcpy_host_to_device(addr, "\xAA".b * 64)
    data, cycles = gpu.memcpy_device_to_host(addr, 64)
    assert_equal "\xAA".b * 64, data
    assert cycles > 0
  end
end

class TestNvidiaGPUKernelLaunch < Minitest::Test
  include CodingAdventures

  def test_launch_simple_kernel
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      program: [GpuCore.limm(0, 42.0), GpuCore.halt],
      grid_dim: [2, 1, 1], block_dim: [32, 1, 1]
    )
    gpu.launch_kernel(kernel)
    refute gpu.idle?
  end

  def test_run_to_completion
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
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
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 4)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "multi_block",
      program: [GpuCore.limm(0, 1.0), GpuCore.halt],
      grid_dim: [8, 1, 1], block_dim: [32, 1, 1]
    )
    gpu.launch_kernel(kernel)
    traces = gpu.run(2000)
    assert gpu.idle?
    assert traces.length > 0
  end
end

class TestNvidiaGPUTraces < Minitest::Test
  include CodingAdventures

  def test_trace_has_cycle
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
    trace = gpu.step
    assert_equal 1, trace.cycle
  end

  def test_trace_has_device_name
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
    trace = gpu.step
    assert_includes trace.device_name, "NVIDIA"
  end

  def test_trace_format
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      program: [GpuCore.limm(0, 42.0), GpuCore.halt],
      grid_dim: [2, 1, 1], block_dim: [32, 1, 1]
    )
    gpu.launch_kernel(kernel)
    trace = gpu.step
    formatted = trace.format
    assert_includes formatted, "NVIDIA"
    assert_includes formatted, "Cycle"
  end

  def test_trace_shows_pending_blocks
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 1)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      program: [GpuCore.limm(0, 1.0), GpuCore.halt],
      grid_dim: [4, 1, 1], block_dim: [32, 1, 1]
    )
    gpu.launch_kernel(kernel)
    trace = gpu.step
    assert trace.pending_blocks >= 0
  end
end

class TestNvidiaGPUStats < Minitest::Test
  include CodingAdventures

  def test_stats_track_kernels
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      program: [GpuCore.limm(0, 42.0), GpuCore.halt],
      grid_dim: [2, 1, 1], block_dim: [32, 1, 1]
    )
    gpu.launch_kernel(kernel)
    gpu.run(500)
    stats = gpu.stats
    assert_equal 1, stats.total_kernels_launched
    assert stats.total_blocks_dispatched >= 1
  end

  def test_stats_track_memory
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
    addr = gpu.malloc(128)
    gpu.memcpy_host_to_device(addr, "\x00".b * 128)
    stats = gpu.stats
    assert_equal 128, stats.global_memory_stats.host_to_device_bytes
  end
end

class TestNvidiaGPUReset < Minitest::Test
  include CodingAdventures

  def test_reset_clears_state
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
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

  def test_reset_clears_memory
    gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 2)
    addr = gpu.malloc(64)
    gpu.memcpy_host_to_device(addr, "\xFF".b * 64)
    gpu.reset
    stats = gpu.stats
    assert_equal 0, stats.global_memory_stats.host_to_device_bytes
  end
end
