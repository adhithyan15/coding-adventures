# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_device_simulator"

# Tests for Apple ANE device simulator.
class TestAppleANEConstruction < Minitest::Test
  include CodingAdventures

  def test_default_construction
    ane = DeviceSimulator::AppleANE.new(num_cores: 4)
    assert_includes ane.name, "Apple"
    assert_equal 4, ane.compute_units.length
  end

  def test_with_config
    config = DeviceSimulator::ANEConfig.new(
      name: "Test ANE",
      num_compute_units: 8,
      global_memory_size: 1024 * 1024,
      unified_memory: true,
      host_latency: 0
    )
    ane = DeviceSimulator::AppleANE.new(config: config)
    assert_equal "Test ANE", ane.name
    assert_equal 8, ane.compute_units.length
  end

  def test_starts_idle
    ane = DeviceSimulator::AppleANE.new(num_cores: 4)
    assert ane.idle?
  end

  def test_unified_memory
    ane = DeviceSimulator::AppleANE.new(num_cores: 4)
    assert ane.unified_memory?
  end
end

class TestAppleANEUnifiedMemory < Minitest::Test
  include CodingAdventures

  def test_zero_copy_host_to_device
    ane = DeviceSimulator::AppleANE.new(num_cores: 4)
    addr = ane.malloc(256)
    cycles = ane.memcpy_host_to_device(addr, "\x42".b * 256)
    assert_equal 0, cycles
  end

  def test_zero_copy_device_to_host
    ane = DeviceSimulator::AppleANE.new(num_cores: 4)
    addr = ane.malloc(64)
    ane.memcpy_host_to_device(addr, "\xAA".b * 64)
    data, cycles = ane.memcpy_device_to_host(addr, 64)
    assert_equal "\xAA".b * 64, data
    assert_equal 0, cycles
  end

  def test_data_persists_after_zero_copy
    ane = DeviceSimulator::AppleANE.new(num_cores: 4)
    addr = ane.malloc(128)
    ane.memcpy_host_to_device(addr, "\xFF".b * 128)
    data, _ = ane.memcpy_device_to_host(addr, 128)
    assert_equal "\xFF".b * 128, data
  end
end

class TestAppleANEInferenceExecution < Minitest::Test
  include CodingAdventures

  def test_launch_inference
    ane = DeviceSimulator::AppleANE.new(num_cores: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "conv2d",
      operation: "conv2d",
      input_data: [[1.0, 2.0], [3.0, 4.0]],
      weight_data: [[0.5, 0.5], [0.5, 0.5]]
    )
    ane.launch_kernel(kernel)
    refute ane.idle?
  end

  def test_run_to_completion
    ane = DeviceSimulator::AppleANE.new(num_cores: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "inference",
      operation: "matmul",
      input_data: [[1.0, 2.0], [3.0, 4.0]],
      weight_data: [[5.0, 6.0], [7.0, 8.0]]
    )
    ane.launch_kernel(kernel)
    traces = ane.run(500)
    assert traces.length > 0
    assert ane.idle?
  end

  def test_schedule_replay
    ane = DeviceSimulator::AppleANE.new(num_cores: 4)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "inference",
      operation: "matmul",
      input_data: [[1.0]], weight_data: [[1.0]]
    )
    ane.launch_kernel(kernel)
    trace = ane.step
    refute_empty trace.distributor_actions
  end
end

class TestAppleANETraces < Minitest::Test
  include CodingAdventures

  def test_trace_format
    ane = DeviceSimulator::AppleANE.new(num_cores: 2)
    trace = ane.step
    formatted = trace.format
    assert_includes formatted, "Apple"
  end

  def test_trace_active_blocks
    ane = DeviceSimulator::AppleANE.new(num_cores: 4)
    trace = ane.step
    assert trace.active_blocks >= 0
  end
end

class TestAppleANEReset < Minitest::Test
  include CodingAdventures

  def test_reset
    ane = DeviceSimulator::AppleANE.new(num_cores: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      operation: "matmul",
      input_data: [[1.0]], weight_data: [[1.0]]
    )
    ane.launch_kernel(kernel)
    ane.run(500)
    ane.reset
    assert ane.idle?
  end

  def test_stats
    ane = DeviceSimulator::AppleANE.new(num_cores: 2)
    kernel = DeviceSimulator::KernelDescriptor.new(
      name: "test",
      operation: "matmul",
      input_data: [[1.0]], weight_data: [[1.0]]
    )
    ane.launch_kernel(kernel)
    ane.run(500)
    stats = ane.stats
    assert_equal 1, stats.total_kernels_launched
  end
end
